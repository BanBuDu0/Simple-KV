extern crate futures;
extern crate grpcio;
extern crate proto;

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
use futures::prelude::*;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use protobuf::{Message as PbMessage};
use raft::{
    Config,
    raw_node::RawNode,
    storage::MemStorage,
};
use raft_proto::eraftpb::*;

use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
use proto::kvraft::{DeleteReply, GetReply, PutReply, ScanReply};
use proto::kvraft_grpc::{self, KvRaft};

//server host
const HOST: &str = "127.0.0.1";
//server port
const PORT: u16 = 5030;

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
        get_reply.set_msg(
            String::from(args.get_key().clone())
        );

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


struct Node {
    // None if the raft is not initialized.
    raft_group: Option<RawNode<MemStorage>>,
    my_mailbox: Receiver<Message>,
    mailboxes: HashMap<u64, Sender<Message>>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    kv_pairs: HashMap<u16, String>,
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

fn main() {
    // gen_raft_node();
    maintain_server();
}
