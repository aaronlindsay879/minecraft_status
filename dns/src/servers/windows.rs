use std::net::IpAddr;

pub(crate) fn find_servers() -> Option<Vec<IpAddr>> {
    info!("no supported method for getting dns servers on this platform");
    None
}
