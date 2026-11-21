#![cfg(windows)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn hello_and_status_over_the_pipe() {
    let pipe_name = format!(r"\\.\pipe\tvpn-pro-test-{}", std::process::id());
    let token_path = std::env::temp_dir().join(format!("tvpn-token-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&token_path);

    let mut child = Command::new(env!("CARGO_BIN_EXE_tvpnd"))
        .env("TVPN_PIPE_NAME", &pipe_name)
        .env("TVPN_TOKEN_PATH", &token_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tvpnd");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !token_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let token = std::fs::read_to_string(&token_path)
        .expect("token file")
        .trim()
        .to_owned();

    let mut stream = loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe_name)
        {
            Ok(stream) => break stream,
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => panic!("open pipe: {error}"),
        }
    };

    let hello = format!(r#"{{"v":0,"id":"h","cmd":"hello","args":{{"token":"{token}"}}}}"#);
    writeln!(stream, "{hello}").expect("write hello");
    let mut reader = BufReader::new(stream.try_clone().expect("clone pipe"));
    let mut line = String::new();
    reader.read_line(&mut line).expect("read hello response");
    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("hello json");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["protocol"].as_u64(), Some(0));

    let status = r#"{"v":0,"id":"s","cmd":"get_status","args":{}}"#;
    writeln!(stream, "{status}").expect("write status");
    line.clear();
    reader.read_line(&mut line).expect("read status response");
    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("status json");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["state"], "OFF");

    child.kill().ok();
    child.wait().ok();
    let _ = std::fs::remove_file(&token_path);
}
