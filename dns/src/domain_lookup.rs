use anyhow::{anyhow, Result};
use log::{debug, info};
use rustdns::{Class, Message, Resource, Type};
use std::net::{IpAddr, UdpSocket};
use std::str::FromStr;
use std::time::Duration;

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
pub fn domain_lookup(domain: &str, port: u16) -> Result<(IpAddr, u16)> {
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

        // if record exists, check if we've reached an ip
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
