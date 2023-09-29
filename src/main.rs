mod config;

use anyhow::Result;
use config::Config;
use log::LevelFilter;

fn main() -> Result<()> {
    // read env file and init logger with default warn level
    let _ = dotenv::dotenv();
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .env()
        .init()
        .unwrap();

    let config = Config::from_env_vars()?;
    println!("{config:?}");

    Ok(())
}
