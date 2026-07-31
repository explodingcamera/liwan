use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ClientIpHeaderSource {
    Provider(ClientIpProvider),
    Header(String),
}

impl ClientIpHeaderSource {
    pub fn as_header_name(&self) -> &str {
        match self {
            Self::Provider(ClientIpProvider::Cloudflare) => "cf-connecting-ip",
            Self::Provider(ClientIpProvider::Fastly) => "fastly-client-ip",
            Self::Provider(ClientIpProvider::Fly) => "fly-client-ip",
            Self::Provider(ClientIpProvider::Cloudfront) => "cloudfront-viewer-address",
            Self::Provider(ClientIpProvider::Akamai) => "true-client-ip",
            Self::Header(value) => value,
        }
    }
}

impl FromStr for ClientIpHeaderSource {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim().to_ascii_lowercase().replace('_', "-");
        let provider = match value.as_str() {
            "akamai" => ClientIpProvider::Akamai,
            "cloudflare" => ClientIpProvider::Cloudflare,
            "cloudfront" => ClientIpProvider::Cloudfront,
            "fastly" => ClientIpProvider::Fastly,
            "fly" => ClientIpProvider::Fly,
            _ => return Ok(Self::Header(value)),
        };
        Ok(Self::Provider(provider))
    }
}

impl<'de> Deserialize<'de> for ClientIpHeaderSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(value.parse().expect("ClientIpHeaderSource parsing is infallible"))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ClientIpProvider {
    Akamai,
    Cloudflare,
    Cloudfront,
    Fastly,
    Fly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum GeoIpHeaderSource {
    Provider(GeoIpProvider),
    Mapping(GeoIpHeaderMapping),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GeoIpProvider {
    Akamai,
    Cloudflare,
    Cloudfront,
    Netlify,
    Vercel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeoIpHeaderMapping {
    pub country: String,
    pub city: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GeoIpHeaderValues {
    pub country: Option<String>,
    pub city: Option<String>,
}

pub fn parse_geoip_headers(headers: &http::HeaderMap, sources: &[GeoIpHeaderSource]) -> GeoIpHeaderValues {
    let value = |value: &str| {
        let value = value.trim();
        (!value.is_empty() && value.len() <= 255).then(|| value.to_owned())
    };
    let header = |name: &str| headers.get(name)?.to_str().ok().and_then(&value);

    let mut values = GeoIpHeaderValues::default();
    for source in sources {
        let (country, city) = match source {
            GeoIpHeaderSource::Provider(GeoIpProvider::Cloudflare) => (header("cf-ipcountry"), header("cf-ipcity")),
            GeoIpHeaderSource::Provider(GeoIpProvider::Cloudfront) => {
                (header("cloudfront-viewer-country"), header("cloudfront-viewer-city"))
            }
            GeoIpHeaderSource::Provider(GeoIpProvider::Vercel) => {
                (header("x-vercel-ip-country"), header("x-vercel-ip-city"))
            }
            GeoIpHeaderSource::Mapping(mapping) => (header(&mapping.country), header(&mapping.city)),
            GeoIpHeaderSource::Provider(GeoIpProvider::Akamai) => {
                let edgescape = headers.get("x-akamai-edgescape").and_then(|value| value.to_str().ok());
                let get = |key: &str| {
                    edgescape?
                        .split(',')
                        .filter_map(|part| part.trim().split_once('='))
                        .find_map(|(name, value)| name.eq_ignore_ascii_case(key).then_some(value))
                        .and_then(&value)
                };
                (get("country_code"), get("city"))
            }
            GeoIpHeaderSource::Provider(GeoIpProvider::Netlify) => {
                let geo = headers
                    .get("x-nf-geo")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
                let get = |pointer: &str| geo.as_ref()?.pointer(pointer)?.as_str().and_then(&value);
                (get("/country/code"), get("/city"))
            }
        };
        values.country = values.country.or(country);
        values.city = values.city.or(city);
        if values.country.is_some() && values.city.is_some() {
            break;
        }
    }
    values
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
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

pub fn parse_header_ip(parts: &http::request::Parts, source: &ClientIpHeaderSource) -> Option<IpAddr> {
    let header = source.as_header_name();
    let value = parts.headers.get(header)?.to_str().ok()?.trim();
    match header {
        "cloudfront-viewer-address" => value.rsplit_once(':')?.0.parse().ok(),
        "x-forwarded-for" => value.split(',').next_back()?.trim().parse().ok(),
        "forwarded" => value
            .split(',')
            .next_back()?
            .split(';')
            .find_map(|p| p.trim().strip_prefix("for="))
            .map(|p| p.trim_matches('"'))
            .and_then(|p| p.parse().ok()),
        _ => value.parse().ok(),
    }
}

pub fn should_trust_proxy_headers(peer_ip: Option<IpAddr>, proxies: &[TrustedProxy]) -> bool {
    proxies.is_empty() || peer_ip.is_some_and(|ip| proxies.iter().any(|proxy| proxy.contains(ip)))
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

        assert_eq!(
            parse_header_ip(&parts, &ClientIpHeaderSource::Header("x-forwarded-for".to_string())),
            Some("8.8.8.8".parse().unwrap())
        );
        assert_eq!(
            parse_header_ip(&parts, &ClientIpHeaderSource::Header("forwarded".to_string())),
            Some("1.1.1.1".parse().unwrap())
        );
        assert_eq!(
            parse_header_ip(&parts, &ClientIpHeaderSource::Header("x-client-ip".to_string())),
            Some("8.8.4.4".parse().unwrap())
        );
    }

    #[test]
    fn parse_compound_geoip_headers() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-akamai-edgescape", "georegion=246,country_code=US,city=SAN FRANCISCO".parse().unwrap());
        assert_eq!(
            parse_geoip_headers(&headers, &[GeoIpHeaderSource::Provider(GeoIpProvider::Akamai)]),
            GeoIpHeaderValues { country: Some("US".to_string()), city: Some("SAN FRANCISCO".to_string()) }
        );

        headers.insert("x-nf-geo", r#"{"city":"Berlin","country":{"code":"DE","name":"Germany"}}"#.parse().unwrap());
        assert_eq!(
            parse_geoip_headers(&headers, &[GeoIpHeaderSource::Provider(GeoIpProvider::Netlify)]),
            GeoIpHeaderValues { country: Some("DE".to_string()), city: Some("Berlin".to_string()) }
        );
    }

    #[test]
    fn trust_decision_respects_flag_and_proxy_list() {
        let trusted = vec![TrustedProxy::Ip("10.0.0.1".parse().unwrap())];

        assert!(should_trust_proxy_headers(Some("10.0.0.1".parse().unwrap()), &trusted));
        assert!(!should_trust_proxy_headers(Some("10.0.0.2".parse().unwrap()), &trusted));
        assert!(should_trust_proxy_headers(Some("10.0.0.2".parse().unwrap()), &[]));
    }
}
