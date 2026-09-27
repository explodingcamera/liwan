use liwan::config::Config;

#[test]
fn validates_base_url_path() {
    let config = Config::load(None, [("LIWAN_BASE_URL", "https://example.com/liwan")]).unwrap();
    assert_eq!(config.base_path(), "/liwan");
    assert_eq!(config.path("/login"), "/liwan/login");
    let mut setup = config.public_url("/setup").unwrap();
    setup.query_pairs_mut().append_pair("t", "a+b/c");
    assert_eq!(setup.as_str(), "https://example.com/liwan/setup?t=a%2Bb%2Fc");
    let uppercase = Config::load(None, [("LIWAN_BASE_URL", "HTTPS://example.com/liwan")]).unwrap();
    assert!(uppercase.secure());
    assert_eq!(Config::default().base_path(), "");
    assert_eq!(Config::default().path("/login"), "/login");
    for url in [
        "https://example.com/liwan/",
        "https://example.com/liwan?x=1",
        "https://example.com/liwan#fragment",
        "https://example.com/liwan/../other",
        "https://example.com//liwan",
    ] {
        assert!(Config::load(None, [("LIWAN_BASE_URL", url)]).is_err(), "{url}");
    }
}
