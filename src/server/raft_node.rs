extern crate futures;
extern crate grpcio;
extern crate proto;
extern crate raft;
extern crate raft_proto;
#[macro_use]
extern crate slog;
extern crate slog_term;

use std::{str, thread, io};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use protobuf::Message as PbMessage;
use raft::{Config, raw_node::RawNode};
use raft::{prelude::*, StateRole};
use raft::storage::MemStorage;
use regex::Regex;
use slog::Drain;

use crate::proposal::Proposal;
use crate::server::maintain_server;
use futures_util::__private::Option::Some;
use std::fs::File;
use std::fs::OpenOptions;
use futures::io::Error;
use std::io::{Write, Read, BufRead};
extern crate encoding;
use encoding::all::ASCII;
use encoding::{Encoding, EncoderTrap};
use std::path::Path;

mod server;
mod proposal;

const NUM_NODES: u32 = 3;

pub struct Node {
    // None if the raft is not initialized.
    raft_group: Arc<Mutex<RawNode<MemStorage>>>,
    // message Receiver
    my_mailbox: Receiver<Message>,
    // message sender
    mailboxes: HashMap<u64, Sender<Message>>,
    // agree_chs: HashMap<i64, Receiver<Proposal>>,

    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    kv_pairs: Arc<Mutex<HashMap<i64, String>>>,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


impl Node {
    // Create a raft leader only with itself in its configuration.
    fn create_raft_leader(
        id: u64,
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
        logger: &slog::Logger,
    ) -> Self {
        let cfg = example_config(id);
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let mut s = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];
        let storage = MemStorage::new();
        // let stora = MemStorage::from()
        // let mut file =  File::open("entry.data").unwrap();
        if let Ok(lines) = read_lines(format!("entry{}.data", id)) {
            // 使用迭代器，返回一个（可选）字符串
            for line in lines {
                if let Ok(ip) = line {
                    println!("{}", ip);
                }
            }
        }
        // let data = file.;

        storage.wl().apply_snapshot(s).unwrap();

        let raft_group = RawNode::new(&cfg, storage, &logger).unwrap();
        let raft_group = Arc::new(Mutex::new(raft_group));
        Node {
            raft_group,
            my_mailbox,
            mailboxes,
            kv_pairs: Default::default(),
        }
    }

