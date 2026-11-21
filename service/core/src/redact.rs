//! Log redaction scanner.
//!
//! Mirrors the CI gate in `tests/log-redaction/check-log-redaction.ps1`:
//! IPv4, IPv6, MAC and URLs are leaks, loopback addresses are fine.
//!
//! The scanner returns categories and line numbers only. It never returns the
//! matched text, so a failure report cannot spread the leaked value.

use std::net::{Ipv4Addr, Ipv6Addr};

/// What kind of observable was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakKind {
    Ipv4,
    Ipv6,
    Mac,
    Url,
}

impl LeakKind {
    /// Stable name for reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Mac => "mac",
            Self::Url => "url",
        }
    }
}

/// One finding, without the offending value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding {
    pub line: usize,
    pub kind: LeakKind,
}

/// Scans a log text and reports every finding in line order.
pub fn scan(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (index, line) in text.lines().enumerate() {
        for raw in line.split_whitespace() {
            let token = raw.trim_matches(|c: char| {
                matches!(
                    c,
                    ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | '<' | '>' | '='
                )
            });
            // Split key=value pairs so "peer=203.0.113.7" is seen as the address.
            let token = token.rsplit_once('=').map_or(token, |(_, value)| value);
            if token.is_empty() {
                continue;
            }
            if let Some(kind) = classify(token) {
                findings.push(Finding {
                    line: index + 1,
                    kind,
                });
            }
        }
    }

    findings
}

/// True when the text contains any leak.
pub fn contains_leak(text: &str) -> bool {
    !scan(text).is_empty()
}

fn classify(token: &str) -> Option<LeakKind> {
    if token.contains("://") {
        return if url_is_loopback(token) {
            None
        } else {
            Some(LeakKind::Url)
        };
    }

    if is_mac(token) {
        return Some(LeakKind::Mac);
    }

    if let Some(address) = ipv4_candidate(token) {
        return if address.is_loopback() || address.is_unspecified() {
            None
        } else {
            Some(LeakKind::Ipv4)
        };
    }

    if parse_ipv6(token).is_some_and(|address| !is_allowed_v6(address)) {
        return Some(LeakKind::Ipv6);
    }

    None
}

fn ipv4_candidate(token: &str) -> Option<Ipv4Addr> {
    // "203.0.113.7:443" and "203.0.113.7." are still the same address.
    let trimmed = token.trim_end_matches('.');
    let candidate = match trimmed.rsplit_once(':') {
        Some((host, port)) if host.contains('.') && port.chars().all(|c| c.is_ascii_digit()) => {
            host
        }
        _ => trimmed,
    };

    candidate.parse().ok()
}

fn parse_ipv6(token: &str) -> Option<Ipv6Addr> {
    if !token.contains(':') {
        return None;
    }
    let without_brackets = token
        .strip_prefix('[')
        .and_then(|rest| rest.split(']').next())
        .unwrap_or(token);
    let without_zone = without_brackets
        .split('%')
        .next()
        .unwrap_or(without_brackets);
    without_zone.parse().ok()
}

fn is_allowed_v6(address: Ipv6Addr) -> bool {
    address.is_loopback() || address.is_unspecified()
}

fn is_mac(token: &str) -> bool {
    let separator = if token.contains(':') {
        ':'
    } else if token.contains('-') {
        '-'
    } else {
        return false;
    };

    let parts: Vec<&str> = token.split(separator).collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
}

fn url_is_loopback(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    let rest = lower.split_once("://").map_or("", |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map_or(authority, |(host, _)| host);
    let host = host.trim_matches(['[', ']']);

    if host == "localhost" {
        return true;
    }
    host.parse::<Ipv4Addr>()
        .is_ok_and(|address| address.is_loopback())
        || host
            .parse::<Ipv6Addr>()
            .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<&'static str> {
        scan(text)
            .into_iter()
            .map(|finding| finding.kind.as_str())
            .collect()
    }

    #[test]
    fn detects_every_category() {
        let text = "\
2026-11-21T10:00:00Z INFO state_changed from=OFF to=PREFLIGHT
2026-11-21T10:00:01Z WARN peer=203.0.113.7 port=443
2026-11-21T10:00:02Z DEBUG mac=3C:7A:8F:1D:22:B4
2026-11-21T10:00:03Z DEBUG url=https://example.org/files/report.pdf
2026-11-21T10:00:04Z DEBUG host=2001:db8::1";
        assert_eq!(kinds(text), ["ipv4", "mac", "url", "ipv6"]);
    }

    #[test]
    fn allows_loopback() {
        let text = "\
addr=127.0.0.1:9150 scheme=socks5h
addr=[::1]:9150
bind=0.0.0.0:9050
url=https://127.0.0.1:8080/health";
        assert!(scan(text).is_empty(), "findings: {:?}", scan(text));
    }

    #[test]
    fn ignores_harmless_numbers_and_words() {
        let text = "\
time=12:30:45 percent=100 count=3 build=1.100.0 hash_prefix=9f2c1a state=READY";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn reports_line_numbers() {
        let findings = scan("clean\npeer=203.0.113.7\nclean");
        assert_eq!(
            findings,
            [Finding {
                line: 2,
                kind: LeakKind::Ipv4
            }]
        );
    }

    #[test]
    fn accepts_addresses_with_port_and_trailing_dot() {
        assert_eq!(kinds("peer=203.0.113.7:443"), ["ipv4"]);
        assert_eq!(kinds("peer=203.0.113.7."), ["ipv4"]);
        assert_eq!(kinds("peer=[2001:db8::1]:443"), ["ipv6"]);
    }

    #[test]
    fn detects_mac_with_dashes_and_uppercase() {
        assert_eq!(kinds("mac=3c-7a-8f-1d-22-b4"), ["mac"]);
    }

    #[test]
    fn does_not_treat_version_strings_as_ipv4() {
        assert!(scan("version=1.100.0").is_empty());
    }
}
