use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use rustdns::{Class, Message, Resource, Type};
use std::net::{IpAddr, UdpSocket};
use std::str::FromStr;
use std::time::Duration;

/// Default refresh interval (60 seconds)
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
/// Default port (25565)
const DEFAULT_PORT: u16 = 25565;

/// Stores configuration loaded at program start
#[derive(Debug, Copy, Clone)]
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

/// Creates code to add a question for a specific record_type to a given message with a domain
macro_rules! message_question {
    ($message:expr, $domain:expr => SRV) => {
        $message.add_question(
            &format!("_minecraft._tcp.{}", $domain),
            Type::SRV,
            Class::Internet,
        )
    };
    ($message:expr, $domain:expr => $record_type:ident) => {
        $message.add_question($domain, Type::$record_type, Class::Internet);
    };
}

/// Performs a DNS request to find the specified record type, using given socket and domain
macro_rules! find_record {
    ($socket:expr, $domain:expr => $record_type:ident) => {{
        // create requests
        let mut message = Message::default();
        message_question!(message, $domain => $record_type);

        debug!("checking {} for {} record", $domain, stringify!($record_type));

        // send over socket
        let question = message.to_vec()?;
        $socket.send(&question)?;

        // read into buffer and then parse
        let mut response = [0; 512];
        let len = $socket.recv(&mut response)?;

        // now we have the answers, find the ones we care about
        let answers = Message::from_slice(&response[0..len])?.answers;
        answers.iter().find_map(|record| {
            if let Resource::$record_type(rec) = &record.resource {
                Some(rec.clone())
            } else {
                None
            }
        })
    }};
}

/// looks up ip address for a given domain and port, checking SRV, CNAME and A records (in that order)
fn domain_lookup(domain: &str, port: u16) -> Result<(IpAddr, u16)> {
    // first create a socket for dns requests
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::new(5, 0)))?;
    socket.connect("8.8.8.8:53")?; // google's dns servers

    // inner method to help with recursive search
    fn domain_lookup_inner(socket: &UdpSocket, domain: &str, port: u16) -> Result<(IpAddr, u16)> {
        // check for SRV, A and CNAME records (in that order) and use results as discovered
        let (ip, port) = if let Some(srv) = find_record!(socket, domain => SRV) {
            info!("using SRV record:\n\t{srv}");

            (srv.name, srv.port)
        } else if let Some(a) = find_record!(socket, domain => A) {
            info!("using A record:\n\t{a}");

            (a.to_string(), port)
        } else if let Some(cname) = find_record!(socket, domain => CNAME) {
            info!("using CNAME record:\n\t{cname}");

            (cname, port)
        } else {
            return Err(anyhow!("no valid records"));
        };

        // if srv record exist, check if we've reached an ip
        if let Ok(ip) = IpAddr::from_str(&ip) {
            // we've reached the end of the trail!
            Ok((ip, port))
        } else {
            info!("continuing search for {ip}");
            domain_lookup_inner(socket, &ip, port)
        }
    }

    domain_lookup_inner(&socket, domain, port)
}
