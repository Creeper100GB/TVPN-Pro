//! MAC address generation and validation.
//!
//! Never persist or log the full address. The caller may show it only in the
//! local UI if it wants to; the service keeps it in memory for the adapter
//! restore path.

use std::fmt;

use getrandom::getrandom;

/// A 48-bit Ethernet address.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Random, locally administered, unicast.
    pub fn random() -> Self {
        let mut bytes = [0u8; 6];
        getrandom(&mut bytes).expect("CSPRNG must be available");
        // Bit 0 is the multicast bit, bit 1 is the local bit.
        bytes[0] = (bytes[0] & 0xFC) | 0x02;
        Self(bytes)
    }

    pub fn as_bytes(&self) -> [u8; 6] {
        self.0
    }

    pub fn is_locally_administered(&self) -> bool {
        self.0[0] & 0x02 != 0
    }

    pub fn is_unicast(&self) -> bool {
        self.0[0] & 0x01 == 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Keep accidental debug logging useless.
        formatter.write_str("MacAddress(redacted)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_is_local_and_unicast() {
        for _ in 0..32 {
            let mac = MacAddress::random();
            assert!(mac.is_locally_administered(), "{mac}");
            assert!(mac.is_unicast(), "{mac}");
        }
    }

    #[test]
    fn formatting_is_stable() {
        let mac = MacAddress::new([0x02, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert_eq!(mac.to_string(), "02:11:22:33:44:55");
        assert_eq!(format!("{mac:?}"), "MacAddress(redacted)");
    }

    #[test]
    fn bits_are_checked() {
        assert!(MacAddress::new([0x02, 0, 0, 0, 0, 0]).is_locally_administered());
        assert!(!MacAddress::new([0x00, 0, 0, 0, 0, 0]).is_locally_administered());
        assert!(!MacAddress::new([0x03, 0, 0, 0, 0, 0]).is_unicast());
    }
}
