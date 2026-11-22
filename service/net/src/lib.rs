//! Network core: WFP firewall allowlist, Tor controller, VPN controller, recovery.
//!
//! The Tor control client is implemented and mocked in tests. The WFP and
//! Mullvad layers are still blocked by spikes S1 and S6 (docs/SPIKES.md).

pub mod tor;
