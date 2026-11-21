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

const LOCAL_SUFFIXES: [&str; 4] = [".local", ".internal", ".lan", ".home.arpa"];

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

fn validate_host(host: &str) -> Result<(), UrlError> {
    if host.is_empty() || host.contains(':') {
        return Err(UrlError::InvalidHost);
    }

    let host = host.to_ascii_lowercase();

    if host.len() > 253 {
        return Err(UrlError::InvalidHost);
    }

    if host == "localhost" || host.ends_with(".localhost") {
        return Err(UrlError::LocalHost);
    }
    if LOCAL_SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) {
        return Err(UrlError::LocalHost);
    }

    // An IPv4 literal is never a download host. `Ipv4Addr` is strict about the
    // four-octet form; the digit/dot fallback catches shorthand like 127.1.
    if host.parse::<Ipv4Addr>().is_ok()
        || (host.contains('.') && host.chars().all(|c| c.is_ascii_digit() || c == '.'))
    {
        return Err(UrlError::IpLiteral);
    }

    let labels: Vec<&str> = host.split('.').collect();
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
}
