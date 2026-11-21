//! State machine from `contracts/ipc-v0.md` section 8.
//!
//! [`next`] is a pure function. It does not log, does not touch the network and
//! does not keep history; the caller owns the current state and emits exactly
//! one `state_changed` when the returned state differs from the previous one.

use std::fmt;

/// The eleven states of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Off,
    Preflight,
    Arming,
    Portal,
    VpnUp,
    TorUp,
    Ready,
    Degraded,
    FailClosed,
    Teardown,
    Recovery,
}

impl State {
    /// Every state, in contract order.
    pub const ALL: [State; 11] = [
        State::Off,
        State::Preflight,
        State::Arming,
        State::Portal,
        State::VpnUp,
        State::TorUp,
        State::Ready,
        State::Degraded,
        State::FailClosed,
        State::Teardown,
        State::Recovery,
    ];

    /// Wire name used in IPC payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Off => "OFF",
            State::Preflight => "PREFLIGHT",
            State::Arming => "ARMING",
            State::Portal => "PORTAL",
            State::VpnUp => "VPN_UP",
            State::TorUp => "TOR_UP",
            State::Ready => "READY",
            State::Degraded => "DEGRADED",
            State::FailClosed => "FAILCLOSED",
            State::Teardown => "TEARDOWN",
            State::Recovery => "RECOVERY",
        }
    }

    /// True while our firewall filters are supposed to be installed.
    ///
    /// `FAILCLOSED` counts as armed on purpose: the filters stay set until
    /// `TEARDOWN` or `RECOVERY` removes them.
    pub fn is_armed(self) -> bool {
        matches!(
            self,
            State::Arming
                | State::Portal
                | State::VpnUp
                | State::TorUp
                | State::Ready
                | State::Degraded
                | State::FailClosed
        )
    }

    /// True for states that normally pass within seconds.
    pub fn is_transient(self) -> bool {
        matches!(
            self,
            State::Preflight | State::Arming | State::VpnUp | State::TorUp | State::Teardown
        )
    }
}

impl fmt::Display for State {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Inputs that move the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Enable,
    Disable,
    Panic,
    Recover,
    PreflightOk,
    PreflightFailed,
    PortalRequired,
    PortalDone,
    PortalTimeout,
    VpnUp,
    VpnFailed,
    TorUp,
    TorFailed,
    LeakCheckOk,
    LeakCheckFailed,
    WatchdogWarn,
    WatchdogHealed,
    WatchdogCritical,
    TeardownDone,
    TeardownIncomplete,
    RecoveryDone,
    RecoveryFailed,
    StaleSession,
}

impl Event {
    /// Wire name used in logs and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Event::Enable => "enable",
            Event::Disable => "disable",
            Event::Panic => "panic",
            Event::Recover => "recover",
            Event::PreflightOk => "preflight_ok",
            Event::PreflightFailed => "preflight_failed",
            Event::PortalRequired => "portal_required",
            Event::PortalDone => "portal_done",
            Event::PortalTimeout => "portal_timeout",
            Event::VpnUp => "vpn_up",
            Event::VpnFailed => "vpn_failed",
            Event::TorUp => "tor_up",
            Event::TorFailed => "tor_failed",
            Event::LeakCheckOk => "leak_check_ok",
            Event::LeakCheckFailed => "leak_check_failed",
            Event::WatchdogWarn => "watchdog_warn",
            Event::WatchdogHealed => "watchdog_healed",
            Event::WatchdogCritical => "watchdog_critical",
            Event::TeardownDone => "teardown_done",
            Event::TeardownIncomplete => "teardown_incomplete",
            Event::RecoveryDone => "recovery_done",
            Event::RecoveryFailed => "recovery_failed",
            Event::StaleSession => "stale_session",
        }
    }
}

/// Why a transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    pub from: State,
    pub event: Event,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "E_STATE_CONFLICT: {} + {}",
            self.from.as_str(),
            self.event.as_str()
        )
    }
}

impl std::error::Error for TransitionError {}

