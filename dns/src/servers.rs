use log::info;
use std::net::IpAddr;
use std::str::FromStr;

/// Finds dns servers in use by parsing /etc/resolv.conf
#[cfg(unix)]
fn find_servers() -> Option<Vec<IpAddr>> {
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

#[cfg(windows)]
fn find_servers() -> Option<Vec<IpAddr>> {
    info!("no supported method for getting dns servers on this platform");
    None
}

#[cfg(all(not(unix), not(windows)))]
fn find_servers() -> Option<Vec<IpAddr>> {
    info!("no supported method for getting dns servers on this platform");
    None
}

/// Finds DNS servers to use, defaulting to cloudflare if can't find servers client is using
pub fn dns_servers() -> Vec<IpAddr> {
    find_servers().unwrap_or_else(|| {
        let default_servers = vec!["1.1.1.1".parse().unwrap(), "1.0.0.1".parse().unwrap()];

        info!("using default DNS servers: {default_servers:?}");
        default_servers
    })
}
