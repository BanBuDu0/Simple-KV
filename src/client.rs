extern crate grpcio;
extern crate proto;

use std::io;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use grpcio::{ChannelBuilder, EnvBuilder};

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
    let ports: Arc<Vec<u16>> = Arc::new(vec![5030, 5031, 5032]);
    // const PORT: u16 = 5030;
    const CLIENT_ID: i64 = 1;
    // let env = Arc::new(EnvBuilder::new().build());
    // let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", ports[*leader_id.lock().unwrap()]).as_str());
    // let client = KvRaftClient::new(ch);

    let serial_num = Arc::new(Mutex::new(1));
    let leader_id = Arc::new(Mutex::new(1));
    println!("Command");
    println!("put i64 String");
    println!("get i64");
    println!("delete i64");
    println!("scan i64 i64");
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

            let key = commands[1];

            match key.parse::<i64>() {
                Ok(temp) => {
                    if temp < 0 {
                        println!("Input positive integer key");
                        continue;
                    }
                    let mut get_args = GetArgs::new();
                    get_args.set_client_id(CLIENT_ID);
                    get_args.set_key(temp);
                    get_args.set_serial_num(*num);
                    let leader_id = Arc::clone(&leader_id);
                    loop {
                        let env = Arc::new(EnvBuilder::new().build());
                        let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", ports[*leader_id.lock().unwrap()]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.get(&get_args).expect("RPC Failed");
                        if ok.success {
                            println!("Get Success: {}, msg: {}, Val: {}", ok.success, ok.msg, ok.val);
                            break;
                        } else {
                            if ok.msg.eq(&String::from("ERROR NO KEY")) {
                                println!("Get Success: {}, msg: {}, Val: {}", ok.success, ok.msg, ok.val);
                                break;
                            } else {
                                *leader_id.lock().unwrap() += 1;
                                *leader_id.lock().unwrap() %= 3;

                                println!("Get Success: {}, msg: {}, Val: {}, retry other server: {}", ok.success, ok.msg, ok.val, *leader_id.lock().unwrap());
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("Input i64 key");
                    continue;
                }
            }
        } else if commands[0].to_lowercase() == "put" {
            if commands.len() < 3 {
                println!("Input put's key and value");
                continue;
            }
            let key = commands[1];
            let val = commands[2];

            match key.parse::<i64>() {
                Ok(temp) => {
                    if temp < 0 {
                        println!("Input positive integer key");
                        continue;
                    }
                    let mut put_args = PutArgs::new();
                    put_args.set_client_id(CLIENT_ID);
                    put_args.set_key(temp);
                    put_args.set_val(String::from(val.clone()));
                    put_args.set_serial_num(*num);
                    let leader_id = Arc::clone(&leader_id);
                    loop {
                        let env = Arc::new(EnvBuilder::new().build());
                        let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", ports[*leader_id.lock().unwrap()]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.put(&put_args).expect("RPC Failed");
                        if ok.success {
                            println!("Put Success: {}, msg: {}", ok.success, ok.msg);
                            break;
                        } else {
                            *leader_id.lock().unwrap() += 1;
                            *leader_id.lock().unwrap() %= 3;
                            println!("Put Success: {}, msg: {}, retry other server: {}", ok.success, ok.msg, *leader_id.lock().unwrap());
                        }
                    }
                }
                Err(_) => {
                    println!("Input i64 key");
                    continue;
                }
            }
        } else if commands[0].to_lowercase() == "delete" {
            if commands.len() == 1 {
                println!("Input delete's key");
                continue;
            }

            let key = commands[1];
            match key.parse::<i64>() {
                Ok(temp) => {
                    if temp < 0 {
                        println!("Input positive integer key");
                        continue;
                    }

                    let mut delete_args = DeleteArgs::new();

                    delete_args.set_client_id(CLIENT_ID);
                    delete_args.set_key(temp);
                    delete_args.set_serial_num(*num);
                    let leader_id = Arc::clone(&leader_id);
                    loop {
                        let env = Arc::new(EnvBuilder::new().build());
                        let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", ports[*leader_id.lock().unwrap()]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.delete(&delete_args).expect("RPC Failed");
                        if ok.success {
                            println!("Delete Success: {}, msg: {}", ok.success, ok.msg);
                            break;
                        } else {
                            *leader_id.lock().unwrap() += 1;
                            *leader_id.lock().unwrap() %= 3;
                            println!("Delete Success: {}, msg: {}, retry other server: {}", ok.success, ok.msg, *leader_id.lock().unwrap());
                        }
                    }
                }
                Err(_) => {
                    println!("Input i64 key");
                    continue;
                }
            }
        } else if commands[0].to_lowercase() == "scan" {
            if commands.len() == 1 {
                println!("Input scan's start key and end key");
                continue;
            }

            let start_index: i64;
            let end_index: i64;

            match commands[1].parse::<i64>() {
                Ok(temp) => {
                    start_index = temp;
                }
                Err(_) => {
                    println!("Input i64 start_index");
                    continue;
                }
            }

            let mut scan_args = ScanArgs::new();
            if commands.len() == 2 {
                scan_args.set_serial_num(*num);
                scan_args.set_client_id(CLIENT_ID);
                scan_args.set_start_key(start_index);
                scan_args.set_end_key(start_index + 10);
            }

            if commands.len() == 3 {
                match commands[2].parse::<i64>() {
                    Ok(temp) => {
                        end_index = temp;
                    }
                    Err(_) => {
                        println!("Input i64 end_index");
                        continue;
                    }
                }

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
            let leader_id = Arc::clone(&leader_id);
            loop {
                let env = Arc::new(EnvBuilder::new().build());
                let ch = ChannelBuilder::new(env).connect(format!("localhost:{}", ports[*leader_id.lock().unwrap()]).as_str());
                let client = KvRaftClient::new(ch);
                let ok = client.scan(&scan_args).expect("RPC Failed");
                if ok.success {
                    println!("Delete Success: {}, msg: {}, {{ {:?} }}", ok.success, ok.msg, ok.keys);
                    break;
                } else {
                    *leader_id.lock().unwrap() += 1;
                    *leader_id.lock().unwrap() %= 3;
                    println!("Scan Success: {}, msg: {}, retry other server: {}", ok.success, ok.msg, *leader_id.lock().unwrap());
                }
            }
        } else if commands[0].to_lowercase() == "exit" {
            return;
        }else{
            println!("Command");
            println!("put i64 String");
            println!("get i64");
            println!("delete i64");
            println!("scan i64 i64");
        }
    }
}