/// Computes the next state.
///
/// Invariants guaranteed here:
/// - `Panic` is accepted from every state and ends in `TEARDOWN`.
/// - `Recover` is accepted from every state except `OFF`.
/// - `OFF` can only be left through `Enable` or `StaleSession`.
/// - `FAILCLOSED` never goes straight back to an armed state.
///
/// A result equal to `state` is a no-op, not a transition: the caller must not
/// emit `state_changed` for it.
pub fn next(state: State, event: Event) -> Result<State, TransitionError> {
    use Event as E;
    use State as S;

    if event == E::Panic {
        return Ok(S::Teardown);
    }

    if event == E::Recover {
        return if state == S::Off {
            Err(TransitionError { from: state, event })
        } else {
            Ok(S::Recovery)
        };
    }

    let target = match (state, event) {
        (S::Off, E::Enable) => S::Preflight,
        (S::Off, E::StaleSession) => S::Recovery,

        (S::Preflight, E::PreflightOk) => S::Arming,
        (S::Preflight, E::PreflightFailed) => S::FailClosed,
        (S::Preflight, E::Disable) => S::Teardown,

        (S::Arming, E::PortalRequired) => S::Portal,
        (S::Arming, E::VpnUp) => S::VpnUp,
        (S::Arming, E::VpnFailed) => S::FailClosed,
        (S::Arming, E::WatchdogCritical) => S::FailClosed,
        (S::Arming, E::Disable) => S::Teardown,

        (S::Portal, E::PortalDone) => S::VpnUp,
        (S::Portal, E::PortalTimeout) => S::FailClosed,
        (S::Portal, E::VpnFailed) => S::FailClosed,
        (S::Portal, E::WatchdogCritical) => S::FailClosed,
        (S::Portal, E::Disable) => S::Teardown,

        (S::VpnUp, E::TorUp) => S::TorUp,
        (S::VpnUp, E::TorFailed) => S::FailClosed,
        (S::VpnUp, E::WatchdogCritical) => S::FailClosed,
        (S::VpnUp, E::Disable) => S::Teardown,

        (S::TorUp, E::LeakCheckOk) => S::Ready,
        (S::TorUp, E::LeakCheckFailed) => S::FailClosed,
        (S::TorUp, E::TorFailed) => S::FailClosed,
        (S::TorUp, E::WatchdogCritical) => S::FailClosed,
        (S::TorUp, E::Disable) => S::Teardown,

        (S::Ready, E::WatchdogWarn) => S::Degraded,
        (S::Ready, E::WatchdogCritical) => S::FailClosed,
        (S::Ready, E::Disable) => S::Teardown,

        (S::Degraded, E::WatchdogHealed) => S::Ready,
        (S::Degraded, E::WatchdogCritical) => S::FailClosed,
        (S::Degraded, E::Disable) => S::Teardown,

        (S::FailClosed, E::Disable) => S::Teardown,

        (S::Teardown, E::TeardownDone) => S::Off,
        (S::Teardown, E::TeardownIncomplete) => S::Recovery,

        (S::Recovery, E::RecoveryDone) => S::Off,
        // Staying in RECOVERY on a failed recovery is deliberate: the network
        // stays closed and the caller raises a critical alarm instead of a
        // state change.
        (S::Recovery, E::RecoveryFailed) => S::Recovery,

        _ => return Err(TransitionError { from: state, event }),
    };

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Event as E;
    use State as S;

    #[test]
    fn enable_is_only_allowed_from_off() {
        for state in S::ALL {
            let result = next(state, E::Enable);
            if state == S::Off {
                assert_eq!(result, Ok(S::Preflight));
            } else {
                assert_eq!(
                    result,
                    Err(TransitionError {
                        from: state,
                        event: E::Enable
                    })
                );
            }
        }
    }

    #[test]
    fn happy_path_reaches_ready() {
        let mut state = S::Off;
        for (event, expected) in [
            (E::Enable, S::Preflight),
            (E::PreflightOk, S::Arming),
            (E::VpnUp, S::VpnUp),
            (E::TorUp, S::TorUp),
            (E::LeakCheckOk, S::Ready),
        ] {
            state = next(state, event).expect("happy path must be allowed");
            assert_eq!(state, expected);
        }
    }

    #[test]
    fn panic_from_every_state_ends_in_teardown() {
        for state in S::ALL {
            assert_eq!(
                next(state, E::Panic),
                Ok(S::Teardown),
                "panic must work from {state}"
            );
        }
    }

    #[test]
    fn panic_never_lands_in_recovery() {
        for state in S::ALL {
            assert_ne!(next(state, E::Panic), Ok(S::Recovery));
        }
    }

    #[test]
    fn recover_is_refused_only_in_off() {
        for state in S::ALL {
            let result = next(state, E::Recover);
            if state == S::Off {
                assert!(result.is_err());
            } else {
                assert_eq!(result, Ok(S::Recovery));
            }
        }
    }

    #[test]
    fn failclosed_does_not_reopen_directly() {
        for event in [
            E::PreflightOk,
            E::PortalDone,
            E::VpnUp,
            E::TorUp,
            E::LeakCheckOk,
            E::WatchdogWarn,
            E::WatchdogHealed,
        ] {
            assert!(
                next(S::FailClosed, event).is_err(),
                "FAILCLOSED must not accept {event:?}"
            );
        }
        assert_eq!(next(S::FailClosed, E::Disable), Ok(S::Teardown));
    }

    #[test]
    fn teardown_and_recovery_finish_in_off() {
        assert_eq!(next(S::Teardown, E::TeardownDone), Ok(S::Off));
        assert_eq!(next(S::Teardown, E::TeardownIncomplete), Ok(S::Recovery));
        assert_eq!(next(S::Recovery, E::RecoveryDone), Ok(S::Off));
        assert_eq!(next(S::Recovery, E::RecoveryFailed), Ok(S::Recovery));
    }

    #[test]
    fn stale_session_starts_recovery() {
        assert_eq!(next(S::Off, E::StaleSession), Ok(S::Recovery));
    }

    #[test]
    fn disable_works_from_every_armed_state() {
        for state in S::ALL.into_iter().filter(|state| state.is_armed()) {
            assert_eq!(
                next(state, E::Disable),
                Ok(S::Teardown),
                "disable must work from {state}"
            );
        }
    }

    #[test]
    fn armed_states_match_the_contract() {
        let armed: Vec<&str> = S::ALL
            .into_iter()
            .filter(|state| state.is_armed())
            .map(State::as_str)
            .collect();
        assert_eq!(
            armed,
            [
                "ARMING",
                "PORTAL",
                "VPN_UP",
                "TOR_UP",
                "READY",
                "DEGRADED",
                "FAILCLOSED"
            ]
        );
    }

    #[test]
    fn wire_names_are_stable() {
        assert_eq!(S::FailClosed.as_str(), "FAILCLOSED");
        assert_eq!(S::TorUp.as_str(), "TOR_UP");
        assert_eq!(E::LeakCheckFailed.as_str(), "leak_check_failed");
    }
}
