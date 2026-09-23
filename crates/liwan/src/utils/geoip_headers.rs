use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod geo_tests {
    use super::*;

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
}
