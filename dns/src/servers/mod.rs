use cfg_if::cfg_if;
use lazy_static::lazy_static;
use log::info;
use std::net::IpAddr;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        use unix::find_servers;
    } else if #[cfg(windows)] {
        mod windows;
        use self::windows::find_servers;
    } else {
        fn find_servers() -> Option<Vec<IpAddr>> {
            info!("no supported method for getting dns servers on this platform");
            None
        }
    }
}

/// Finds DNS servers to use, defaulting to cloudflare if can't find servers client is using
fn dns_servers() -> Vec<IpAddr> {
    find_servers().unwrap_or_else(|| {
        let default_servers = vec!["1.1.1.1".parse().unwrap(), "1.0.0.1".parse().unwrap()];

        info!("using default DNS servers: {default_servers:?}");
        default_servers
    })
}

lazy_static! {
    pub static ref DNS_SERVERS: Vec<IpAddr> = dns_servers();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_servers() {
        let servers = find_servers();

        dbg!(&servers);
        assert!(servers.is_some());
    }
}
