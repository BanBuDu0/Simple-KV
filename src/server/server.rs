extern crate futures;
extern crate grpcio;
extern crate proto;
extern crate raft;
extern crate raft_proto;
extern crate slog;
extern crate slog_term;

use std::{io, str, thread};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::{Arc, Mutex};

use futures::{FutureExt, TryFutureExt};
use futures::channel::oneshot;
use futures::executor::block_on;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use raft::RawNode;
use raft::storage::MemStorage;
use raft::StateRole;

use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
use proto::kvraft::{DeleteReply, GetReply, PutReply, ScanReply};
use proto::kvraft_grpc::{self, KvRaft};

use crate::proposal::Proposal;
use std::sync::mpsc::Sender;
use crate::Signal;
use std::time::{Instant, Duration};

//server host
const HOST: &str = "127.0.0.1";
const ERR_LEADER: &str = "ERROR WRONG LEADER";
const ERR_KEY: &str = "ERROR NO KEY";
const SUCCESS_KILL_NODE: &str = "SUCCESS KILL ONE RAFT NODE";
const TIMEOUT: &str = "ERROR TIMEOUT";


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
    db: Arc<Mutex<HashMap<i64, String>>>,

    proposals: Arc<Mutex<VecDeque<Proposal>>>,

    node: Arc<Mutex<RawNode<MemStorage>>>,
    stop_signal: Arc<Mutex<Sender<Signal>>>,
}


impl Clone for KvRaftService {
    fn clone(&self) -> Self {
        let temp = KvRaftService {
            client_last_seq: self.client_last_seq.clone(),
            db: self.db.clone(),
            proposals: self.proposals.clone(),
            node: self.node.clone(),
            stop_signal: self.stop_signal.clone(),
        };
        temp
    }

    fn clone_from(&mut self, source: &Self) {
        self.db = source.db.clone();
        self.client_last_seq = source.client_last_seq.clone();
        self.proposals = source.proposals.clone();
        self.node = source.node.clone();
        self.stop_signal = source.stop_signal.clone();
    }
}


/**
init the KvRaftService
**/
impl KvRaftService {
    fn new(p: Arc<Mutex<VecDeque<Proposal>>>,
           n: Arc<Mutex<RawNode<MemStorage>>>,
           s: Arc<Mutex<HashMap<i64, String>>>,
           sig: Arc<Mutex<Sender<Signal>>>) -> Self {
        Self {
            // mu: Mutex::new(KvRaftService),
            client_last_seq: Default::default(),
            db: s,
            proposals: p,
            node: n,
            stop_signal: sig,
            // kv_pairs: Default::default(),
            // agree_chs: Default::default(),
        }
    }
}

/**
start grpc server for client
**/
pub fn maintain_server(proposals: Arc<Mutex<VecDeque<Proposal>>>,
                       nodes: HashMap<usize, Arc<Mutex<RawNode<MemStorage>>>>,
                       stores: HashMap<usize, Arc<Mutex<HashMap<i64, String>>>>,
                       stop_sig: Sender<Signal>,
) {
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
    let port: Arc<Vec<u16>> = Arc::new(vec![5030, 5031, 5032]);
    let mut handles = Vec::new();
    let stop = Arc::new(Mutex::new(stop_sig));
    for i in 0..3usize {
        let raft_group = Arc::clone(&nodes.get(&i).unwrap());
        // let store = stores.get(&i);
        let store = Arc::clone(&stores.get(&i).unwrap());
        // let ref mut raft_group1 = raft_group.lock().unwrap();
        // let mut raft_group2 = raft_group1.as_mut();
        // let raft_group2 = match raft_group2 {
        //     Some(ref mut r) => {
        //         println!("{} init", node.0);
        //         r
        //     },
        //     // When Node::raft_group is `None` it means the node is not initialized.
        //     _ => {
        //         println!("{} not init", node.0);
        //         continue
        //     },
        // };

        let proposals = Arc::clone(&proposals);
        let port = Arc::clone(&port);
        let stop = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let env = Arc::new(Environment::new(1));
            let kv_raft_server = KvRaftService::new(proposals, raft_group, store, stop);

            let service = kvraft_grpc::create_kv_raft(kv_raft_server);
            let mut server = ServerBuilder::new(env)
                .register_service(service)
                .bind(HOST, port[i])
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
        );
        handles.push(handle);
    }
    // for node in nodes {}

    // Wait for the thread to finish
    for th in handles {
        th.join().unwrap();
    }
}


