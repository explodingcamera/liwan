use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::Infallible;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrustedHeader {
    CfConnectingIp,
    FlyClientIp,
    TrueClientIp,
    XRealIp,
    CloudfrontViewerAddress,
    XForwardedFor,
    Forwarded,
    Other(String),
}

impl TrustedHeader {
    pub const fn all() -> &'static [Self] {
        &[
            Self::CfConnectingIp,
            Self::FlyClientIp,
            Self::TrueClientIp,
            Self::XRealIp,
            Self::CloudfrontViewerAddress,
            Self::XForwardedFor,
            Self::Forwarded,
        ]
    }

    pub fn as_header_name(&self) -> &str {
        match self {
            Self::CfConnectingIp => "cf-connecting-ip",
            Self::FlyClientIp => "fly-client-ip",
            Self::TrueClientIp => "true-client-ip",
            Self::XRealIp => "x-real-ip",
            Self::CloudfrontViewerAddress => "cloudfront-viewer-address",
            Self::XForwardedFor => "x-forwarded-for",
            Self::Forwarded => "forwarded",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl FromStr for TrustedHeader {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_lowercase().replace('_', "-");

        Ok(match normalized.as_str() {
            "cf-connecting-ip" => Self::CfConnectingIp,
            "fly-client-ip" => Self::FlyClientIp,
            "true-client-ip" => Self::TrueClientIp,
            "x-real-ip" => Self::XRealIp,
            "cloudfront-viewer-address" => Self::CloudfrontViewerAddress,
            "x-forwarded-for" => Self::XForwardedFor,
            "forwarded" => Self::Forwarded,
            _ => Self::Other(normalized),
        })
    }
}

impl Serialize for TrustedHeader {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_header_name())
    }
}

impl<'de> Deserialize<'de> for TrustedHeader {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(value.parse().expect("TrustedHeader parsing is infallible"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustedProxy {
    Ip(IpAddr),
    Cidr(IpNet),
}

impl TrustedProxy {
    pub fn contains(&self, ip: IpAddr) -> bool {
        match self {
            TrustedProxy::Ip(proxy_ip) => *proxy_ip == ip,
            TrustedProxy::Cidr(net) => net.contains(&ip),
        }
    }
}

impl FromStr for TrustedProxy {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err(anyhow::anyhow!("trusted proxy cannot be empty"));
        }

        if let Ok(ip) = value.parse::<IpAddr>() {
            return Ok(TrustedProxy::Ip(ip));
        }

        if let Ok(net) = value.parse::<IpNet>() {
            return Ok(TrustedProxy::Cidr(net));
        }

        Err(anyhow::anyhow!("invalid trusted proxy: {value}"))
    }
}

impl Serialize for TrustedProxy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            TrustedProxy::Ip(ip) => serializer.serialize_str(&ip.to_string()),
            TrustedProxy::Cidr(net) => serializer.serialize_str(&net.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for TrustedProxy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse::<TrustedProxy>().map_err(serde::de::Error::custom)
    }
}

pub fn deserialize_trusted_headers<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<TrustedHeader>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum HeadersInput {
        Single(String),
        Multiple(Vec<String>),
    }

    let input = HeadersInput::deserialize(deserializer)?;
    let values = match input {
        HeadersInput::Single(value) => value.split(',').map(str::to_owned).collect::<Vec<_>>(),
        HeadersInput::Multiple(values) => values,
    };

    let mut seen = HashSet::new();
    let headers = values
        .into_iter()
        .map(|value| value.parse::<TrustedHeader>().expect("TrustedHeader parsing is infallible"))
        .filter(|header| seen.insert(header.clone()))
        .collect();

    Ok(headers)
}

pub fn deserialize_trusted_proxies<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<TrustedProxy>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ProxiesInput {
        Single(String),
        Multiple(Vec<String>),
    }

    let input = ProxiesInput::deserialize(deserializer)?;
    let values = match input {
        ProxiesInput::Single(value) => value.split(',').map(str::to_owned).collect::<Vec<_>>(),
        ProxiesInput::Multiple(values) => values,
    };

    let proxies = values
        .into_iter()
        .map(|value| value.parse::<TrustedProxy>().map_err(serde::de::Error::custom))
        .collect::<Result<Vec<_>, D::Error>>()?;

    Ok(proxies)
}

pub fn parse_header_ip(parts: &http::request::Parts, header: &TrustedHeader) -> Option<IpAddr> {
    let value = parts.headers.get(header.as_header_name())?.to_str().ok()?.trim();
    match header {
        TrustedHeader::CloudfrontViewerAddress => value.rsplit_once(':')?.0.parse().ok(),
        TrustedHeader::XForwardedFor => value.split(',').next_back()?.trim().parse().ok(),
        TrustedHeader::Forwarded => value
            .split(',')
            .next_back()?
            .split(';')
            .find_map(|p| p.trim().strip_prefix("for="))
            .map(|p| p.trim_matches('"'))
            .and_then(|p| p.parse().ok()),
        TrustedHeader::Other(_) => value.parse().ok(),
        _ => value.parse().ok(),
    }
}

pub fn should_trust_forwarded_headers(
    use_forward_headers: bool,
    peer_ip: Option<IpAddr>,
    proxies: &[TrustedProxy],
) -> bool {
    use_forward_headers
        && (proxies.is_empty() || peer_ip.is_some_and(|ip| proxies.iter().any(|proxy| proxy.contains(ip))))
}

pub fn public_ip(ip: Option<IpAddr>) -> Option<IpAddr> {
    ip.filter(is_public_ip)
}

fn is_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            !(ipv4.is_private()
                || ipv4.is_loopback()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
                || ipv4.is_multicast())
        }
        IpAddr::V6(ipv6) => {
            !(ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                || ipv6.is_unique_local()
                || ipv6.is_unicast_link_local())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_and_custom_headers() {
        let req = http::Request::builder()
            .header("x-forwarded-for", "9.9.9.9, 8.8.8.8")
            .header("Forwarded", "for=1.1.1.1;proto=https")
            .header("X-Client-IP", "8.8.4.4")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();

        assert_eq!(parse_header_ip(&parts, &TrustedHeader::XForwardedFor), Some("8.8.8.8".parse().unwrap()));
        assert_eq!(parse_header_ip(&parts, &TrustedHeader::Forwarded), Some("1.1.1.1".parse().unwrap()));
        assert_eq!(
            parse_header_ip(&parts, &TrustedHeader::Other("x-client-ip".to_string())),
            Some("8.8.4.4".parse().unwrap())
        );
    }

    #[test]
    fn trust_decision_respects_flag_and_proxy_list() {
        let trusted = vec![TrustedProxy::Ip("10.0.0.1".parse().unwrap())];

        assert!(should_trust_forwarded_headers(true, Some("10.0.0.1".parse().unwrap()), &trusted));
        assert!(!should_trust_forwarded_headers(true, Some("10.0.0.2".parse().unwrap()), &trusted));
        assert!(should_trust_forwarded_headers(true, Some("10.0.0.2".parse().unwrap()), &[]));
        assert!(!should_trust_forwarded_headers(false, Some("10.0.0.1".parse().unwrap()), &trusted));
    }

    #[test]
    fn public_ip_filters_reserved_ranges() {
        assert_eq!(public_ip(Some("8.8.8.8".parse().unwrap())), Some("8.8.8.8".parse().unwrap()));
        assert_eq!(public_ip(Some("10.0.0.1".parse().unwrap())), None);
        assert_eq!(public_ip(Some("127.0.0.1".parse().unwrap())), None);
        assert_eq!(public_ip(Some("::1".parse().unwrap())), None);
    }
}
