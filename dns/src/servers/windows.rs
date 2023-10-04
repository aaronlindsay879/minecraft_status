use log::{info, warn};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_PREFIX, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
};

/// Family of IP addresses to look for (both ipv4 and ipv6)
const FAMILY: u32 = AF_UNSPEC.0 as u32;

pub(crate) fn find_servers() -> Option<Vec<IpAddr>> {
    // make request with too short of a buffer so we can know how much size is needed
    let mut buf_len = 0;
    // SAFETY: safe because only writing to a u32, which can have any valid state
    let return_code =
        unsafe { GetAdaptersAddresses(FAMILY, GAA_FLAG_INCLUDE_PREFIX, None, None, &mut buf_len) };

    if return_code != ERROR_BUFFER_OVERFLOW.0 {
        warn!("GetAdaptersAddresses didn't overflow despite 0 length buffer");
        return None;
    }

    // now we know how big to size the buffer
    let mut buf = vec![0u8; buf_len as usize];
    let mut ptr = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

    // and then make another call to GetAdaptersAddresses
    // SAFETY: we checked size of buffer to prevent buffer overrun
    let return_code = unsafe {
        GetAdaptersAddresses(
            FAMILY,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            Some(ptr),
            &mut buf_len,
        )
    };

    // ensure successful call to make sure we dont process invalid data
    if return_code != ERROR_SUCCESS.0 {
        warn!("GetAdaptersAddresses returned error `{return_code}`");
        return None;
    }

    // now can process results
    let mut servers = Vec::new();
    // loop through each device
    while !ptr.is_null() {
        // SAFETY: using functions as specified in docs
        unsafe {
            // then loop through each dns entry for that device
            let mut dns_ptr = (*ptr).FirstDnsServerAddress;

            while !dns_ptr.is_null() {
                let address = (*dns_ptr).Address.lpSockaddr;

                let dns_server = match (*address).sa_family {
                    AF_INET => {
                        // if ipv4
                        let address = *(address as *mut SOCKADDR_IN);
                        IpAddr::from(Ipv4Addr::from(address.sin_addr.S_un.S_addr.clone()))
                    }
                    AF_INET6 => {
                        // if ipv6
                        let address = *(address as *mut SOCKADDR_IN6);
                        IpAddr::from(Ipv6Addr::from(address.sin6_addr.u.Byte.clone()))
                    }
                    dns_family => {
                        // if ??
                        warn!(
                            "GetAdaptersAddresses returned a dns server with invalid family `{dns_family:?}`"
                        );
                        return None;
                    }
                };

                servers.push(dns_server);
                dns_ptr = (*dns_ptr).Next;
            }

            ptr = (*ptr).Next;
        }
    }

    if servers.len() > 0 {
        info!("using nameservers from GetAdaptersAddresses");
        Some(servers)
    } else {
        info!("no valid nameservers from GetAdaptersAddresses");
        None
    }
}
