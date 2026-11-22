//! Windows Filtering Platform default-deny layer.
//!
//! Installs one block filter per ALE connect/receive layer and a loopback
//! permit with a higher weight. Everything else stays blocked until the
//! allowlist filters are added. The real filter test runs in the VM; the
//! ignored test must never run on the host.

use std::fmt;

use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
    FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FwpmFilterDeleteByKey0, FwpmSubLayerAdd0,
    FwpmSubLayerDeleteByKey0, FwpmTransactionAbort0, FwpmTransactionBegin0, FwpmTransactionCommit0,
    FWPM_ACTION0, FWPM_ACTION0_0, FWPM_CONDITION_FLAGS, FWPM_DISPLAY_DATA0, FWPM_FILTER0,
    FWPM_FILTER_CONDITION0, FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_LAYER_ALE_AUTH_CONNECT_V6,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4, FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6, FWPM_SUBLAYER0,
    FWP_ACTION_BLOCK, FWP_ACTION_PERMIT, FWP_ACTION_TYPE, FWP_CONDITION_FLAG_IS_LOOPBACK,
    FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0, FWP_MATCH_FLAGS_ALL_SET, FWP_UINT32, FWP_UINT64,
    FWP_VALUE0, FWP_VALUE0_0,
};
use windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

/// Stable identity of the TVPN Pro sublayer.
const SUBLAYER: GUID = GUID::from_u128(0x54_56_50_4e_5f_50_52_4f_5f_57_46_50_00_00_00_01);
const FILTER_BASE: u128 = 0x5456504e5f0000000000000000000000;

const BLOCK_WEIGHT: u64 = 0x100;
const PERMIT_WEIGHT: u64 = 0x200;

const LAYERS: [GUID; 4] = [
    FWPM_LAYER_ALE_AUTH_CONNECT_V4,
    FWPM_LAYER_ALE_AUTH_CONNECT_V6,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6,
];

#[derive(Debug)]
pub struct WfpError(pub u32);

impl fmt::Display for WfpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "WFP error 0x{:08x}", self.0)
    }
}

impl std::error::Error for WfpError {}

