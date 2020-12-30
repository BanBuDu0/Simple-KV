extern crate grpcio;
extern crate proto;

use std::{env, io};
use std::io::prelude::*;
use std::sync::{Arc, atomic, Mutex};

use grpcio::{ChannelBuilder, EnvBuilder};
use rand::random;

use proto::kvraft::{DeleteArgs, GetArgs, PutArgs};
use proto::kvraft_grpc::KvRaftClient;

fn read_line_buffer() -> String {
    // Read one line of input buffer-style
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}


fn print_single_line(text: &str) {
    // We can print lines without adding a newline
    print!("{}", text);
    // However, we need to flush stdout afterwards
    // in order to guarantee that the data actually displays
    io::stdout().flush().expect("Failed to flush stdout");
}

fn main() {
    const PORT: u16 = 5030;
    const CLIENT_ID: i64 = 1;
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", PORT).as_str());
    // let client = DinerClient::new(ch);
    let client = KvRaftClient::new(ch);
    let mut serial_num = 1;
    loop {
        print_single_line("Please enter command: ");
        let commands_raw = read_line_buffer();

        let commands: Vec<&str> = commands_raw.split_whitespace().collect();
        if commands[0].to_lowercase() == "get" {
            let mut key = commands[1];
            let mut get_args = GetArgs::new();
            serial_num += 1;
            get_args.set_client_id(CLIENT_ID);
            get_args.set_key(String::from(key.clone()));
            get_args.set_serial_num(serial_num.clone());

            let ok = client.get(&get_args).expect("RPC Failed");
            println!("Get Success: {}, msg: {}, Val: {}", ok.success, ok.msg, ok.val);
        } else if commands[0].to_lowercase() == "put" {
            let key = commands[1];
            let val = commands[2];
            // let mut sn = serial_num.lock().unwrap();
            // *sn += 1;
            serial_num += 1;
            let mut put_args = PutArgs::new();
            put_args.set_client_id(CLIENT_ID);
            put_args.set_key(String::from(key.clone()));
            put_args.set_val(String::from(val.clone()));
            put_args.set_serial_num(serial_num.clone());

            let ok = client.put(&put_args).expect("RPC Failed");
            println!("Get Success: {}, msg: {}", ok.success, ok.msg);
        } else if commands[0].to_lowercase() == "delete" {
            let mut key = commands[1];
            let mut delete_args = DeleteArgs::new();

            serial_num += 1;
            delete_args.set_client_id(CLIENT_ID);
            delete_args.set_key(String::from(key.clone()));
            delete_args.set_serial_num(serial_num.clone());

            let ok = client.delete(&delete_args).expect("RPC Failed");
            println!("Get Success: {}, msg: {}", ok.success, ok.msg);
        } else if commands[0].to_lowercase() == "exit" {
            return;
        }
    }
}
