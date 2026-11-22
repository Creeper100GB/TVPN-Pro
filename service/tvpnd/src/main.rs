//! `tvpnd` front end: named pipe on Windows, NDJSON console elsewhere.

use std::io::{self, BufRead, Write};
use tvpn_core::ipc::{parse_request, Response};
use tvpnd::Service;

fn console() {
    let token = match std::env::var("TVPN_TOKEN") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            #[cfg(windows)]
            {
                let token = tvpnd::pipe::generate_token();
                if let Err(error) = tvpnd::pipe::write_token(&token) {
                    eprintln!("tvpnd: cannot write the session token: {error}");
                    std::process::exit(1);
                }
                token
            }
            #[cfg(not(windows))]
            {
                eprintln!("tvpnd: set TVPN_TOKEN for console mode");
                std::process::exit(2);
            }
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

#[cfg(windows)]
fn main() {
    if std::env::args().any(|argument| argument == "--console") {
        console();
        return;
    }

    if std::env::args().any(|argument| argument == "--install") {
        if let Err(error) = tvpnd::service::install() {
            eprintln!("tvpnd: install failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|argument| argument == "--uninstall") {
        if let Err(error) = tvpnd::service::uninstall() {
            eprintln!("tvpnd: uninstall failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|argument| argument == "--service") {
        if let Err(error) = tvpnd::service::run() {
            eprintln!("tvpnd: {error}");
            std::process::exit(1);
        }
        return;
    }

    let token = tvpnd::pipe::generate_token();
    if let Err(error) = tvpnd::pipe::run(&token) {
        eprintln!("tvpnd: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    console();
}
