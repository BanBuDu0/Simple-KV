extern crate futures;
extern crate grpcio;
extern crate proto;
extern crate raft;
extern crate raft_proto;
// #[macro_use]
extern crate slog;
extern crate slog_term;

use std::{io, str, thread};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{FutureExt, TryFutureExt};
use futures::channel::oneshot;
use futures::executor::block_on;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};

use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
use proto::kvraft::{DeleteReply, GetReply, PutReply, ScanReply};
use proto::kvraft_grpc::{self, KvRaft};

use crate::proposal::Proposal;

//server host
const HOST: &str = "127.0.0.1";
//server port
const PORT: u16 = 5030;
//raft node num
const NUM_NODES: u32 = 3;
const ERR_LEADER: &str = "ERROR WRONG LEADER";
const ERR_KEY: &str = "ERROR NO KEY";


// #[derive(Clone)]
struct KvRaftService {
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
    db: Arc<Mutex<HashMap<String, String>>>,

    // /**
    // agree_chs, wait raft apply a proposal and notify the KvRaftService to reply client
    // **/
    // agree_chs: HashMap<i64, Receiver<Proposal>>,
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
}


impl Clone for KvRaftService {
    fn clone(&self) -> Self {
        let temp = KvRaftService {
            client_last_seq: self.client_last_seq.clone(),
            db: self.db.clone(),
            proposals: self.proposals.clone(),
        };
        temp
    }

    fn clone_from(&mut self, source: &Self) {
        self.db = source.db.clone();
        self.client_last_seq = source.client_last_seq.clone();
        self.proposals = source.proposals.clone();
    }
}


/**
init the KvRaftService
**/
impl KvRaftService {
    fn new() -> Self {
        Self {
            // mu: Mutex::new(KvRaftService),
            client_last_seq: Default::default(),
            db: Arc::new(Mutex::new(HashMap::new())),
            proposals: Arc::new(Mutex::new(Default::default())),
            // kv_pairs: Default::default(),
            // agree_chs: Default::default(),
        }
    }
}

/**
start grpc server for client
**/
pub fn maintain_server(proposals: Arc<Mutex<VecDeque<Proposal>>>) {
    (0..10u16)
        .filter(|i| {
            let (proposal, rx) = Proposal::normal(String::from(i.to_string()), "hello, world".to_owned());
            proposals.lock().unwrap().push_back(proposal);
            println!("\nproposal {}\n", i);
            // After we got a response from `rx`, we can assume the put succeeded and following
            // `get` operations can find the key-value pair.
            rx.recv().unwrap()
        })
        .count();

    let env = Arc::new(Environment::new(1));
    let mut kv_raft_server = KvRaftService::new();
    kv_raft_server.proposals = proposals;
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

/**
implement KvRaft Method in KvRaftService
**/
impl KvRaft for KvRaftService {
    fn get(&mut self, ctx: RpcContext, args: GetArgs, sink: UnarySink<GetReply>) {
        println!("Received Get request {{ {:?} }}", args);
        let mut get_reply = GetReply::new();

        let (proposal, rx) = Proposal::normal(args.key.clone(), "".to_string());
        self.proposals.lock().unwrap().push_back(proposal);

        if rx.recv().unwrap() {
            if let Some(val) = self.db.lock().unwrap().get(args.get_key().clone()) {
                get_reply.set_success(true);
                get_reply.set_val(val.to_string().clone());
            } else {
                get_reply.set_success(false);
                get_reply.set_val(String::from(ERR_KEY));
            }
        } else {
            get_reply.set_success(false);
            get_reply.set_msg(String::from(ERR_LEADER));
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
        let (proposal, rx) = Proposal::normal(args.key.clone(), args.val.clone());
        self.proposals.lock().unwrap().push_back(proposal);

        if rx.recv().unwrap() {
            put_reply.set_success(true);
            let db = &mut self.db;
            db.lock().unwrap().insert(args.key.clone(), args.val.clone());
            // self.db.insert(args.key.clone(), args.val.clone());
        } else {
            put_reply.set_success(false);
            put_reply.set_msg(String::from(ERR_LEADER));
        }

        if let Some(val) = self.db.lock().unwrap().get(&args.key) {
            println!("key: {}, val: {}", args.key, val);
        }


        let f = sink.success(put_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with PutReply {{ {:?} }}", put_reply));
        ctx.spawn(f)
    }

    fn delete(&mut self, ctx: RpcContext, args: DeleteArgs, sink: UnarySink<DeleteReply>) {
        println!("Received Put request {{ {:?} }}", args);
        let mut delete_reply = DeleteReply::new();

        let (proposal, rx) = Proposal::normal(args.key.clone(), "".to_string());
        self.proposals.lock().unwrap().push_back(proposal);

        if rx.recv().unwrap() {
            delete_reply.set_success(true);
            if let Some(val) = self.db.lock().unwrap().get(args.get_key().clone()) {
                self.db.lock().unwrap().remove(&val.to_string());
            }
        } else {
            delete_reply.set_success(false);
            delete_reply.set_msg(String::from(ERR_LEADER));
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
