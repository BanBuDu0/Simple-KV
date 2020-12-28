extern crate protoc_grpcio;

fn main() {
    // protoc_rust_grpc::run(protoc_rust_grpc::Args {
    //     out_dir: "src/rpc",
    //     includes: &[],
    //     input: &["src/proto/kvraft.proto"],
    //     rust_protobuf: true,
    // }).expect("protoc-rust-grpc");
    let proto_root = "src/proto";
    println!("cargo:rerun-if-changed={}", proto_root);
    protoc_grpcio::compile_grpc_protos(&["eraftpb.proto", "kvraft.proto"], &[proto_root], &proto_root, None)
        .expect("Failed to compile gRPC definitions!");
}