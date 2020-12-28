extern crate futures;
extern crate grpcio;
extern crate proto;
extern crate raft;
extern crate raft_proto;
#[macro_use]
extern crate slog;

use std::{io, thread};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

use futures::{FutureExt, TryFutureExt};
use futures::channel::mpsc::Sender;
use futures::channel::oneshot;
use futures::executor::block_on;
// use futures::prelude::*;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use protobuf::Message as PbMessage;
use raft::{Config, raw_node::RawNode, storage::MemStorage};
use raft_proto::eraftpb::*;

// use proto::eraftpb::*;
use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
use proto::kvraft::{DeleteReply, GetReply, PutReply, ScanReply};
use proto::kvraft_grpc::{self, KvRaft};

//server host
const HOST: &str = "127.0.0.1";
//server port
const PORT: u16 = 5030;
//raft node num
const NUM_NODES: u32 = 5;

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
}

/**
init the KvRaftService
**/
impl KvRaftService {
    fn new() -> Self {
        Self {
            // mu: Mutex::new(KvRaftService),
            client_last_seq: HashMap::new(),
            db: HashMap::new(),
        }
    }
}

/**
implement KvRaft Method in KvRaftService
**/
impl KvRaft for KvRaftService {
    fn get(&mut self, ctx: RpcContext, args: GetArgs, sink: UnarySink<GetReply>) {
        println!("Received Get request {{ {:?} }}", args);
        let mut get_reply = GetReply::new();
        get_reply.set_msg(String::from(args.get_key().clone()));

        let f = sink
            .success(get_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", get_reply));

        ctx.spawn(f)
    }

    fn put(&mut self, ctx: RpcContext, req: PutArgs, sink: UnarySink<PutReply>) {
        unimplemented!()
    }

    fn delete(&mut self, ctx: RpcContext, req: DeleteArgs, sink: UnarySink<DeleteReply>) {
        unimplemented!()
    }

    fn scan(&mut self, ctx: RpcContext, req: ScanArgs, sink: UnarySink<ScanReply>) {
        unimplemented!()
    }
}

fn maintain_server() {
    let env = Arc::new(Environment::new(1));
    let kv_raft_server = KvRaftService::new();
    let service = kvraft_grpc::create_kv_raft(kv_raft_server);
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
}

fn example_config(i: u64) -> Config {
    Config {
        id: i,
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    }
}

struct Node {
    // None if the raft is not initialized.
    raft_group: Option<RawNode<MemStorage>>,
    // message Receiver
    my_mailbox: Receiver<Message>,
    // message sender
    mailboxes: HashMap<u64, Sender<Message>>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    kv_pairs: HashMap<u16, String>,
}

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
            kv_pairs: Default::default(),
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
            kv_pairs: Default::default(),
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


// fn gen_raft_node() {
//     let config = Config {
//         id: 1,
//         ..Default::default()
//     };
//     let storage = MemStorage::default();
//     let storage = HashMap::new();
//     config.validate().unwrap();
//     // We'll use the built-in `MemStorage`, but you will likely want your own.
//     // Finally, create our Raft node!
//     let mut node = RawNode::new(&config, storage).unwrap();
//     // We will coax it into being the lead of a single node cluster for exploration.
//     node.raft.become_leader();
//
//     const NUM_NODES: u32 = 3;
//     // Create 5 mailboxes to send/receive messages. Every node holds a `Receiver` to receive
//     // messages from others, and uses the respective `Sender` to send messages to others.
//     let (mut tx_vec, mut rx_vec) = (Vec::new(), Vec::new());
//     for _ in 0..NUM_NODES {
//         let (tx, rx) = mpsc::channel();
//         tx_vec.push(tx);
//         rx_vec.push(rx);
//     }
//
//     let (tx_stop, rx_stop) = mpsc::channel();
//     let rx_stop = Arc::new(Mutex::new(rx_stop));
//
//     // let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));
//     let mut handles = Vec::new();
//     for (i, rx) in rx_vec.into_iter().enumerate() {}
//
// }

fn maintain_raft_state_machine() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(4096)
        .overflow_strategy(slog_async::OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, o!());
    // Create 5 mailboxes to send/receive messages. Every node holds a `Receiver` to receive
    // messages from others, and uses the respective `Sender` to send messages to others.
    let (mut tx_vec, mut rx_vec) = (Vec::new(), Vec::new());
    for _ in 0..NUM_NODES {
        let (tx, rx) = mpsc::channel();
        tx_vec.push(tx);
        rx_vec.push(rx);
    }
}

fn main() {
    // let node = Node
    // gen_raft_node();
    maintain_raft_state_machine();
    maintain_server();
}
