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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Synapse {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub synapse: Synapse,
    pub env: String, // prd / stg / dev / biz
}

const SYNAPSE_URL_ENV: &str = "SYNAPSE_URL";
const ENV_VAR: &str = "ENV";

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();
        log::debug!("Args: {:#?}", args);

        let config = config::Config::builder()
            .add_source(File::with_name("configuration"))
            .add_source(
                config::Environment::default()
                    .with_list_parse_key(SYNAPSE_URL_ENV)
                    .with_list_parse_key(ENV_VAR)
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(" "),
            )
            .set_override_option("server.host", args.host)?
            .set_override_option("server.port", args.port)?
            .set_default("synapse.url", "https://synapse.decentraland.zone")?
            .set_default("env", "dev")?
            .build()?;

        config.try_deserialize()
    }
}
