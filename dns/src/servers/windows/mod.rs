mod adapter_info;

use adapter_info::AdapterInfoList;
use log::info;
use std::net::IpAddr;

pub fn find_servers() -> Option<Vec<IpAddr>> {
    let servers = AdapterInfoList::new()?.dns_servers();

    if servers.len() > 0 {
        info!("using nameservers from GetAdaptersAddresses");
        Some(servers)
    } else {
        info!("no valid nameservers from GetAdaptersAddresses");
        None
    }
}
