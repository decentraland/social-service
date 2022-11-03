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
pub struct Config {
    pub server: Server,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();
        log::debug!("Args: {:#?}", args);

        let config = config::Config::builder()
            .add_source(File::with_name("configuration"))
            .set_override_option("server.host", args.host)?
            .set_override_option("server.port", args.port)?
            .build()?;

        config.try_deserialize()
    }
}
