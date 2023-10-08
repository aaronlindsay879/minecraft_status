use cfg_if::cfg_if;
use lazy_static::lazy_static;
use log::debug;
use std::net::IpAddr;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        use unix::find_servers;
    } else if #[cfg(windows)] {
        #[allow(unsafe_code)]
        mod windows;

        use self::windows::find_servers;
    } else {
        fn find_servers() -> Option<Vec<IpAddr>> {
            info!("no supported method for getting dns servers on this platform");
            None
        }
    }
}

lazy_static! {
    pub static ref DNS_SERVERS: Vec<IpAddr> = {
        let servers = find_servers().unwrap_or_default();
        debug!("using dns servers:\n{servers:#?}");

        servers
    };
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
