use log::{debug, warn};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS},
    NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_PREFIX, IP_ADAPTER_ADDRESSES_LH,
        IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
    },
    Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6},
};

/// Stores information about a single network adapter, analogous to a stripped-down
/// [IP_ADAPTER_ADDRESSES_LH](https://learn.microsoft.com/en-us/windows/win32/api/iptypes/ns-iptypes-ip_adapter_addresses_lh)
#[derive(Debug)]
pub(super) struct AdapterInfo {
    /// Represents how good of a choice this adapter is for ipv4
    pub(super) ipv4_metric: u32,
    /// Represents how good of a choice this adapter is for ipv6
    pub(super) ipv6_metric: u32,
    /// List of dns servers for this adapter
    pub(super) dns_servers: Vec<IpAddr>,
}

impl AdapterInfo {
    /// Reads dns servers for adapter by traversing the linked list in [IP_ADAPTER_DNS_SERVER_ADDRESS_XP]
    ///
    /// # SAFETY
    /// * ptr is checked to not be null
    /// * caller must ensure ptr is only gotten from a successful GetAdaptersAddresses call
    unsafe fn read_dns_servers(
        mut ptr: *const IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
    ) -> Option<Vec<IpAddr>> {
        let mut servers = Vec::new();

        // loop through each ptr until reach end
        while !ptr.is_null() {
            // get address of server
            let address = (*ptr).Address.lpSockaddr;

            // and then convert to IpAddr
            let dns_server = match (*address).sa_family {
                AF_INET => {
                    // if ipv4
                    let address = *(address as *mut SOCKADDR_IN);
                    let address = u32::from_be(address.sin_addr.S_un.S_addr);

                    IpAddr::from(Ipv4Addr::from(address))
                }
                AF_INET6 => {
                    // if ipv6
                    let address = *(address as *mut SOCKADDR_IN6);
                    let address = u128::from_be_bytes(address.sin6_addr.u.Byte);

                    IpAddr::from(Ipv6Addr::from(address))
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
            // then move onto next dns server in list
            ptr = (*ptr).Next;
        }

        Some(servers)
    }

    /// Fetches information for a single adapter using the information provided
    fn new(adapter: &IP_ADAPTER_ADDRESSES_LH) -> Option<Self> {
        // SAFETY: adapter is a normal rust type and thus should have been created correctly
        // meaning pointers inside are valid
        let dns_servers = unsafe {
            let dns_ptr = adapter.FirstDnsServerAddress;
            Self::read_dns_servers(dns_ptr)
        }?;

        Some(Self {
            dns_servers,
            ipv4_metric: adapter.Ipv4Metric,
            ipv6_metric: adapter.Ipv6Metric,
        })
    }
}

/// Stores information about all network adapters in a system
#[derive(Debug)]
pub(super) struct AdapterInfoList {
    pub adapters: Vec<AdapterInfo>,
}

impl AdapterInfoList {
    /// Family of IP addresses to look for (both ipv4 and ipv6)
    const FAMILY: u32 = AF_UNSPEC.0 as u32;

    /// Gets the size of the buffer needed to store adapter information
    fn get_buffer_size() -> Option<u32> {
        // make request with too short of a buffer so we can know how much size is needed
        let mut buf_len = 0;
        // SAFETY: safe because only writing to a u32, which can have any valid state
        let return_code = unsafe {
            GetAdaptersAddresses(
                Self::FAMILY,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                None,
                &mut buf_len,
            )
        };

        if return_code == ERROR_BUFFER_OVERFLOW.0 {
            Some(buf_len)
        } else {
            warn!("GetAdaptersAddresses didn't overflow despite 0 length buffer");
            None
        }
    }

    /// Finds adapter information by calling
    /// [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses)
    pub(super) fn new() -> Option<Self> {
        // create buffer, and ptr to start of buffer
        let mut buf_len = Self::get_buffer_size()?;

        let mut buf = vec![0u8; buf_len as usize];
        let mut ptr = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

        // and then make another call to GetAdaptersAddresses
        // SAFETY: we checked size of buffer to prevent buffer overrun
        let return_code = unsafe {
            GetAdaptersAddresses(
                Self::FAMILY,
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

        // now loop through each entry by traversing linked list
        let mut adapters = Vec::new();

        while !ptr.is_null() {
            // SAFETY: we ensured pointer is not null and GetAdaptersAddresses was a success,
            // so pointer *should* be valid
            let adapter = unsafe {
                let adapter = *ptr;
                debug!("reading adapter `{}`", adapter.FriendlyName.display());

                adapter
            };

            match AdapterInfo::new(&adapter) {
                Some(adapter) => {
                    debug!("adding adapter:\n\t{adapter:?}");
                    adapters.push(adapter)
                }
                _ => warn!("invalid adapter found"),
            }

            ptr = adapter.Next;
        }

        // and then finally sort by min of ipv4 and ipv6 metric (useful for choosing best dns servers)
        adapters.sort_unstable_by_key(|adapter| adapter.ipv4_metric.min(adapter.ipv6_metric));

        Some(Self { adapters })
    }

    /// Returns a vector of all dns servers for all adapters
    pub(super) fn dns_servers(&self) -> Vec<IpAddr> {
        self.adapters
            .iter()
            .flat_map(|adapter| adapter.dns_servers.iter().cloned())
            .collect()
    }
}
