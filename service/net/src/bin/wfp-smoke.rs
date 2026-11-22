//! VM-only smoke test for the WFP default-deny layer.
//!
//! Never run this on the host. It blocks all non-loopback traffic until the
//! matching `disarm` completes.

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = tvpn_net::wfp::WfpEngine::open()?;
    engine.arm_default_deny()?;
    let armed = engine.verify();
    engine.disarm()?;
    let disarmed = engine.verify();
    println!("armed={armed} disarmed={disarmed}");
    if armed != 8 || disarmed != 0 {
        return Err("unexpected WFP filter count".into());
    }

    // A dropped engine must clean up after itself. Otherwise a normal service
    // shutdown leaves static filters behind and the host stays blocked.
    engine.arm_default_deny()?;
    let rearmed = engine.verify();
    drop(engine);
    let after_drop = tvpn_net::wfp::WfpEngine::open()?.verify();
    println!("rearmed={rearmed} after_drop={after_drop}");
    if rearmed != 8 || after_drop != 0 {
        return Err("drop did not remove the WFP filters".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("wfp-smoke is Windows-only");
    std::process::exit(1);
}
