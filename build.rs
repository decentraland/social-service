extern crate prost_build;
use reqwest::{header::USER_AGENT, Url};
use std::{
    env,
    io::{Cursor, Result},
    time::Duration,
};

const DCL_PROTOCOL_REPO_URL: &str =
    "https://api.github.com/repos/decentraland/protocol/contents/proto/decentraland";
const FRIENDSHIP_PROTO_PATH: &str = "/social/friendships/friendships.proto";
/// Modify this value to update the proto version, it is the commit sha from protocol repo used for downloading the proto file
const FRIENDSHIPS_PROTOCOL_VERSION: &str = "5f70bff8c33a5639a7f9bb962aee965c017ad484";
const DEFINITIONS_FOLDER: &str = "ext-proto";
const PROTO_FILE_DEST: &str = "ext-proto/friendships.proto";

fn main() -> Result<()> {
    if should_download_proto() {
        download_proto_from_github()?;
    }
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=ext-proto/friendships.proto");

    let mut prost_config = prost_build::Config::new();
    prost_config.protoc_arg("--experimental_allow_proto3_optional");
    prost_config.service_generator(Box::new(dcl_rpc::codegen::RPCServiceGenerator::new()));
    prost_config.compile_protos(&[PROTO_FILE_DEST], &[DEFINITIONS_FOLDER])?;
    Ok(())
}

/// Avoid the GitHub Request if the file exists and has been modified in the last hour.
/// It will return `true` if the file has not been modified in the last hour or doesn't exist.
/// If the file has been modified within the last hour, the function will return `false`.
fn should_download_proto() -> bool {
    if let Ok(cwd) = env::current_dir() {
        let path = cwd.join(PROTO_FILE_DEST);
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if modified
                    .elapsed()
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    < Duration::from_secs(3600)
                {
                    return false;
                }
            }
        }
    }
    true
}

fn download_proto_from_github() -> Result<()> {
    let client = reqwest::blocking::Client::new();

    let file_url = build_github_url_to_download();
    let file_metadata = get_file_info(&client, file_url);

    let content_url = extract_file_url(file_metadata);
    let content = download_file(client, content_url);

    save_content_to_file(content)
}

fn save_content_to_file(content: reqwest::blocking::Response) -> Result<()> {
    let cwd = env::current_dir()?;
    // Create folder if missing
    std::fs::create_dir_all(String::from(cwd.to_string_lossy()) + "/" + DEFINITIONS_FOLDER)?;

    let file_path: String = String::from(cwd.to_string_lossy()) + "/" + PROTO_FILE_DEST;
    // Create destination file
    let mut file = std::fs::File::create(file_path)?;
    let inner = match content.bytes() {
        Ok(i) => i,
        Err(err) => panic!("There was an error reading content, {err}"),
    };
    let mut content = Cursor::new(inner);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

fn download_file(client: reqwest::blocking::Client, file_url: Url) -> reqwest::blocking::Response {
    match client
        .get(file_url)
        .header(USER_AGENT, "Social Service")
        .send()
    {
        Ok(it) => it,
        Err(err) => panic!("Failed to download the friendship proto def with {err}"),
    }
}

fn extract_file_url(body: serde_json::Value) -> Url {
    let file_url = body["download_url"]
        .as_str()
        .expect("Failed to obtain download_url from response");

    Url::parse(file_url).expect("Failed parse URL from response")
}

fn get_file_info(client: &reqwest::blocking::Client, url: Url) -> serde_json::Value {
    let res = match client.get(url).header(USER_AGENT, "Social Service").send() {
        Ok(it) => it,
        Err(err) => panic!("Failed to get file info with {err}"),
    };
    match res.json::<serde_json::Value>() {
        Ok(body) => body,
        Err(err) => panic!("Failed to parse response as JSON: {err}"),
    }
}

fn build_github_url_to_download() -> Url {
    let github_url = format!(
        "{DCL_PROTOCOL_REPO_URL}{FRIENDSHIP_PROTO_PATH}?ref={FRIENDSHIPS_PROTOCOL_VERSION}"
    );

    match Url::parse(&github_url) {
        Ok(it) => it,
        Err(err) => panic!("Failed parse URL with {err}"),
    }
}
