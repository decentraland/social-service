extern crate prost_build;
use reqwest::{header::USER_AGENT, Url};
use std::{
    env,
    io::{Cursor, Result},
};

fn main() -> Result<()> {
    // Download the proto file from GitHub
    let url = match Url::parse("https://api.github.com/repos/decentraland/protocol/contents/proto/decentraland/social/friendships/friendships.proto"){
        Ok(it) => it,
        Err(err) => panic!("Failed parse URL with {}", err),
    };
    // Need to be blocking, not async
    let client = reqwest::blocking::Client::new();
    let res = match client.get(url).header(USER_AGENT, "Social Service").send() {
        Ok(it) => it,
        Err(err) => panic!("Failed to get file info with {}", err),
    };
    let body = res.json::<serde_json::Value>().unwrap();
    let file_url = body.get("download_url").unwrap().as_str().unwrap();
    let file_url = match Url::parse(file_url) {
        Ok(it) => it,
        Err(err) => panic!("Failed parse URL with {}", err),
    };
    let response = match client
        .get(file_url)
        .header(USER_AGENT, "Social Service")
        .send()
    {
        Ok(it) => it,
        Err(err) => panic!("Failed to download the friendship proto def with {}", err),
    };

    // Store local
    let cwd = env::current_dir().unwrap();
    let file_path: String = String::from(cwd.to_string_lossy()) + "/ext-proto/downloaded.proto";
    let mut file = std::fs::File::create(file_path)?;
    let mut content = Cursor::new(response.bytes().unwrap());
    std::io::copy(&mut content, &mut file)?;

    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=ext-proto/friendships.proto");

    let mut conf = prost_build::Config::new();
    conf.service_generator(Box::new(dcl_rpc_codegen::RPCServiceGenerator::new()));
    conf.compile_protos(&["ext-proto/friendships.proto"], &["ext-proto"])?;
    Ok(())
}