fn filter_key(index: u128) -> GUID {
    GUID::from_u128(FILTER_BASE + index)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Owns the WFP engine handle and the installed filter set.
pub struct WfpEngine {
    handle: HANDLE,
    installed: bool,
}

impl WfpEngine {
    /// Opens a dynamic WFP session.
    pub fn open() -> Result<Self, WfpError> {
        let mut handle = HANDLE::default();
        let code =
            unsafe { FwpmEngineOpen0(PCWSTR::null(), RPC_C_AUTHN_WINNT, None, None, &mut handle) };
        if code != 0 {
            return Err(WfpError(code));
        }
        Ok(Self {
            handle,
            installed: false,
        })
    }

    /// Installs default-deny and loopback permits. Idempotent for our keys.
    pub fn arm_default_deny(&mut self) -> Result<(), WfpError> {
        self.delete_owned();

        let code = unsafe { FwpmTransactionBegin0(self.handle, 0) };
        if code != 0 {
            return Err(WfpError(code));
        }

        let result = self.install_in_transaction();
        match result {
            Ok(()) => {
                let commit = unsafe { FwpmTransactionCommit0(self.handle) };
                if commit != 0 {
                    unsafe {
                        let _ = FwpmTransactionAbort0(self.handle);
                    }
                    return Err(WfpError(commit));
                }
                self.installed = true;
                Ok(())
            }
            Err(error) => {
                unsafe {
                    let _ = FwpmTransactionAbort0(self.handle);
                }
                Err(error)
            }
        }
    }

    fn install_in_transaction(&mut self) -> Result<(), WfpError> {
        let mut sublayer_name = wide("TVPN Pro");
        let sublayer = FWPM_SUBLAYER0 {
            subLayerKey: SUBLAYER,
            displayData: FWPM_DISPLAY_DATA0 {
                name: PWSTR(sublayer_name.as_mut_ptr()),
                description: PWSTR::null(),
            },
            flags: 0,
            providerKey: std::ptr::null_mut(),
            providerData: Default::default(),
            weight: 0x100,
        };
        let code = unsafe { FwpmSubLayerAdd0(self.handle, &sublayer, None) };
        if code != 0 {
            return Err(WfpError(code));
        }

        for (index, layer) in LAYERS.iter().enumerate() {
            self.add_filter(
                *layer,
                filter_key(index as u128),
                BLOCK_WEIGHT,
                FWP_ACTION_BLOCK,
                None,
            )?;
            self.add_filter(
                *layer,
                filter_key(100 + index as u128),
                PERMIT_WEIGHT,
                FWP_ACTION_PERMIT,
                Some(FWP_CONDITION_FLAG_IS_LOOPBACK),
            )?;
        }

        Ok(())
    }

    fn add_filter(
        &self,
        layer: GUID,
        key: GUID,
        weight: u64,
        action: FWP_ACTION_TYPE,
        loopback_flags: Option<u32>,
    ) -> Result<(), WfpError> {
        let mut name = wide("TVPN Pro filter");
        let mut weight_value = weight;
        let mut conditions = Vec::new();

        if let Some(flags) = loopback_flags {
            conditions.push(FWPM_FILTER_CONDITION0 {
                fieldKey: FWPM_CONDITION_FLAGS,
                matchType: FWP_MATCH_FLAGS_ALL_SET,
                conditionValue: FWP_CONDITION_VALUE0 {
                    r#type: FWP_UINT32,
                    Anonymous: FWP_CONDITION_VALUE0_0 { uint32: flags },
                },
            });
        }

        let filter = FWPM_FILTER0 {
            filterKey: key,
            displayData: FWPM_DISPLAY_DATA0 {
                name: PWSTR(name.as_mut_ptr()),
                description: PWSTR::null(),
            },
            layerKey: layer,
            subLayerKey: SUBLAYER,
            weight: FWP_VALUE0 {
                r#type: FWP_UINT64,
                Anonymous: FWP_VALUE0_0 {
                    uint64: &mut weight_value as *mut u64,
                },
            },
            numFilterConditions: conditions.len() as u32,
            filterCondition: if conditions.is_empty() {
                std::ptr::null_mut()
            } else {
                conditions.as_mut_ptr()
            },
            action: FWPM_ACTION0 {
                r#type: action,
                Anonymous: FWPM_ACTION0_0 {
                    filterType: GUID::default(),
                },
            },
            ..Default::default()
        };

        let code = unsafe { FwpmFilterAdd0(self.handle, &filter, None, None) };
        if code != 0 {
            return Err(WfpError(code));
        }
        Ok(())
    }

    /// Removes only our filters and sublayer.
    pub fn disarm(&mut self) -> Result<(), WfpError> {
        self.delete_owned();
        self.installed = false;
        Ok(())
    }

    fn delete_owned(&self) {
        for index in 0..LAYERS.len() {
            let key = filter_key(index as u128);
            unsafe {
                let _ = FwpmFilterDeleteByKey0(self.handle, &key);
            }
            let permit = filter_key(100 + index as u128);
            unsafe {
                let _ = FwpmFilterDeleteByKey0(self.handle, &permit);
            }
        }
        unsafe {
            let _ = FwpmSubLayerDeleteByKey0(self.handle, &SUBLAYER);
        }
    }

    /// Number of block/permit filters this object claims to own.
    pub fn verify(&self) -> usize {
        if self.installed {
            LAYERS.len() * 2
        } else {
            0
        }
    }
}

impl Drop for WfpEngine {
    fn drop(&mut self) {
        unsafe {
            let _ = FwpmEngineClose0(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_nonzero() {
        assert_ne!(filter_key(0), filter_key(1));
        assert_ne!(filter_key(0), GUID::default());
        assert_ne!(SUBLAYER, GUID::default());
    }

    #[test]
    #[ignore = "changes the host firewall; run only in the test VM"]
    fn default_deny_arms_and_disarms_in_the_vm() {
        let mut engine = WfpEngine::open().expect("open WFP");
        engine.arm_default_deny().expect("arm");
        assert_eq!(engine.verify(), 8);
        engine.disarm().expect("disarm");
        assert_eq!(engine.verify(), 0);
    }
}
