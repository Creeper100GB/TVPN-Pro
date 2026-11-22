//! Wächter-Bewertung.
//!
//! Die pure Funktion bildet die Mindestbedingungen ab, die der Dienst bei
//! jedem Statusabruf prüft. Der spätere eigene Wächterprozess kann dieselbe
//! Logik mit den Sekunden- und 30-Sekunden-Takten aufrufen.

/// Alarmstufe aus `contracts/ipc-v0.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Warn,
    Critical,
}

/// Bewertet den Schutz. `Critical` heißt: die Filter sind weg oder Tor hat
/// keinen Circuit; der Dienst muss `FAILCLOSED` gehen.
pub fn evaluate(
    firewall_armed: bool,
    tor_circuit: bool,
    vpn_required: bool,
    vpn_connected: bool,
) -> Level {
    if !firewall_armed || !tor_circuit {
        return Level::Critical;
    }
    if vpn_required && !vpn_connected {
        return Level::Critical;
    }
    if vpn_required {
        Level::Ok
    } else {
        // Tor-only is deliberately visible as degraded, not as full protection.
        Level::Warn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_chain_is_ok() {
        assert_eq!(evaluate(true, true, true, true), Level::Ok);
    }

    #[test]
    fn missing_filter_is_critical() {
        assert_eq!(evaluate(false, true, true, true), Level::Critical);
    }

    #[test]
    fn missing_circuit_is_critical() {
        assert_eq!(evaluate(true, false, true, true), Level::Critical);
    }

    #[test]
    fn missing_vpn_is_critical_when_required() {
        assert_eq!(evaluate(true, true, true, false), Level::Critical);
    }

    #[test]
    fn tor_only_is_warn() {
        assert_eq!(evaluate(true, true, false, false), Level::Warn);
    }
}
