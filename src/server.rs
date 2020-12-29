extern crate futures;
extern crate grpcio;
extern crate proto;
extern crate raft;
extern crate raft_proto;
#[macro_use]
extern crate slog;
extern crate slog_term;


use std::{io, str, thread};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::time::{Duration, Instant};

use futures::{FutureExt, TryFutureExt};
use futures::channel::oneshot;
use futures::executor::block_on;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use protobuf::Message as PbMessage;
use raft::{Config, raw_node::RawNode};
use raft::{prelude::*, StateRole};
use raft::storage::MemStorage;
use raft_proto::eraftpb::*;
use regex::Regex;
use slog::Drain;

// use proto::eraftpb::*;
use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
use proto::kvraft::{DeleteReply, GetReply, PutReply, ScanReply};
use proto::kvraft_grpc::{self, KvRaft};

//server host
const HOST: &str = "127.0.0.1";
//server port
const PORT: u16 = 5030;
//raft node num
const NUM_NODES: u32 = 3;
const ERR_LEADER: String = String::from("ERROR WRONG LEADER");
const ERR_KEY: String = String::from("ERROR NO KEY");


#[derive(Clone)]
struct KvRaftService {
    /**
    use to lock the concurrent read write
    **/
    // mu: Mutex<KvRaftService>,

    /**
    store the last sequence num of each client
    use to ignore any operation that it has already sean
    key -> client id
    val -> last sequence id
    **/
    client_last_seq: HashMap<i64, i32>,

    /**
    use to store the String KV pairs
    key -> PutArgs.key
    val -> PutArgs.val
    **/
    db: HashMap<String, String>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    kv_pairs: HashMap<String, String>,

    /**
    agree_chs, wait raft apply a proposal and notify the KvRaftService to reply client
    **/
    // agree_chs: HashMap<i64, Receiver<Proposal>>,
    // node: RawNode<MemStorage>,
}

/**
init the KvRaftService
**/
impl KvRaftService {
    fn new() -> Self {
        Self {
            // mu: Mutex::new(KvRaftService),
            client_last_seq: Default::default(),
            db: HashMap::new(),
            kv_pairs: Default::default(),
        }
    }
}


struct Node {
    // None if the raft is not initialized.
    raft_group: Option<RawNode<MemStorage>>,
    // message Receiver
    my_mailbox: Receiver<Message>,
    // message sender
    mailboxes: HashMap<u64, Sender<Message>>,

}
//TODO 拆分raft Node 和 server

/**
Some initialize method
**/
impl Node {
    // Create a raft leader only with itself in its configuration.
    fn create_raft_leader(
        id: u64,
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
        logger: &slog::Logger,
    ) -> Self {
        let mut cfg = example_config(id);
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let mut s = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];
        let storage = MemStorage::new();
        storage.wl().apply_snapshot(s).unwrap();
        let raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
        Node {
            raft_group,
            my_mailbox,
            mailboxes,
            // kv_pairs: Default::default(),
        }
    }

    // Create a raft follower.
    fn create_raft_follower(
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
    ) -> Self {
        Node {
            raft_group: None,
            my_mailbox,
            mailboxes,
            // kv_pairs: Default::default(),
        }
    }

    // Initialize raft for followers.
    fn initialize_raft_from_message(&mut self, msg: &Message, logger: &slog::Logger) {
        if !is_initial_msg(msg) {
            return;
        }
        let mut cfg = example_config(msg.to);
        let logger = logger.new(o!("tag" => format!("peer_{}", msg.to)));
        let storage = MemStorage::new();
        self.raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
    }

    // Step a raft message, initialize the raft if need.
    fn step(&mut self, msg: Message, logger: &slog::Logger) {
        if self.raft_group.is_none() {
            if is_initial_msg(&msg) {
                self.initialize_raft_from_message(&msg, &logger);
            } else {
                return;
            }
        }
        let raft_group = self.raft_group.as_mut().unwrap();
        //Step advances the state machine using the given message.
        let _ = raft_group.step(msg);
    }
}

// The message can be used to initialize a raft node or not.
fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestVote
        || msg_type == MessageType::MsgRequestPreVote
        || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}

