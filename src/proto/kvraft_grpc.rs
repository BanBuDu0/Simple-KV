// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_KV_RAFT_GET: ::grpcio::Method<super::kvraft::GetArgs, super::kvraft::GetReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kvraft.KVRaft/Get",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_KV_RAFT_PUT: ::grpcio::Method<super::kvraft::PutArgs, super::kvraft::PutReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kvraft.KVRaft/Put",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_KV_RAFT_DELETE: ::grpcio::Method<super::kvraft::DeleteArgs, super::kvraft::DeleteReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kvraft.KVRaft/Delete",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_KV_RAFT_SCAN: ::grpcio::Method<super::kvraft::ScanArgs, super::kvraft::ScanReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kvraft.KVRaft/Scan",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct KvRaftClient {
    client: ::grpcio::Client,
}

impl KvRaftClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        KvRaftClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn get_opt(&self, req: &super::kvraft::GetArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvraft::GetReply> {
        self.client.unary_call(&METHOD_KV_RAFT_GET, req, opt)
    }

    pub fn get(&self, req: &super::kvraft::GetArgs) -> ::grpcio::Result<super::kvraft::GetReply> {
        self.get_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_async_opt(&self, req: &super::kvraft::GetArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::GetReply>> {
        self.client.unary_call_async(&METHOD_KV_RAFT_GET, req, opt)
    }

    pub fn get_async(&self, req: &super::kvraft::GetArgs) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::GetReply>> {
        self.get_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn put_opt(&self, req: &super::kvraft::PutArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvraft::PutReply> {
        self.client.unary_call(&METHOD_KV_RAFT_PUT, req, opt)
    }

    pub fn put(&self, req: &super::kvraft::PutArgs) -> ::grpcio::Result<super::kvraft::PutReply> {
        self.put_opt(req, ::grpcio::CallOption::default())
    }

    pub fn put_async_opt(&self, req: &super::kvraft::PutArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::PutReply>> {
        self.client.unary_call_async(&METHOD_KV_RAFT_PUT, req, opt)
    }

    pub fn put_async(&self, req: &super::kvraft::PutArgs) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::PutReply>> {
        self.put_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_opt(&self, req: &super::kvraft::DeleteArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvraft::DeleteReply> {
        self.client.unary_call(&METHOD_KV_RAFT_DELETE, req, opt)
    }

    pub fn delete(&self, req: &super::kvraft::DeleteArgs) -> ::grpcio::Result<super::kvraft::DeleteReply> {
        self.delete_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_async_opt(&self, req: &super::kvraft::DeleteArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::DeleteReply>> {
        self.client.unary_call_async(&METHOD_KV_RAFT_DELETE, req, opt)
    }

    pub fn delete_async(&self, req: &super::kvraft::DeleteArgs) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::DeleteReply>> {
        self.delete_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn scan_opt(&self, req: &super::kvraft::ScanArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvraft::ScanReply> {
        self.client.unary_call(&METHOD_KV_RAFT_SCAN, req, opt)
    }

    pub fn scan(&self, req: &super::kvraft::ScanArgs) -> ::grpcio::Result<super::kvraft::ScanReply> {
        self.scan_opt(req, ::grpcio::CallOption::default())
    }

    pub fn scan_async_opt(&self, req: &super::kvraft::ScanArgs, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::ScanReply>> {
        self.client.unary_call_async(&METHOD_KV_RAFT_SCAN, req, opt)
    }

    pub fn scan_async(&self, req: &super::kvraft::ScanArgs) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvraft::ScanReply>> {
        self.scan_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait KvRaft {
    fn get(&mut self, ctx: ::grpcio::RpcContext, req: super::kvraft::GetArgs, sink: ::grpcio::UnarySink<super::kvraft::GetReply>);
    fn put(&mut self, ctx: ::grpcio::RpcContext, req: super::kvraft::PutArgs, sink: ::grpcio::UnarySink<super::kvraft::PutReply>);
    fn delete(&mut self, ctx: ::grpcio::RpcContext, req: super::kvraft::DeleteArgs, sink: ::grpcio::UnarySink<super::kvraft::DeleteReply>);
    fn scan(&mut self, ctx: ::grpcio::RpcContext, req: super::kvraft::ScanArgs, sink: ::grpcio::UnarySink<super::kvraft::ScanReply>);
}

pub fn create_kv_raft<S: KvRaft + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_KV_RAFT_GET, move |ctx, req, resp| {
        instance.get(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_KV_RAFT_PUT, move |ctx, req, resp| {
        instance.put(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_KV_RAFT_DELETE, move |ctx, req, resp| {
        instance.delete(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_KV_RAFT_SCAN, move |ctx, req, resp| {
        instance.scan(ctx, req, resp)
    });
    builder.build()
}
