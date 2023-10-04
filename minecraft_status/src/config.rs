use anyhow::{anyhow, Result};
use dns::domain_lookup;
use log::warn;
use std::net::IpAddr;
use std::time::Duration;

/// Default refresh interval (60 seconds)
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
/// Default port (25565)
const DEFAULT_PORT: u16 = 25565;

/// Stores configuration loaded at program start
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Config {
    /// How often to refresh data
    pub(crate) refresh_interval: Duration,
    /// Ip to check minecraft status for
    pub(crate) ip: IpAddr,
    /// Port minecraft server is listening on
    pub(crate) port: u16,
}

impl Config {
    /// Creates a config from set env vars
    pub fn from_env_vars() -> Result<Self> {
        // get refresh interval from end var, try and parse it, and use default if any steps fail
        let refresh_interval = {
            let refresh_interval_str = std::env::var("REFRESH_INTERVAL").ok();

            // 3 cases we care about:
            // REFRESH_INTERVAL has value and it's valid duration -> use that duration
            // REFRESH_INTERVAL has value but not a valid duration -> use default duration and log invalid
            // REFRESH_INTERVAL has no value -> use default
            match refresh_interval_str {
                Some(duration_str) => match parse_duration::parse(&duration_str) {
                    Ok(duration) => duration,
                    Err(_) => {
                        warn!("env var `REFRESH_INTERVAL` has invalid value `{duration_str}`");
                        DEFAULT_REFRESH_INTERVAL
                    }
                },
                _ => DEFAULT_REFRESH_INTERVAL,
            }
        };

        let (ip, port) = {
            // get server and port from env vars
            let server =
                std::env::var("SERVER").map_err(|_| anyhow!("env var `SERVER` is missing"))?;

            let port_string = std::env::var("SERVER_PORT").ok();
            // same logic as refresh_interval above
            let port = match port_string {
                Some(port_string) => match port_string.parse() {
                    Ok(port) => port,
                    Err(_) => {
                        warn!("env var `SERVER_PORT` has invalid value `{port_string}`");
                        DEFAULT_PORT
                    }
                },
                _ => DEFAULT_PORT,
            };

            // then perform a lookup to find ip to use
            domain_lookup(&server, port)?
        };

        Ok(Self {
            refresh_interval,
            ip,
            port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let _ = dotenvy::dotenv();
        let config = Config::from_env_vars();

        let ip = std::env::var("TEST_IP").unwrap().parse().unwrap();
        let port = std::env::var("TEST_PORT").unwrap().parse().unwrap();

        assert!(config.is_ok());
        assert_eq!(
            config.unwrap(),
            Config {
                refresh_interval: Duration::from_secs(30),
                ip,
                port,
            }
        );
    }

    #[test]
    fn test_domain_lookup() {
        let _ = dotenvy::dotenv();

        let url = std::env::var("TEST_URL").unwrap();

        let ip = std::env::var("TEST_IP").unwrap().parse().unwrap();
        let port = std::env::var("TEST_PORT").unwrap().parse().unwrap();

        let result = domain_lookup(&url, DEFAULT_PORT);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (ip, port));
    }
}