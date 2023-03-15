extern crate prost_build;
use std::io::Result;

fn main() -> Result<()> {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=src/ws/service/friendships.proto");

    let mut conf = prost_build::Config::new();
    conf.service_generator(Box::new(dcl_rpc_codegen::RPCServiceGenerator::new()));
    conf.compile_protos(&["src/ws/service/friendships.proto"], &["src/ws/service"])?;
    Ok(())
}
