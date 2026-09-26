//! A framework-neutral client for the Liwan API.

#![forbid(unsafe_code)]

mod client;
mod client_ip;
mod event;
mod integrations;
mod ip_headers;
mod transport;

pub use client::{Builder, Client, Error, Worker};
pub use client_ip::ClientIpConfig;
pub use event::{Event, IntoTimestamp, RequestMetadata};
pub use ip_headers::{ClientIpHeaderSource, ClientIpProvider, TrustedProxy};
pub use transport::{Transport, TransportError};

#[cfg(feature = "reqwest")]
pub use integrations::ReqwestTransport;
#[cfg(feature = "tower")]
pub use integrations::{LiwanLayer, LiwanService, PathFilter, ResolvedClientIp};
