extern crate grpcio;
extern crate proto;

use std::{env, io};
use std::io::prelude::*;
use std::sync::{Arc, atomic, Mutex};

use grpcio::{ChannelBuilder, EnvBuilder};
use rand::random;

use proto::kvraft::{DeleteArgs, GetArgs, PutArgs, ScanArgs};
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
    let mut serial_num = Arc::new(Mutex::new(1));
    loop {
        print_single_line("Please enter command: ");
        let commands_raw = read_line_buffer();

        let commands: Vec<&str> = commands_raw.split_whitespace().collect();
        if commands.len() == 0 { continue; }
        let mut num = serial_num.lock().unwrap();
        *num += 1;
        if commands[0].to_lowercase() == "get" {
            if commands.len() == 1 {
                println!("Input get's key");
                continue;
            }

            let mut key = commands[1];
            let mut get_args = GetArgs::new();
            get_args.set_client_id(CLIENT_ID);
            get_args.set_key(key.parse::<i64>().unwrap());
            get_args.set_serial_num(*num);

            let ok = client.get(&get_args).expect("RPC Failed");
            println!("Get Success: {}, msg: {}, Val: {}", ok.success, ok.msg, ok.val);
        } else if commands[0].to_lowercase() == "put" {
            if commands.len() < 3 {
                println!("Input put's key and value");
                continue;
            }
            let key = commands[1];
            let val = commands[2];

            let mut put_args = PutArgs::new();
            put_args.set_client_id(CLIENT_ID);
            put_args.set_key(key.parse::<i64>().unwrap());
            put_args.set_val(String::from(val.clone()));

            put_args.set_serial_num(*num);

            let ok = client.put(&put_args).expect("RPC Failed");
            println!("Put Success: {}, msg: {}", ok.success, ok.msg);
        } else if commands[0].to_lowercase() == "delete" {
            if commands.len() == 1 {
                println!("Input delete's key");
                continue;
            }

            let mut key = commands[1];
            let mut delete_args = DeleteArgs::new();

            delete_args.set_client_id(CLIENT_ID);
            delete_args.set_key(key.parse::<i64>().unwrap());
            delete_args.set_serial_num(*num);

            let ok = client.delete(&delete_args).expect("RPC Failed");
            println!("Delete Success: {}, msg: {}", ok.success, ok.msg);
        } else if commands[0].to_lowercase() == "scan" {
            if commands.len() == 1 {
                println!("Input scan's start key and end key");
                continue;
            }

            let mut scan_args = ScanArgs::new();
            if commands.len() == 2 {
                let mut start_index = commands[1].parse::<i64>().unwrap();
                scan_args.set_serial_num(*num);
                scan_args.set_client_id(CLIENT_ID);
                scan_args.set_start_key(start_index);
                scan_args.set_end_key(start_index + 10);
            }

            if commands.len() == 3 {
                let mut start_index = commands[1].parse::<i64>().unwrap();
                let mut end_index = commands[2].parse::<i64>().unwrap();
                scan_args.set_serial_num(*num);
                scan_args.set_client_id(CLIENT_ID);
                scan_args.set_start_key(start_index);
                if start_index < 0 {
                    scan_args.set_end_key(-1);
                } else {
                    scan_args.set_end_key(start_index + 10);
                    scan_args.set_end_key(end_index);
                }
            }

            let ok = client.scan(&scan_args).expect("RPC Failed");
            println!("Delete Success: {}, msg: {}, {{ {:?} }}", ok.success, ok.msg, ok.keys);
        } else if commands[0].to_lowercase() == "exit" {
            return;
        }
    }
}
