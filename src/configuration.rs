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
}

const SYNAPSE_URL_ENV: &str = "SYNAPSE_URL";

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();
        log::debug!("Args: {:#?}", args);

        let config = config::Config::builder()
            .add_source(File::with_name("configuration"))
            .add_source(
                config::Environment::default()
                    .with_list_parse_key(SYNAPSE_URL_ENV)
                    .try_parsing(true)
                    .separator("_"),
            )
            .set_override_option("server.host", args.host)?
            .set_override_option("server.port", args.port)?
            .set_default("synapse.url", "https://synapse.decentraland.zone")?
            .build()?;

        config.try_deserialize()
    }
}
