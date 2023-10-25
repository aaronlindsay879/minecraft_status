use log::info;
use std::{net::IpAddr, str::FromStr};

/// Finds dns server in use by parsing /etc/resolv.conf
pub(crate) fn find_servers() -> Option<Vec<IpAddr>> {
    let resolv_conf = std::fs::read_to_string("/etc/resolv.conf").ok()?;

    // parse resolv.conf, taking any lines that start with "nameserver " and attempting to parse the ip
    let servers: Vec<_> = resolv_conf
        .lines()
        .filter_map(|line| line.strip_prefix("nameserver "))
        .filter_map(|line| IpAddr::from_str(line).ok())
        .collect();

    if servers.len() > 0 {
        info!("using nameservers from /etc/resolv.conf");
        Some(servers)
    } else {
        info!("no valid nameservers in /etc/resolv.conf");
        None
    }
}