/**
implement KvRaft Method in KvRaftService
**/
impl KvRaft for KvRaftService {
    fn get(&mut self, ctx: RpcContext, args: GetArgs, sink: UnarySink<GetReply>) {
        println!("Received Get request {{ {:?} }}", args);
        let mut get_reply = GetReply::new();

        let raft_group = Arc::clone(&self.node);
        // let raft_group = raft_group.lock().unwrap();
        // let raft_group = raft_group.unwrap();
        if raft_group.lock().unwrap().raft.state != StateRole::Leader {
            get_reply.set_success(false);
            get_reply.set_msg(String::from(ERR_LEADER));
        } else {
            let (proposal, rx) = Proposal::normal(args.get_client_id().to_string(), args.get_serial_num().to_string());
            self.proposals.lock().unwrap().push_back(proposal);

            if rx.recv().unwrap() {
                if let Some(val) = self.db.lock().unwrap().get(&args.get_key()) {
                    get_reply.set_success(true);
                    get_reply.set_val(val.to_string().clone());
                } else {
                    get_reply.set_success(false);
                    get_reply.set_msg(String::from(ERR_KEY));
                }
            } else {
                get_reply.set_success(false);
                get_reply.set_msg(String::from(ERR_LEADER));
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

        let raft_group = Arc::clone(&self.node);
        // let raft_group = raft_group.lock().unwrap();
        // let raft_group = raft_group.unwrap();
        if raft_group.lock().unwrap().raft.state != StateRole::Leader {
            put_reply.set_success(false);
            put_reply.set_msg(String::from(ERR_LEADER));
        } else {
            let (proposal, rx) = Proposal::normal(args.get_client_id().to_string(), args.get_serial_num().to_string());
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

            // if let Some(val) = self.db.lock().unwrap().get(&args.key) {
            //     println!("key: {}, val: {}", args.key, val);
            // }
        }

        let f = sink.success(put_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with PutReply {{ {:?} }}", put_reply));
        ctx.spawn(f)
    }

    fn delete(&mut self, ctx: RpcContext, args: DeleteArgs, sink: UnarySink<DeleteReply>) {
        println!("Received Delete request {{ {:?} }}", args);
        let mut delete_reply = DeleteReply::new();

        let raft_group = Arc::clone(&self.node);
        // let raft_group = raft_group.lock().unwrap();
        // let raft_group = raft_group.unwrap();
        if raft_group.lock().unwrap().raft.state != StateRole::Leader {
            delete_reply.set_success(false);
            delete_reply.set_msg(String::from(ERR_LEADER));
        } else {
            let (proposal, rx) = Proposal::normal(args.get_client_id().to_string(), args.get_serial_num().to_string());
            self.proposals.lock().unwrap().push_back(proposal);

            if rx.recv().unwrap() {
                // /// if delete key < 0, it means delete one raft node
                // if t.elapsed() >= Duration::from_secs(5) {
                //     ///进入这里的是超时且没有受到消息
                //     delete_reply.set_success(false);
                //     delete_reply.set_msg(String::from(TIMEOUT));
                // } else {
                ///这里是正常收到消息
                if args.key < 0 {
                    self.stop_signal.lock().unwrap().send(Signal::Terminate).unwrap();
                    println!("kill: {}", self.node.lock().unwrap().raft.id);
                    delete_reply.set_success(true);
                    delete_reply.set_msg(String::from(SUCCESS_KILL_NODE));
                } else {
                    delete_reply.set_success(true);
                    self.db.lock().unwrap().remove(&args.get_key());
                }
            } else {
                delete_reply.set_success(false);
                delete_reply.set_msg(String::from(ERR_LEADER));
            }
        }

        let f = sink.success(delete_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", delete_reply));

        ctx.spawn(f)
    }

    fn scan(&mut self, ctx: RpcContext, args: ScanArgs, sink: UnarySink<ScanReply>) {
        println!("Received Scan request {{ {:?} }}", args);
        let mut scan_reply = ScanReply::new();
        let raft_group = Arc::clone(&self.node);
        // let raft_group = raft_group.lock().unwrap();
        // let raft_group = raft_group.unwrap();
        if raft_group.lock().unwrap().raft.state != StateRole::Leader {
            scan_reply.set_success(false);
            scan_reply.set_msg(String::from(ERR_LEADER));
        } else {
            let (proposal, rx) = Proposal::normal(args.get_client_id().to_string(), args.get_serial_num().to_string());
            self.proposals.lock().unwrap().push_back(proposal);
            if rx.recv().unwrap() {
                let mut res = Vec::new();
                for (&key, _) in self.db.lock().unwrap().iter() {
                    if (key >= args.start_key && key < args.end_key) || args.start_key == -1 {
                        res.push(key);
                    }
                }
                res.sort();
                scan_reply.set_success(true);
                scan_reply.set_keys(res);
            } else {
                scan_reply.set_success(true);
                scan_reply.set_msg(String::from(ERR_LEADER));
            }
        }
        let f = sink.success(scan_reply.clone())
            .map_err(move |err| eprintln!("Failed to reply: {:?}", err))
            .map(move |_| println!("Responded with GetReply {{ {:?} }}", scan_reply));

        ctx.spawn(f)
    }
}
