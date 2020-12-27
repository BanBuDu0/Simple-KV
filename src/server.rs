extern crate futures;
extern crate grpcio;
extern crate proto;

use std::{io, thread};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::Mutex;

use futures::{FutureExt, TryFutureExt};
use futures::channel::oneshot;
use futures::executor::block_on;
use futures::prelude::*;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
// use raft::{
//     Config,
//     raw_node::RawNode,
//     storage::MemStorage,
// };

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

// fn gen_raft_node() {
//     let config = Config {
//         id: 1,
//         ..Default::default()
//     };
//     let storage = MemStorage::default();
//     config.validate().unwrap();
//     // let mut node = RawNode::new(&config, storage, vec![]).unwrap();
// }

fn main() {
    // gen_raft_node();
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