struct Proposal {
    normal: Option<(u16, String)>,
    // key is an u16 integer, and value is a string.
    conf_change: Option<ConfChange>,
    // conf change.
    transfer_leader: Option<u64>,
    // If it's proposed, it will be set to the index of the entry.
    proposed: u64,
    propose_success: SyncSender<bool>,
}

impl Proposal {
    /**
    Use for configure change, return a Proposal that contain changed config
    input: changed configure
    output: Configure change Proposal and Receiver
    **/
    fn conf_change(cc: &ConfChange) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: None,
            conf_change: Some(cc.clone()),
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }

    /**
    Use for normal Proposal, insert Key/Value pairs
    input: insert Key and Value
    output: Proposal and Receiver
    When the Proposal is applied, the Receiver will receive the Proposal
    **/
    fn normal(key: u16, value: String) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: Some((key, value)),
            conf_change: None,
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }
}

fn propose(raft_group: &mut RawNode<MemStorage>, proposal: &mut Proposal) {
    let last_index1 = raft_group.raft.raft_log.last_index() + 1;
    if let Some((ref key, ref value)) = proposal.normal {
        let data = format!("put {} {}", key, value).into_bytes();
        let _ = raft_group.propose(vec![], data);
    } else if let Some(ref cc) = proposal.conf_change {
        let _ = raft_group.propose_conf_change(vec![], cc.clone());
    } else if let Some(_transferee) = proposal.transfer_leader {
        // TODO: implement transfer leader.
        unimplemented!();
    }

    let last_index2 = raft_group.raft.raft_log.last_index() + 1;
    if last_index2 == last_index1 {
        // Propose failed, don't forget to respond to the client.
        proposal.propose_success.send(false).unwrap();
    } else {
        proposal.proposed = last_index1;
    }
}

fn maintain_raft_state_machine(kv_pairs: &mut HashMap<String, String>,) {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(4096)
        .overflow_strategy(slog_async::OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, o!());

    // Create NUM_NODES mailboxes to send/receive messages. Every node holds a `Receiver` to receive
    // messages from others, and uses the respective `Sender` to send messages to others.
    let (mut tx_vec, mut rx_vec) = (Vec::new(), Vec::new());
    for _ in 0..NUM_NODES {
        let (tx, rx) = mpsc::channel();
        tx_vec.push(tx);
        rx_vec.push(rx);
    }

    let (tx_stop, rx_stop) = mpsc::channel();
    let rx_stop = Arc::new(Mutex::new(rx_stop));

    // A global pending proposals queue. New proposals will be pushed back into the queue, and
    // after it's committed by the raft cluster, it will be poped from the queue.
    let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));

    //保存了每个raft node的处理线程
    let mut handles = Vec::new();

    // init NUM_NODES Raft node
    for (i, rx) in rx_vec.into_iter().enumerate() {
        // A map[peer_id -> sender]. In the example we create 5 nodes, with ids in [1, 5].
        let mailboxes = (1..4u64).zip(tx_vec.iter().cloned()).collect();
        let mut node = match i {
            // Peer 1 is the leader.
            0 => Node::create_raft_leader(1, rx, mailboxes, &logger),
            // Other peers are followers.
            _ => Node::create_raft_follower(rx, mailboxes),
        };

        let proposals = Arc::clone(&proposals);
        // Tick the raft node per 100ms. So use an `Instant` to trace it.
        let mut t = Instant::now();

        // Clone the stop receiver
        let rx_stop_clone = Arc::clone(&rx_stop);
        let logger = logger.clone();

        // Here we spawn the node on a new thread and keep a handle so we can join on them later.
        let handle = thread::spawn(move ||
            loop {
                thread::sleep(Duration::from_millis(10));
                loop {
                    // Step raft messages.
                    match node.my_mailbox.try_recv() {
                        Ok(msg) => node.step(msg, &logger),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }

                let raft_group = match node.raft_group {
                    Some(ref mut r) => r,
                    // When Node::raft_group is `None` it means the node is not initialized.
                    _ => continue,
                };

                if t.elapsed() >= Duration::from_millis(100) {
                    raft_group.tick();
                    t = Instant::now();
                }

                // Let the leader pick pending proposals from the global queue.
                if raft_group.raft.state == StateRole::Leader {
                    // Handle new proposals.
                    let mut proposals = proposals.lock().unwrap();
                    for p in proposals.iter_mut().skip_while(|p| p.proposed > 0) {
                        propose(raft_group, p);
                    }
                }

                // Handle readies from the raft.
                on_ready(
                    raft_group,
                    kv_pairs,
                    // &mut node.kv_pairs,
                    &node.mailboxes,
                    &proposals,
                    &logger,
                );

                // Check control signals from
                if check_signals(&rx_stop_clone) {
                    return;
                };
            });
        handles.push(handle);
    }


    // Propose some conf changes so that followers can be initialized.
    add_all_followers(proposals.as_ref());
    // Put 100 key-value pairs.
    info!(
        logger,
        "We get a 3 nodes Raft cluster now, now propose 100 proposals"
    );
}

