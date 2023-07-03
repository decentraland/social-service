use clap::Parser;
use config::{ConfigError, File};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Host
    #[clap(short, long, value_parser)]
    host: Option<String>,

    /// Port to expose the server
    #[clap(short, long, value_parser)]
    port: Option<i16>,

    /// RPC WS Host
    #[clap(long, value_parser)]
    rpc_host: Option<String>,

    /// RPC WS Port to expose the server
    #[clap(long, value_parser)]
    rpc_port: Option<i16>,

    /// RPC WS Ping interval in seconds
    #[clap(long, value_parser)]
    rpc_ping_interval_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcServerConfig {
    pub port: u16,
    pub ping_interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Synapse {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub host: String,
    pub name: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub rpc_server: RpcServerConfig,
    pub synapse: Synapse,
    pub db: Database,
    pub env: String, // prd / stg / dev / biz
    pub wkc_metrics_bearer_token: String,
    pub redis: RedisConfig,
    pub cache_hashing_key: String,
    pub friends_stream_page_size: u16,
}

const SYNAPSE_URL_ENV: &str = "SYNAPSE_URL";
const ENV_VAR: &str = "ENV";
const METRICS_TOKEN: &str = "WKC_METRICS_BEARER_TOKEN";
const DB_HOST: &str = "DB_HOST";
const DB_USER: &str = "DB_USER";
const DB_PWD: &str = "DB_PASSWORD";
const DB_NAME: &str = "DB_NAME";

const REDIS_HOST: &str = "REDIS_HOST";

const CACHE_HASHING_KEY: &str = "CACHE_HASHING_KEY";

const FRIENDS_STREAM_PAGE_SIZE: &str = "FRIENDS_STREAM_PAGE_SIZE";

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();
        log::debug!("Args: {:#?}", args);

        let config = config::Config::builder()
            .add_source(File::with_name("configuration"))
            .add_source(
                config::Environment::default()
                    .with_list_parse_key(SYNAPSE_URL_ENV)
                    .with_list_parse_key(DB_HOST)
                    .with_list_parse_key(DB_USER)
                    .with_list_parse_key(DB_PWD)
                    .with_list_parse_key(DB_NAME)
                    .with_list_parse_key(REDIS_HOST)
                    .with_list_parse_key(FRIENDS_STREAM_PAGE_SIZE)
                    .try_parsing(true)
                    .separator("_"),
            )
            .add_source(
                config::Environment::default()
                    .with_list_parse_key(CACHE_HASHING_KEY)
                    .with_list_parse_key(METRICS_TOKEN)
                    .with_list_parse_key(ENV_VAR)
                    .try_parsing(true),
            )
            .set_override_option("server.port", args.port)?
            .set_override_option("rpc_server.port", args.rpc_port)?
            .set_override_option(
                "rpc_server.ping_interval_seconds",
                args.rpc_ping_interval_seconds,
            )?
            .set_default("rpc_server.ping_interval_seconds", 30)?
            .set_default("synapse.url", "https://synapse.decentraland.zone")?
            .set_default("env", "dev")?
            .set_default("wkc_metrics_bearer_token", "")?
            .set_default("db.host", "0.0.0.0:3500")? // docker-compose -> local env
            .set_default("db.user", "postgres")? // docker-compose -> local env
            .set_default("db.password", "postgres")? // docker-compose -> local env
            .set_default("db.name", "social_service")? // docker-compose -> local env
            .set_default("redis.host", "0.0.0.0")? // docker-compose -> local env
            .set_default("cache_hashing_key", "test_key")? // docker-compose -> local env
            .set_default("friends_stream_page_size", 20)?
            .build()?;

        config.try_deserialize()
    }
}
