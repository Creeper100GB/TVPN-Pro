//! Privacy shield primitives: locally administered MAC addresses and the
//! hostname plan. The actual adapter write is Windows-only and driver
//! dependent; the VM's synthetic adapter reports it as unsupported.

pub mod hostname;
pub mod mac;
