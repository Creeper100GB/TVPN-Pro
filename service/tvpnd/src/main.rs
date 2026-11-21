//! Console front end for the protocol core.
//!
//! The Windows service and named-pipe transport are added next; this binary
//! lets the protocol be exercised with NDJSON on stdin/stdout on any host.

use std::io::{self, BufRead, Write};
use tvpn_core::ipc::{parse_request, Response};
use tvpnd::Service;

fn main() {
    let token = match std::env::var("TVPN_TOKEN") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!("tvpnd: set TVPN_TOKEN for console mode");
            std::process::exit(2);
        }
    };

    let mut service = Service::new(token);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match parse_request(&line) {
            Ok(request) => service.handle(&request),
            Err(error) => Response::error("?", error.code(), Some(error.detail())),
        };

        if serde_json::to_writer(&mut stdout, &response).is_err() {
            break;
        }
        if writeln!(&mut stdout).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}
