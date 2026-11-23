//! Windows Filtering Platform default-deny layer.
//!
//! Installs one block filter per ALE connect/receive layer, a loopback permit
//! with a higher weight, and per-program permits for the components that must
//! reach the network after arming (`tor.exe`, the Mullvad daemon). Everything
//! else stays blocked until the service disarms or recovers.
//!
//! The real filter test runs in the VM; the ignored test must never run on the
//! host.

use std::ffi::c_void;
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
    FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FwpmFilterDeleteByKey0, FwpmFilterGetByKey0,
    FwpmFreeMemory0, FwpmGetAppIdFromFileName0, FwpmSubLayerAdd0, FwpmSubLayerDeleteByKey0,
    FwpmTransactionAbort0, FwpmTransactionBegin0, FwpmTransactionCommit0, FWPM_ACTION0,
    FWPM_ACTION0_0, FWPM_CONDITION_ALE_APP_ID, FWPM_CONDITION_FLAGS, FWPM_DISPLAY_DATA0,
    FWPM_FILTER0, FWPM_FILTER_CONDITION0, FWPM_LAYER_ALE_AUTH_CONNECT_V4,
    FWPM_LAYER_ALE_AUTH_CONNECT_V6, FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6, FWPM_SESSION0, FWPM_SUBLAYER0, FWP_ACTION_BLOCK,
    FWP_ACTION_PERMIT, FWP_ACTION_TYPE, FWP_BYTE_BLOB, FWP_BYTE_BLOB_TYPE,
    FWP_CONDITION_FLAG_IS_LOOPBACK, FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0, FWP_MATCH_EQUAL,
    FWP_MATCH_FLAGS_ALL_SET, FWP_UINT32, FWP_UINT64, FWP_VALUE0, FWP_VALUE0_0,
};
use windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

/// Stable identity of the TVPN Pro sublayer.
const SUBLAYER: GUID = GUID::from_u128(0x54_56_50_4e_5f_50_52_4f_5f_57_46_50_00_00_00_01);
const FILTER_BASE: u128 = 0x5456504e5f0000000000000000000000;

const BLOCK_WEIGHT: u64 = 0x100;
const PERMIT_WEIGHT: u64 = 0x200;
const APP_WEIGHT: u64 = 0x300;

/// Four components at most: tor, mullvad, the fallback browser and the
/// torrent client. Each gets one permit filter per layer.
const MAX_PERMITTED_APPS: usize = 4;

const LAYERS: [GUID; 4] = [
    FWPM_LAYER_ALE_AUTH_CONNECT_V4,
    FWPM_LAYER_ALE_AUTH_CONNECT_V6,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4,
    FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6,
];

fn block_key(layer: usize) -> GUID {
    filter_key(layer as u128)
}

fn loopback_key(layer: usize) -> GUID {
    filter_key(100 + layer as u128)
}

fn app_key(app: usize, layer: usize) -> GUID {
    filter_key(200 + (app * LAYERS.len() + layer) as u128)
}

fn filter_key(index: u128) -> GUID {
    GUID::from_u128(FILTER_BASE + index)
}

/// Filters that `arm_default_deny` installs: one block and one loopback permit
/// per ALE layer.
const BASE_FILTERS: usize = LAYERS.len() * 2;

/// Synthetic error code for "our filters are still present after a disarm".
/// WFP has no matching HRESULT, and the caller only needs the failure.
const ERR_LEFTOVER_FILTERS: u32 = 0x8007_0bad;

/// Expected number of our filters for `permitted` allowed programs.
///
/// The leak check compares the live filter count against this value instead of
/// a bare minimum: a single missing block filter must not pass.
pub fn filter_count_for(permitted: usize) -> usize {
    BASE_FILTERS + permitted * LAYERS.len()
}

/// True when the live filter set covers every filter we expect.
///
/// A partial set (permits without the block filters, or half of the ALE layers)
/// leaves traffic unblocked, so it must not count as armed. `expected == 0`
/// means we cannot say what a complete set is, and that is not armed either.
pub fn filters_are_complete(filters: usize, expected: usize) -> bool {
    expected > 0 && filters >= expected
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Debug)]
pub struct WfpError(pub u32);

