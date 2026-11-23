//! URL rules from `contracts/ipc-v0.md` section 7.
//!
//! Deliberately hand-written: one dependency less, and the rules stay readable.
//! Only what the contract allows gets through.

use std::fmt;
use std::net::Ipv4Addr;

/// Why a URL was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError {
    Empty,
    TooLong,
    NotHttps,
    UserInfo,
    PortNotAllowed,
    IpLiteral,
    LocalHost,
    InvalidHost,
    NotMagnet,
    InvalidMagnet,
}

impl UrlError {
    /// Wire code from `contracts/ipc-v0.md` section 9.
    pub fn code(self) -> &'static str {
        "E_DOWNLOAD_URL_INVALID"
    }

    /// Short machine detail, never the URL itself.
    pub fn detail(self) -> &'static str {
        match self {
            UrlError::Empty => "empty",
            UrlError::TooLong => "too_long",
            UrlError::NotHttps => "not_https",
            UrlError::UserInfo => "userinfo",
            UrlError::PortNotAllowed => "port",
            UrlError::IpLiteral => "ip_literal",
            UrlError::LocalHost => "local_host",
            UrlError::InvalidHost => "host",
            UrlError::NotMagnet => "magnet",
            UrlError::InvalidMagnet => "magnet_hash",
        }
    }
}

impl fmt::Display for UrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail())
    }
}

impl std::error::Error for UrlError {}

/// Longest accepted URL, in bytes.
pub const MAX_URL_BYTES: usize = 8192;

const LOCAL_SUFFIXES: [&str; 5] = [".local", ".internal", ".lan", ".home.arpa", ".localdomain"];

/// Validates an HTTPS URL against the contract.
pub fn validate(input: &str) -> Result<(), UrlError> {
    if input.is_empty() {
        return Err(UrlError::Empty);
    }
    if input.len() > MAX_URL_BYTES {
        return Err(UrlError::TooLong);
    }
    if !input
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        return Err(UrlError::NotHttps);
    }

    let rest = &input[8..];
    if rest.is_empty() {
        return Err(UrlError::InvalidHost);
    }

    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();

    if authority.contains('@') {
        return Err(UrlError::UserInfo);
    }
    if authority.contains('[') || authority.contains(']') {
        return Err(UrlError::IpLiteral);
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };

    if let Some(port) = port {
        if port.is_empty() || port != "443" {
            return Err(UrlError::PortNotAllowed);
        }
    }

    validate_host(host)
}

/// Validates a magnet link for the VPN-only bulk route.
///
/// Only `xt=urn:btih:` is read; every other parameter is ignored. BitTorrent v2
/// (`btmh`) is not accepted.
pub fn validate_magnet(input: &str) -> Result<(), UrlError> {
    if input.is_empty() {
        return Err(UrlError::NotMagnet);
    }
    if input.len() > MAX_URL_BYTES {
        return Err(UrlError::TooLong);
    }

    let lower = input.to_ascii_lowercase();
    let Some(query) = lower.strip_prefix("magnet:?") else {
        return Err(UrlError::NotMagnet);
    };

    let hash = query
        .split('&')
        .find_map(|part| part.strip_prefix("xt=urn:btih:"))
        .ok_or(UrlError::InvalidMagnet)?;

    let hex = hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit());
    let base32 = hash.len() == 32 && hash.chars().all(|c| matches!(c, 'a'..='z' | '2'..='7'));

    if hex || base32 {
        Ok(())
    } else {
        Err(UrlError::InvalidMagnet)
    }
}

fn validate_host(host: &str) -> Result<(), UrlError> {
    if host.is_empty() || host.contains(':') {
        return Err(UrlError::InvalidHost);
    }

    let host = host.to_ascii_lowercase();

    if host.len() > 253 {
        return Err(UrlError::InvalidHost);
    }

    if host == "localhost" || host.ends_with(".localhost") || host == "localdomain" {
        return Err(UrlError::LocalHost);
    }
    if LOCAL_SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) {
        return Err(UrlError::LocalHost);
    }

    let labels: Vec<&str> = host.split('.').collect();

    // An IPv4 literal is never a download host. `Ipv4Addr` is strict about the
    // four-octet form; the numeric-label rule also catches the shorthand,
    // octal and 0x-hex forms that the HTTP client would normalise to an
    // address (`0x7f.0.0.1` becomes 127.0.0.1).
    if host.parse::<Ipv4Addr>().is_ok() || labels.iter().all(|label| is_numeric_label(label)) {
        return Err(UrlError::IpLiteral);
    }

    if labels.len() < 2 {
        return Err(UrlError::InvalidHost);
    }
    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return Err(UrlError::InvalidHost);
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(UrlError::InvalidHost);
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(UrlError::InvalidHost);
        }
    }

    Ok(())
}

