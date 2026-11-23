//! VM-only smoke test for the WFP default-deny layer.
//!
//! Never run this on the host. It blocks all non-loopback traffic until the
//! matching `disarm` completes.

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tvpn_net::wfp::{filter_count_for, WfpEngine};

    let base = filter_count_for(0);
    let mut engine = WfpEngine::open()?;
    engine.arm_default_deny()?;
    let armed = engine.verify();
    engine.disarm()?;
    let disarmed = engine.verify();
    println!("armed={armed} disarmed={disarmed}");
    if armed != base || disarmed != 0 {
        return Err("unexpected WFP filter count".into());
    }

    // A dropped engine must NOT clean up: a service stop or crash is not a
    // user-requested disable, so the filters stay (fail-closed). Only an
    // explicit disarm removes them.
    engine.arm_default_deny()?;
    let rearmed = engine.verify();
    drop(engine);
    let mut reopened = WfpEngine::open()?;
    let after_drop = reopened.verify();
    println!("rearmed={rearmed} after_drop={after_drop}");
    if rearmed != base || after_drop != base {
        return Err("drop must leave the fail-closed filter set installed".into());
    }

    reopened.disarm()?;
    let after_disarm = reopened.verify();
    println!("after_disarm={after_disarm}");
    if after_disarm != 0 {
        return Err("disarm did not remove the WFP filters".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("wfp-smoke is Windows-only");
    std::process::exit(1);
}
