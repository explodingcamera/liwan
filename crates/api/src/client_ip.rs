use std::net::IpAddr;

use http::HeaderMap;

use crate::ip_headers::{ClientIpHeaderSource, TrustedProxy, parse_client_ip, should_trust_proxy_headers};

/// Resolves client addresses while preventing untrusted forwarded-header spoofing.
#[derive(Debug, Clone, Default)]
pub struct ClientIpConfig {
    sources: Vec<ClientIpHeaderSource>,
    trusted_proxies: Vec<TrustedProxy>,
}

impl ClientIpConfig {
    /// Creates a configuration that uses the direct peer address.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a header source in precedence order.
    pub fn source(mut self, source: ClientIpHeaderSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Adds an IP address or network that is allowed to relay client addresses.
    pub fn trusted_proxy(mut self, proxy: impl Into<TrustedProxy>) -> Self {
        self.trusted_proxies.push(proxy.into());
        self
    }

    /// Resolves the client address from request headers and the direct peer address.
    pub fn resolve(&self, headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
        if should_trust_proxy_headers(peer, &self.trusted_proxies) {
            for source in &self.sources {
                if let Some(address) = parse_client_ip(headers, source, peer, &self.trusted_proxies) {
                    return Some(address);
                }
            }
        }
        peer
    }
}
