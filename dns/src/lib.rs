#![deny(unsafe_code)]

mod domain_lookup;
mod servers;

pub use domain_lookup::domain_lookup;
use servers::DNS_SERVERS;