/**
start grpc server for client
**/
fn maintain_server() -> KvRaftService {
    let env = Arc::new(Environment::new(1));
    let kv_raft_server = KvRaftService::new();
    let service = kvraft_grpc::create_kv_raft(&kv_raft_server);
    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind(HOST, PORT)
        .build()
        .unwrap();
    server.start();
    for (ref host, port) in server.bind_addrs() {
        println!("listening on {}:{}", host, port);
    }
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        println!("Press ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = block_on(rx);
    let _ = block_on(server.shutdown());
    kv_raft_server
}

/**
implement KvRaft Method in KvRaftService
**/
impl KvRaft for KvRaftService {
    fn get(&mut self, ctx: RpcContext, args: GetArgs, sink: UnarySink<GetReply>) {
        println!("Received Get request {{ {:?} }}", args);
        let mut get_reply = GetReply::new();

        let is_leader = wait_opertaion();

        if !is_leader {
            get_reply.set_success(false);
            get_reply.set_msg(String::from(ERR_LEADER));
        } else {
            if let Some(val) = self.kv_pairs.get(args.get_key().clone()) {
                get_reply.set_success(true);
                get_reply.set_val(val.to_string().clone());
            } else {
                get_reply.set_success(false);
                get_reply.set_val(String::from(ERR_KEY));
            }
        }

        let f = sink
            .success(get_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", get_reply));
        ctx.spawn(f)
    }

    fn put(&mut self, ctx: RpcContext, args: PutArgs, sink: UnarySink<PutReply>) {
        println!("Received Put request {{ {:?} }}", args);
        let mut put_reply = PutReply::new();

        let is_leader = wait_opertaion();
        if is_leader {
            put_reply.set_success(false);
            put_reply.set_msg(ERR_LEADER);
        } else {
            put_reply.set_success(true);
        }

        let f = sink.success(put_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", put_reply));
        ctx.spawn(f)
    }

    fn delete(&mut self, ctx: RpcContext, args: DeleteArgs, sink: UnarySink<DeleteReply>) {
        println!("Received Put request {{ {:?} }}", args);
        let mut delete_reply = DeleteReply::new();

        let is_leader = wait_opertaion();
        if is_leader {
            delete_reply.set_success(false);
            delete_reply.set_msg(ERR_LEADER);
        } else {
            delete_reply.set_success(true);
        }

        let f = sink.success(delete_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", delete_reply));

        ctx.spawn(f)
    }

    fn scan(&mut self, ctx: RpcContext, req: ScanArgs, sink: UnarySink<ScanReply>) {
        unimplemented!()
    }
}

fn wait_opertaion() -> bool {
    //TODO Wtire Opertaion to proposals queue
    thread::sleep(Duration::from_millis(1000));
    unimplemented!()
}

fn example_config(i: u64) -> Config {
    Config {
        id: i,
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    }
}

fn on_ready(
    raft_group: &mut RawNode<MemStorage>,
    kv_pairs: &mut HashMap<String, String>,
    mailboxes: &HashMap<u64, Sender<Message>>,
    proposals: &Mutex<VecDeque<Proposal>>,
    logger: &slog::Logger,
) {
    if !raft_group.has_ready() {
        return;
    }
    let store = raft_group.raft.raft_log.store.clone();
    // Get the `Ready` with `RawNode::ready` interface.
    let mut ready = raft_group.ready();

    let handle_messages = |msgs: Vec<Vec<Message>>| {
        for vec_msg in msgs {
            for msg in vec_msg {
                let to = msg.to;
                if mailboxes[&to].send(msg).is_err() {
                    error!(
                        logger,
                        "send raft message to {} fail, let Raft retry it", to
                    );
                }
            }
        }
    };

    // Send out the messages come from the node.
    handle_messages(ready.take_messages());

    // Apply the snapshot. It's necessary because in `RawNode::advance` we stabilize the snapshot.
    if *ready.snapshot() != Snapshot::default() {
        let s = ready.snapshot().clone();
        if let Err(e) = store.wl().apply_snapshot(s) {
            error!(
                logger,
                "apply snapshot fail: {:?}, need to retry or panic", e
            );
            return;
        }
    }

    let mut handle_committed_entries =
        |rn: &mut RawNode<MemStorage>, committed_entries: Vec<Entry>| {
            for entry in committed_entries {
                if entry.data.is_empty() {
                    // From new elected leaders.
                    continue;
                }
                if let EntryType::EntryConfChange = entry.get_entry_type() {
                    // For conf change messages, make them effective.
                    let mut cc = ConfChange::default();
                    cc.merge_from_bytes(&entry.data).unwrap();
                    let cs = rn.apply_conf_change(&cc).unwrap();
                    store.wl().set_conf_state(cs);
                } else {
                    // For normal proposals, extract the key-value pair and then
                    // insert them into the kv engine.
                    let data = str::from_utf8(&entry.data).unwrap();
                    // let reg = Regex::new("put ([0-9]+) (.+)").unwrap();
                    let reg = Regex::new("put (.+) (.+)").unwrap();
                    if let Some(caps) = reg.captures(&data) {
                        // kv_pairs.insert(caps[1].parse().unwrap(), caps[2].to_string());
                        kv_pairs.insert(caps[1].to_string(), caps[2].to_string());
                    }
                }
                if rn.raft.state == StateRole::Leader {
                    // The leader should response to the clients, tell them if their proposals
                    // succeeded or not.
                    let proposal = proposals.lock().unwrap().pop_front().unwrap();
                    proposal.propose_success.send(true).unwrap();
                }
            }
        };
    // Apply all committed entries.
    handle_committed_entries(raft_group, ready.take_committed_entries());

    // Persistent raft logs. It's necessary because in `RawNode::advance` we stabilize
    // raft logs to the latest position.
    if let Err(e) = store.wl().append(ready.entries()) {
        error!(
            logger,
            "persist raft log fail: {:?}, need to retry or panic", e
        );
        return;
    }

    if let Some(hs) = ready.hs() {
        // Raft HardState changed, and we need to persist it.
        store.wl().set_hardstate(hs.clone());
    }

    // Call `RawNode::advance` interface to update position flags in the raft.
    let mut light_rd = raft_group.advance(ready);
    // Send out the messages.
    handle_messages(light_rd.take_messages());
    // Apply all committed entries.
    handle_committed_entries(raft_group, light_rd.take_committed_entries());
    // Advance the apply index.
    raft_group.advance_apply();
}

enum Signal {
    Terminate,
}


fn check_signals(receiver: &Arc<Mutex<mpsc::Receiver<Signal>>>) -> bool {
    match receiver.lock().unwrap().try_recv() {
        Ok(Signal::Terminate) => true,
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => true,
    }
}

// Proposes some conf change for peers [2, 5].
fn add_all_followers(proposals: &Mutex<VecDeque<Proposal>>) {
    for i in 2..4u64 {
        let mut conf_change = ConfChange::default();
        conf_change.node_id = i;
        conf_change.set_change_type(ConfChangeType::AddNode);
        loop {
            let (proposal, rx) = Proposal::conf_change(&conf_change);
            proposals.lock().unwrap().push_back(proposal);
            if rx.recv().unwrap() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}


fn main() {
    // let kv_raft = maintain_server();
    maintain_raft_state_machine();
}