/// A label that only encodes a number: decimal, octal or `0x`-hex. Every
/// label of such a host is a number, so the whole host is an address.
fn is_numeric_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    if let Some(hex) = label.strip_prefix("0x") {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    label.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_https() {
        assert_eq!(validate("https://example.org"), Ok(()));
        assert_eq!(
            validate("https://example.org/path?query=1#fragment"),
            Ok(())
        );
        assert_eq!(validate("https://example.org:443/file"), Ok(()));
        assert_eq!(validate("HTTPS://EXAMPLE.ORG/file"), Ok(()));
        assert_eq!(validate("https://sub.domain.example.org/x"), Ok(()));
        assert_eq!(validate("https://abcdefghij.onion/file"), Ok(()));
    }

    #[test]
    fn rejects_other_schemes() {
        assert_eq!(validate("http://example.org"), Err(UrlError::NotHttps));
        assert_eq!(validate("ftp://example.org"), Err(UrlError::NotHttps));
        assert_eq!(validate("example.org"), Err(UrlError::NotHttps));
        assert_eq!(validate(""), Err(UrlError::Empty));
    }

    #[test]
    fn rejects_other_ports() {
        assert_eq!(
            validate("https://example.org:8443/x"),
            Err(UrlError::PortNotAllowed)
        );
        assert_eq!(
            validate("https://example.org:/x"),
            Err(UrlError::PortNotAllowed)
        );
    }

    #[test]
    fn rejects_userinfo() {
        assert_eq!(
            validate("https://user:secret@example.org/x"),
            Err(UrlError::UserInfo)
        );
        assert_eq!(
            validate("https://user@example.org/x"),
            Err(UrlError::UserInfo)
        );
    }

    #[test]
    fn rejects_ip_literals() {
        assert_eq!(validate("https://127.0.0.1/x"), Err(UrlError::IpLiteral));
        assert_eq!(validate("https://203.0.113.7/x"), Err(UrlError::IpLiteral));
        assert_eq!(validate("https://127.1/x"), Err(UrlError::IpLiteral));
        assert_eq!(validate("https://[::1]/x"), Err(UrlError::IpLiteral));
        assert_eq!(
            validate("https://[2001:db8::1]/x"),
            Err(UrlError::IpLiteral)
        );
    }

    #[test]
    fn rejects_hex_and_octal_ip_notation() {
        // The HTTP client normalises these to 127.0.0.1; the validator sees a
        // hostname and would let them through without this rule.
        assert_eq!(validate("https://0x7f.0.0.0.1/"), Err(UrlError::IpLiteral));
        assert_eq!(
            validate("https://0x7f.0x0.0x0.0x1/"),
            Err(UrlError::IpLiteral)
        );
        assert_eq!(validate("https://0177.0.0.1/"), Err(UrlError::IpLiteral));
        assert_eq!(
            validate("https://0x7f.0.0.1/file"),
            Err(UrlError::IpLiteral)
        );
    }

    #[test]
    fn rejects_compressed_ipv4_boundaries() {
        // Both normalise to 127.0.0.1 in the HTTP client.
        assert_eq!(validate("https://2130706433/"), Err(UrlError::IpLiteral));
        assert_eq!(validate("https://0x7f000001/"), Err(UrlError::IpLiteral));
        // A numeric label next to real labels is a hostname, not an address.
        assert_eq!(validate("https://1.example.org/"), Ok(()));
    }

    #[test]
    fn rejects_localdomain_and_localhost_subdomains() {
        assert_eq!(
            validate("https://localhost.localdomain/"),
            Err(UrlError::LocalHost)
        );
        assert_eq!(
            validate("https://box.localdomain/x"),
            Err(UrlError::LocalHost)
        );
        assert_eq!(
            validate("https://api.localhost/x"),
            Err(UrlError::LocalHost)
        );
        assert_eq!(validate("https://localdomain/x"), Err(UrlError::LocalHost));
    }

    #[test]
    fn rejects_local_names() {
        assert_eq!(validate("https://localhost/x"), Err(UrlError::LocalHost));
        assert_eq!(
            validate("https://printer.local/x"),
            Err(UrlError::LocalHost)
        );
        assert_eq!(validate("https://nas.internal/x"), Err(UrlError::LocalHost));
        assert_eq!(validate("https://router.lan/x"), Err(UrlError::LocalHost));
        assert_eq!(
            validate("https://box.home.arpa/x"),
            Err(UrlError::LocalHost)
        );
    }

    #[test]
    fn rejects_malformed_hosts() {
        assert_eq!(validate("https://intranet/x"), Err(UrlError::InvalidHost));
        assert_eq!(validate("https:///x"), Err(UrlError::InvalidHost));
        assert_eq!(
            validate("https://exa mple.org/x"),
            Err(UrlError::InvalidHost)
        );
        assert_eq!(
            validate("https://-bad.example.org/x"),
            Err(UrlError::InvalidHost)
        );
        assert_eq!(
            validate("https://bad-.example.org/x"),
            Err(UrlError::InvalidHost)
        );
        assert_eq!(
            validate("https://example.org./x"),
            Err(UrlError::InvalidHost)
        );
        assert_eq!(
            validate("https://exa_mple.org/x"),
            Err(UrlError::InvalidHost)
        );
    }

    #[test]
    fn rejects_overlong_urls() {
        let long = format!("https://example.org/{}", "a".repeat(MAX_URL_BYTES));
        assert_eq!(validate(&long), Err(UrlError::TooLong));
    }

    #[test]
    fn errors_carry_machine_details() {
        assert_eq!(UrlError::LocalHost.detail(), "local_host");
        assert_eq!(UrlError::NotHttps.code(), "E_DOWNLOAD_URL_INVALID");
    }

    #[test]
    fn accepts_valid_magnets() {
        assert_eq!(
            validate_magnet("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"),
            Ok(())
        );
        assert_eq!(
            validate_magnet("magnet:?dn=image&xt=urn:btih:abcdefghijklmnopqrstuvwxyz234567"),
            Ok(())
        );
    }

    #[test]
    fn rejects_bad_magnets() {
        assert_eq!(
            validate_magnet("https://example.org/x"),
            Err(UrlError::NotMagnet)
        );
        assert_eq!(validate_magnet(""), Err(UrlError::NotMagnet));
        assert_eq!(
            validate_magnet("magnet:?dn=name"),
            Err(UrlError::InvalidMagnet)
        );
        assert_eq!(
            validate_magnet("magnet:?xt=urn:btih:xyz"),
            Err(UrlError::InvalidMagnet)
        );
        assert_eq!(
            validate_magnet("magnet:?xt=urn:btih:01890189018901890189018901890189"),
            Err(UrlError::InvalidMagnet)
        );
    }
}