impl fmt::Display for WfpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "WFP error 0x{:08x}", self.0)
    }
}

impl std::error::Error for WfpError {}

/// Owns the WFP engine handle and the installed filter set.
pub struct WfpEngine {
    handle: HANDLE,
    permitted: usize,
}

impl WfpEngine {
    /// Opens a static WFP session. Filters survive a hard kill; a clean
    /// shutdown disarms them explicitly, and the next start uses `verify` to
    /// detect leftovers and enters RECOVERY.
    pub fn open() -> Result<Self, WfpError> {
        let mut handle = HANDLE::default();
        let session = FWPM_SESSION0 {
            flags: 0,
            ..Default::default()
        };
        let code = unsafe {
            FwpmEngineOpen0(
                PCWSTR::null(),
                RPC_C_AUTHN_WINNT,
                None,
                Some(&session),
                &mut handle,
            )
        };
        if code != 0 {
            return Err(WfpError(code));
        }
        Ok(Self {
            handle,
            permitted: 0,
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
                self.permitted = 0;
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

        // Loopback is always allowed. Without it PowerShell Direct, the pipe
        // and local UI calls would be cut off as well.
        let loopback = FWPM_FILTER_CONDITION0 {
            fieldKey: FWPM_CONDITION_FLAGS,
            matchType: FWP_MATCH_FLAGS_ALL_SET,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_UINT32,
                Anonymous: FWP_CONDITION_VALUE0_0 {
                    uint32: FWP_CONDITION_FLAG_IS_LOOPBACK,
                },
            },
        };

        for (index, layer) in LAYERS.iter().enumerate() {
            self.add_filter(
                *layer,
                block_key(index),
                BLOCK_WEIGHT,
                FWP_ACTION_BLOCK,
                &[],
            )?;
            self.add_filter(
                *layer,
                loopback_key(index),
                PERMIT_WEIGHT,
                FWP_ACTION_PERMIT,
                std::slice::from_ref(&loopback),
            )?;
        }

        Ok(())
    }

    /// Permits one program on all four ALE layers.
    ///
    /// This is a program allowlist, not an interface allowlist: the interface
    /// condition (S1) is still open. The caller must therefore only add the
    /// Tor binary after the VPN is up, and the UI must not claim that the
    /// allowlist is interface-bound.
    pub fn permit_app(&mut self, path: &Path) -> Result<(), WfpError> {
        if self.permitted >= MAX_PERMITTED_APPS {
            return Err(WfpError(87)); // ERROR_INVALID_PARAMETER
        }

        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut blob: *mut FWP_BYTE_BLOB = std::ptr::null_mut();
        let code = unsafe { FwpmGetAppIdFromFileName0(PCWSTR(wide_path.as_ptr()), &mut blob) };
        if code != 0 {
            return Err(WfpError(code));
        }

        let result = (|| {
            let condition = FWPM_FILTER_CONDITION0 {
                fieldKey: FWPM_CONDITION_ALE_APP_ID,
                matchType: FWP_MATCH_EQUAL,
                conditionValue: FWP_CONDITION_VALUE0 {
                    r#type: FWP_BYTE_BLOB_TYPE,
                    Anonymous: FWP_CONDITION_VALUE0_0 { byteBlob: blob },
                },
            };

            for (index, layer) in LAYERS.iter().enumerate() {
                self.add_filter(
                    *layer,
                    app_key(self.permitted, index),
                    APP_WEIGHT,
                    FWP_ACTION_PERMIT,
                    std::slice::from_ref(&condition),
                )?;
            }
            Ok(())
        })();

        let mut raw = blob as *mut c_void;
        unsafe {
            FwpmFreeMemory0(&mut raw);
        }

        if result.is_ok() {
            self.permitted += 1;
        }
        result
    }

    fn add_filter(
        &self,
        layer: GUID,
        key: GUID,
        weight: u64,
        action: FWP_ACTION_TYPE,
        conditions: &[FWPM_FILTER_CONDITION0],
    ) -> Result<(), WfpError> {
        let mut name = wide("TVPN Pro filter");
        let mut weight_value = weight;

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
                conditions.as_ptr() as *mut FWPM_FILTER_CONDITION0
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

    /// Removes only our filters and sublayer. Fails when anything is left, so
    /// a failed delete cannot be reported as a clean teardown.
    pub fn disarm(&mut self) -> Result<(), WfpError> {
        self.delete_owned();
        self.permitted = 0;
        let leftovers = self.verify();
        if leftovers > 0 {
            return Err(WfpError(ERR_LEFTOVER_FILTERS));
        }
        Ok(())
    }

    /// Expected number of filters for the currently permitted programs.
    pub fn expected_filter_count(&self) -> usize {
        filter_count_for(self.permitted)
    }

    fn delete_owned(&self) {
        for index in 0..LAYERS.len() {
            for key in [block_key(index), loopback_key(index)] {
                unsafe {
                    let _ = FwpmFilterDeleteByKey0(self.handle, &key);
                }
            }
        }
        for app in 0..MAX_PERMITTED_APPS {
            for index in 0..LAYERS.len() {
                let key = app_key(app, index);
                unsafe {
                    let _ = FwpmFilterDeleteByKey0(self.handle, &key);
                }
            }
        }
        unsafe {
            let _ = FwpmSubLayerDeleteByKey0(self.handle, &SUBLAYER);
        }
    }

    /// Number of our filters that are actually present.
    ///
    /// This reads the state back from BFE instead of trusting `installed`, so
    /// a failed or skipped teardown is visible. The service uses this to decide
    /// between `TEARDOWN -> OFF` and `TEARDOWN -> RECOVERY`.
    pub fn verify(&self) -> usize {
        let mut present = 0;

        for index in 0..LAYERS.len() {
            for key in [block_key(index), loopback_key(index)] {
                if self.filter_exists(&key) {
                    present += 1;
                }
            }
        }
        for app in 0..MAX_PERMITTED_APPS {
            for index in 0..LAYERS.len() {
                if self.filter_exists(&app_key(app, index)) {
                    present += 1;
                }
            }
        }

        present
    }

    /// True when any filter with our fixed keys is still installed.
    pub fn has_own_filters(&self) -> bool {
        self.verify() > 0
    }

    fn filter_exists(&self, key: &GUID) -> bool {
        let mut filter: *mut FWPM_FILTER0 = std::ptr::null_mut();
        let code = unsafe { FwpmFilterGetByKey0(self.handle, key, &mut filter) };
        if code != 0 {
            return false;
        }
        if !filter.is_null() {
            let mut pointer = filter.cast::<c_void>();
            unsafe {
                FwpmFreeMemory0(&mut pointer);
            }
        }
        true
    }
}

impl Drop for WfpEngine {
    fn drop(&mut self) {
        // Never disarm on drop. A service stop, a crash or a clean process exit
        // is not a user-requested disable; the filters must stay in BFE until
        // an explicit `disarm()` or `recover` removes them. The next start sees
        // them again and enters RECOVERY.
        unsafe {
            let _ = FwpmEngineClose0(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_count_covers_blocks_loopback_and_permits() {
        assert_eq!(filter_count_for(0), 8);
        assert_eq!(filter_count_for(1), 12);
        assert_eq!(filter_count_for(MAX_PERMITTED_APPS), 8 + 4 * 4);
    }

    #[test]
    fn complete_filter_set_needs_every_expected_filter() {
        assert!(!filters_are_complete(0, 0));
        assert!(filters_are_complete(8, 8));
        assert!(!filters_are_complete(7, 8));
        assert!(!filters_are_complete(8, 12));
        assert!(filters_are_complete(12, 12));
    }

    #[test]
    fn keys_are_unique_and_nonzero() {
        assert_ne!(filter_key(0), filter_key(1));
        assert_ne!(filter_key(0), GUID::default());
        assert_ne!(SUBLAYER, GUID::default());
        assert_ne!(app_key(0, 0), block_key(0));
        assert_ne!(app_key(0, 0), app_key(1, 0));
    }

    #[test]
    #[ignore = "changes the host firewall; run only in the test VM"]
    fn default_deny_arms_and_disarms_in_the_vm() {
        let mut engine = WfpEngine::open().expect("open WFP");
        engine.arm_default_deny().expect("arm");
        assert_eq!(engine.verify(), engine.expected_filter_count());
        engine.disarm().expect("disarm");
        assert_eq!(engine.verify(), 0);
    }
}
