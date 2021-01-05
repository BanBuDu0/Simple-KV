use criterion::Criterion;
use crate::{DEFAULT_GET_SETS, DEFAULT_PUT_SETS, DEFAULT_DELETE_SETS, DEFAULT_SCAN_SETS};
use proto::kvraft::{GetArgs, PutArgs, DeleteArgs, ScanArgs};
use std::sync::Arc;
use grpcio::{EnvBuilder, ChannelBuilder};
use proto::kvraft_grpc::KvRaftClient;


pub fn bench_client(c: &mut Criterion){
    bench_put(c);
    bench_get(c);
    bench_scan(c);
    bench_delete_kv(c);
}

pub fn bench_put(c: &mut Criterion){
    DEFAULT_PUT_SETS.iter().for_each(
        |(key ,val)| {
            c.bench_function(&format!("PUT {} {}", key, val), move |b|{
                let mut put_args = PutArgs::new();
                put_args.set_client_id(1);
                put_args.set_key(*key);
                put_args.set_val(String::from(*val));
                put_args.set_serial_num(1);
                let ports = vec![5030, 5031, 5032];
                let mut leader_id = 1;
                let env = Arc::new(EnvBuilder::new().build());
                b.iter(|| {
                    loop {
                        let ch = ChannelBuilder::new(Arc::clone(&env)).connect(format!("localhost:{}", ports[leader_id]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.put(&put_args).expect("RPC Failed");
                        if ok.success {
                            break;
                        } else {
                            leader_id += 1;
                            leader_id %= 3;
                        }
                    }
                })
            });
        }
    );
}

pub fn bench_get(c: &mut Criterion){
    DEFAULT_GET_SETS.iter().for_each(|key| {
        c.bench_function(&format!("GET {}", key), move |b|{
            let mut get_args = GetArgs::new();
            get_args.set_client_id(1);
            get_args.set_key(*key);
            get_args.set_serial_num(1);
            let ports = vec![5030, 5031, 5032];
            let mut leader_id = 1;
            let env = Arc::new(EnvBuilder::new().build());
            b.iter(|| {
                loop {
                    let ch = ChannelBuilder::new(Arc::clone(&env)).connect(format!("localhost:{}", ports[leader_id]).as_str());
                    let client = KvRaftClient::new(ch);
                    let ok = client.get(&get_args).expect("RPC Failed");
                    if ok.success {
                        break;
                    } else {
                        if ok.msg.eq(&String::from("ERROR NO KEY")) {
                            break;
                        } else {
                            leader_id += 1;
                            leader_id %= 3;
                        }
                    }
                }
            })
        });
    });
}

pub fn bench_delete_kv(c: &mut Criterion){
    DEFAULT_DELETE_SETS.iter().for_each(|key| {
        c.bench_function(&format!("DELETE {}", key), move |b|{
            let mut delete_args = DeleteArgs::new();
            delete_args.set_client_id(1);
            delete_args.set_key(*key);
            delete_args.set_serial_num(1);
            let ports = vec![5030, 5031, 5032];
            let mut leader_id = 1;
            let env = Arc::new(EnvBuilder::new().build());
            b.iter(
                || {
                    loop {
                        let ch = ChannelBuilder::new(Arc::clone(&env)).connect(format!("localhost:{}", ports[leader_id]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.delete(&delete_args).expect("RPC Failed");
                        if ok.success {
                            break;
                        } else {
                            leader_id += 1;
                            leader_id %= 3;
                        }
                    }
                }
            )
        });
    });
}

pub fn bench_scan(c: &mut Criterion){
    DEFAULT_SCAN_SETS.iter().for_each(|(start_index, end_index)| {
        c.bench_function(&format!("SCAN {} {}", start_index, end_index), move |b|{
            let mut scan_args = ScanArgs::new();
            scan_args.set_client_id(1);
            scan_args.set_serial_num(1);
            scan_args.set_start_key(*start_index);
            scan_args.set_end_key(*end_index);

            let ports = vec![5030, 5031, 5032];
            let mut leader_id = 1;
            let env = Arc::new(EnvBuilder::new().build());
            b.iter(
                || {
                    loop {
                        let ch = ChannelBuilder::new(Arc::clone(&env)).connect(format!("localhost:{}", ports[leader_id]).as_str());
                        let client = KvRaftClient::new(ch);
                        let ok = client.scan(&scan_args).expect("RPC Failed");
                        if ok.success {
                            break;
                        } else {
                            leader_id += 1;
                            leader_id %= 3;
                        }
                    }
                }
            )
        });
    });
}