    // Create a raft follower.
    fn create_raft_follower(
        id: u64,
        my_mailbox: Receiver<Message>,
        mailboxes: HashMap<u64, Sender<Message>>,
        logger: &slog::Logger,
    ) -> Self {
        let cfg = example_config(id);
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let storage = MemStorage::new();
        Node {
            raft_group: Arc::new(Mutex::new(RawNode::new(&cfg, storage, &logger).unwrap())),
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
        println!("get is_initial_msg {{ {:?} }}", msg);
        let cfg = example_config(msg.to);
        let logger = logger.new(o!("tag" => format!("peer_{}", msg.to)));
        let storage = MemStorage::new();
        self.raft_group = Arc::new(Mutex::new(RawNode::new(&cfg, storage, &logger).unwrap()));
    }

    // Step a raft message, initialize the raft if need.
    fn step(&mut self, msg: Message, logger: &slog::Logger) {
        // if self.raft_group.lock().unwrap().is_none() {
        //     if is_initial_msg(&msg) {
        //         self.initialize_raft_from_message(&msg, &logger);
        //     } else {
        //         return;
        //     }
        // }
        let raft_group = Arc::clone(&self.raft_group);
        let _ = raft_group.lock().unwrap().step(msg);
        // let ref mut raft_group = raft_group.lock().unwrap();
        // let mut raft_group = raft_group.as_mut().unwrap();
        //
        // //Step advances the state machine using the given message.
        // let _ = raft_group.step(msg);
    }
}


// The message can be used to initialize a raft node or not.
fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestVote
        || msg_type == MessageType::MsgRequestPreVote
    // || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}

fn propose(raft_group: Arc<Mutex<RawNode<MemStorage>>>, proposal: &mut Proposal) {
    let last_index1 = raft_group.lock().unwrap().raft.raft_log.last_index() + 1;
    if let Some((ref key, ref value)) = proposal.normal {
        /// 0-none, 1-put, 2-get, 3-delete, 4-scan
        match proposal.op_type {
            1 =>{
                let data = format!("put {} {}", key, value).into_bytes();
                let _ = raft_group.lock().unwrap().propose(vec![], data);
            },
            2 => {
                let data = format!("get {} {}", key, "").into_bytes();
                let _ = raft_group.lock().unwrap().propose(vec![], data);
            },
            3 => {
                let data = format!("del {} {}", key, "").into_bytes();
                let _ = raft_group.lock().unwrap().propose(vec![], data);
            },
            4 => {
                let data = format!("scan {} {}", key, value).into_bytes();
                let _ = raft_group.lock().unwrap().propose(vec![], data);
            },
            _ => {},
        }
    } else if let Some(ref cc) = proposal.conf_change {
        let _ = raft_group.lock().unwrap().propose_conf_change(vec![], cc.clone());
    } else if let Some(_transferee) = proposal.transfer_leader {
        // TODO: implement transfer leader.
        unimplemented!();
    }

    let last_index2 = raft_group.lock().unwrap().raft.raft_log.last_index() + 1;
    if last_index2 == last_index1 {
        // Propose failed, don't forget to respond to the client.
        proposal.propose_success.send(false).unwrap();
    } else {
        proposal.proposed = last_index1;
    }
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
    raft_group: Arc<Mutex<RawNode<MemStorage>>>,
    kv_pairs: &mut Arc<Mutex<HashMap<i64, String>>>,
    mailboxes: &HashMap<u64, Sender<Message>>,
    proposals: &Mutex<VecDeque<Proposal>>,
    logger: &slog::Logger,
) {
    if !raft_group.lock().unwrap().has_ready() {
        return;
    }
    let store = raft_group.lock().unwrap().raft.raft_log.store.clone();
    // Get the `Ready` with `RawNode::ready` interface.
    let mut ready = raft_group.lock().unwrap().ready();
    // let entry = raft_group.lock().unwrap().raft.raft_log.all_entries();
    // println!("entry: {{ {:?} }}", entry);


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

    let handle_committed_entries =
        |rn: Arc<Mutex<RawNode<MemStorage>>>, committed_entries: Vec<Entry>| {
            // let data = committed_entries.into
            let id = raft_group.lock().unwrap().raft.id;
            let a = format!("entry{}.data",&id);
            let mut file = match OpenOptions::new().append(true).open(&a) {
                Ok(f) => {f}
                Err(_) => {
                    let f = File::create(&a).unwrap();
                    f
                }
            };
            for entry in committed_entries {
                if entry.data.is_empty() {
                    // From new elected leaders.
                    continue;
                }

                if let EntryType::EntryConfChange = entry.get_entry_type() {
                    // For conf change messages, make them effective.
                    let mut cc = ConfChange::default();
                    cc.merge_from_bytes(&entry.data).unwrap();
                    let cs = rn.lock().unwrap().apply_conf_change(&cc).unwrap();
                    store.wl().set_conf_state(cs);

                    // persist entry data
                    let data = entry.write_to_bytes().unwrap();
                    file.write_all(&data);
                    file.write(&"\n".as_bytes());
                    println!("id: {}, entry: {{ {:?} }}",&id, entry);
                } else {
                    // For normal proposals, extract the key-value pair and then
                    // insert them into the kv engine.
                    let data = str::from_utf8(&entry.data).unwrap();
                    let put_reg = Regex::new("put ([0-9]+) (.+)").unwrap();
                    let delete_reg = Regex::new("del ([0-9]+)").unwrap();
                    // let get_reg = Regex::new("get ([0-9]+)").unwrap();
                    let mut flag= false;

                    if let Some(caps) = put_reg.captures(&data) {
                        let t1 = caps[1].parse().unwrap();
                        let t2 = caps[2].to_string();
                        println!("insert key: {}, val: {}", t1, t2);
                        kv_pairs.lock().unwrap().insert(t1, t2);
                        flag = true;
                        // kv_pairs.insert(caps[1].to_string(), caps[2].to_string());
                    } else if let Some(caps) = delete_reg.captures(&data) {
                        let t1 = caps[1].parse().unwrap();
                        kv_pairs.lock().unwrap().remove(&t1);
                        println!("remove key: {}", t1);
                        flag=true;
                    }
                    if flag{
                        // persist entry data
                        let data = entry.write_to_bytes().unwrap();
                        file.write_all(&data);
                        file.write(&"\n".as_bytes());
                        println!("id: {}, entry: {{ {:?} }}",&id, entry);
                    }
                }
                if rn.lock().unwrap().raft.state == StateRole::Leader {
                    // The leader should response to the clients, tell them if their proposals
                    // succeeded or not.
                    let proposal = proposals.lock().unwrap().pop_front().unwrap();
                    proposal.propose_success.send(true).unwrap();
                }
            }
        };
    // Apply all committed entries.
    handle_committed_entries(Arc::clone(&raft_group), ready.take_committed_entries());

    // Persistent raft logs. It's necessary because in `RawNode::advance` we stabilize
    // raft logs to the latest position.
    let entries = ready.entries();
    if let Err(e) = store.wl().append(entries) {
        error!(
            logger,
            "persist raft log fail: {:?}, need to retry or panic", e
        );
        println!("term: {}",entries[0].term);

        return;
    }

    if let Some(hs) = ready.hs() {
        // Raft HardState changed, and we need to persist it.
        store.wl().set_hardstate(hs.clone());

    }

    // Call `RawNode::advance` interface to update position flags in the raft.
    let raft_group = Arc::clone(&raft_group);
    let mut light_rd = raft_group.lock().unwrap().advance(ready);
    // Send out the messages.
    handle_messages(light_rd.take_messages());
    // Apply all committed entries.
    handle_committed_entries(Arc::clone(&raft_group), light_rd.take_committed_entries());
    // Advance the apply index.
    raft_group.lock().unwrap().advance_apply();
}

pub enum Signal {
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


pub fn start_raft_state_machine(proposals: Arc<Mutex<VecDeque<Proposal>>>) {
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
    // let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));

    let mut nodes = HashMap::new();
    let mut stores = HashMap::new();

    //保存了每个raft node的处理线程
    let mut handles = Vec::new();

    // init NUM_NODES Raft node
    for (i, rx) in rx_vec.into_iter().enumerate() {
        // A map[peer_id -> sender]. In the example we create 5 nodes, with ids in [1, 5].
        let mailboxes = (1..4u64).zip(tx_vec.iter().cloned()).collect();
        let mut node = match i {
            // Peer 1 is the leader.
            0 => Node::create_raft_leader((i + 1) as u64, rx, mailboxes, &logger),
            // Other peers are followers.
            _ => Node::create_raft_follower((i + 1) as u64, rx, mailboxes, &logger),
        };

        let raft_group = Arc::clone(&node.raft_group);
        let kv_store = Arc::clone(&node.kv_pairs);
        nodes.insert(i, raft_group);
        stores.insert(i, kv_store);

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
                        // You can call the step function when you receive the Raft messages from other nodes.
                        // Calling Raft::step will change the memory state of Raft.
                        Ok(msg) => node.step(msg, &logger),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }

                let raft_group = Arc::clone(&node.raft_group);

                if t.elapsed() >= Duration::from_millis(100) {
                    // let raft_group =  Arc::clone(&raft_group);
                    let e_term = raft_group.lock().unwrap().raft.term;
                    let e_vote = raft_group.lock().unwrap().raft.term;

                    raft_group.lock().unwrap().tick();
                    t = Instant::now();
                    // let raft_group =  Arc::clone(&raft_group);

                    let a_term = raft_group.lock().unwrap().raft.term;
                    let a_vote = raft_group.lock().unwrap().raft.term;
                    if e_term != a_term{
                        //TODO persist
                        info!(
                            logger,
                            "old term {} is differ from new term {}",e_term, a_term
                        );
                    }
                    if e_vote != a_vote{
                        //TODO persist
                        info!(
                            logger,
                            "old vote {} is differ from new vote {}",e_vote, a_vote
                        );
                    }
                }

                // Let the leader pick pending proposals from the global queue.
                if raft_group.lock().unwrap().raft.state == StateRole::Leader {
                    // Handle new proposals.
                    let mut proposals = proposals.lock().unwrap();
                    for p in proposals.iter_mut().skip_while(|p| p.proposed > 0) {
                        info!(
                            logger,
                            "{} Get a proposal and I will propose it  {{ {:?} }}", raft_group.lock().unwrap().raft.id, p
                        );
                        propose(Arc::clone(&raft_group), p);
                    }
                }

                // Handle readies from the raft.
                on_ready(
                    raft_group,
                    &mut node.kv_pairs,
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
        "We get a 3 nodes Raft cluster now, now start KV Server"
    );


    // thread::sleep(Duration::from_secs(3));

    maintain_server(proposals, nodes, stores, tx_stop);

    // (0..10u16)
    //     .filter(|i| {
    //         let (proposal, rx) = Proposal::normal(String::from(i.to_string()), "hello, world".to_owned());
    //         proposals.lock().unwrap().push_back(proposal);
    //         println!("\nproposal {}\n", i);
    //         // After we got a response from `rx`, we can assume the put succeeded and following
    //         // `get` operations can find the key-value pair.
    //         rx.recv().unwrap()
    //     })
    //     .count();
}

fn main() {
    let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));
    // match File::open("entry.data"){
    //     Ok(_) => {}
    //     Err(_) => {
    //         File::create("entry.data").unwrap();
    //     }
    // }
    // match File::open("state.data"){
    //     Ok(_) => {}
    //     Err(_) => {
    //         File::create("state.data").unwrap();
    //     }
    // }

    start_raft_state_machine(proposals);
}