extern crate grpcio;
extern crate proto;

use std::env;
use std::sync::Arc;

use grpcio::{ChannelBuilder, EnvBuilder};
use rand::random;

use proto::kvraft::{GetArgs, PutArgs};
use proto::kvraft_grpc::KvRaftClient;

fn main() {
    // let args = env::args().collect::<Vec<_>>();
    // if args.len() != 2 {
    //     panic!("Expected exactly one argument, the port to connect to.")
    // }
    // let port = args[1]
    //     .parse::<u16>()
    //     .unwrap_or_else(|_| panic!("{} is not a valid port number", args[1]));
    let port: u16 = 5030;
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", port).as_str());
    // let client = DinerClient::new(ch);
    let client = KvRaftClient::new(ch);
    let mut key = String::from("kkkk");
    let mut val = String::from("vvvvv");

    let mut put_args = PutArgs::new();
    put_args.set_client_id(1);
    put_args.set_key(key.clone());
    put_args.set_val(val.clone());
    put_args.set_serial_num(1);
    let ok = client.put(&put_args).expect("RPC Failed");
    println!("Put Success: {}, msg: {}", ok.success, ok.msg);

    let mut get_args = GetArgs::new();
    get_args.set_client_id(1);
    get_args.set_key(key.clone());
    get_args.set_serial_num(2);
    let ok = client.get(&get_args).expect("RPC Failed");
    println!("Get Success: {}, msg: {}, Val: {}", ok.success, ok.msg, ok.val);
}
