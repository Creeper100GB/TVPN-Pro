//! Tor control-port client from `PLAN.md` 4.6.
//!
//! The controller is transport-only: it authenticates with the cookie file,
//! reads bootstrap and circuit state and can request a new identity. Starting
//! and supervising `tor.exe` belongs to the launcher phase and is tested in the
//! VM.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// A reply from the control port, including its status code and raw lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub code: u16,
    pub lines: Vec<String>,
}

impl Reply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.code)
    }

    fn joined(&self) -> String {
        self.lines.join("\n")
    }
}

#[derive(Debug)]
pub enum TorError {
    Io(io::Error),
    Closed,
    Protocol(String),
    Auth,
}

impl fmt::Display for TorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "tor io: {error}"),
            Self::Closed => formatter.write_str("tor control connection closed"),
            Self::Protocol(detail) => write!(formatter, "tor protocol: {detail}"),
            Self::Auth => formatter.write_str("tor control authentication failed"),
        }
    }
}

impl std::error::Error for TorError {}

impl From<io::Error> for TorError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Blocking Tor control-port client.
#[derive(Debug)]
pub struct TorController {
    reader: BufReader<TcpStream>,
}

impl TorController {
    /// Connects, reads `PROTOCOLINFO` and authenticates with the cookie file.
    pub fn connect(address: &str, cookie_path: &Path) -> Result<Self, TorError> {
        let stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let mut controller = Self {
            reader: BufReader::new(stream),
        };

        let info = controller.command("PROTOCOLINFO 1")?;
        if !info.ok() {
            return Err(TorError::Protocol("PROTOCOLINFO refused".to_owned()));
        }
        if !info.joined().contains("METHODS=COOKIE") {
            return Err(TorError::Protocol("cookie auth not offered".to_owned()));
        }

        let cookie = std::fs::read(cookie_path)?;
        let hex = cookie
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let auth = controller.command(&format!("AUTHENTICATE {hex}"))?;
        if !auth.ok() {
            return Err(TorError::Auth);
        }

        Ok(controller)
    }

    /// Bootstrap percentage from `GETINFO status/bootstrap-phase`.
    pub fn bootstrap(&mut self) -> Result<u8, TorError> {
        let reply = self.command("GETINFO status/bootstrap-phase")?;
        if !reply.ok() {
            return Err(TorError::Protocol("bootstrap query refused".to_owned()));
        }
        for line in &reply.lines {
            if let Some(position) = line.find("PROGRESS=") {
                let value = line[position + "PROGRESS=".len()..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if let Ok(percent) = value.parse::<u8>() {
                    return Ok(percent.min(100));
                }
            }
        }
        Err(TorError::Protocol("no bootstrap progress".to_owned()))
    }

    /// Whether a circuit is established from `GETINFO status/circuit-established`.
    pub fn circuit(&mut self) -> Result<bool, TorError> {
        let reply = self.command("GETINFO status/circuit-established")?;
        if !reply.ok() {
            return Err(TorError::Protocol("circuit query refused".to_owned()));
        }
        for line in &reply.lines {
            if let Some(value) = line.split("status/circuit-established=").nth(1) {
                return Ok(value.trim_start().starts_with('1'));
            }
        }
        Err(TorError::Protocol("no circuit state".to_owned()))
    }

    /// Requests a new circuit. The browser's "New Identity" is separate.
    pub fn new_identity(&mut self) -> Result<(), TorError> {
        let reply = self.command("SIGNAL NEWNYM")?;
        if reply.ok() {
            Ok(())
        } else {
            Err(TorError::Protocol("NEWNYM refused".to_owned()))
        }
    }

    fn command(&mut self, command: &str) -> Result<Reply, TorError> {
        let stream = self.reader.get_mut();
        stream.write_all(command.as_bytes())?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        self.read_reply()
    }

    fn read_reply(&mut self) -> Result<Reply, TorError> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let read = self.reader.read_line(&mut line)?;
            if read == 0 {
                return Err(TorError::Closed);
            }
            let trimmed = line.trim_end_matches(['\r', '\n']).to_owned();
            let code = trimmed
                .get(..3)
                .and_then(|value| value.parse::<u16>().ok())
                .ok_or_else(|| TorError::Protocol("missing status code".to_owned()))?;
            let done = trimmed.as_bytes().get(3) == Some(&b' ');
            lines.push(trimmed);
            if done {
                return Ok(Reply { code, lines });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    fn cookie_file() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tvpn-tor-cookie-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        std::fs::write(&path, [0xABu8; 32]).expect("cookie");
        path
    }

    fn run_mock(listener: TcpListener, reject: bool) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut writer = stream.try_clone().expect("clone");
            let mut reader = BufReader::new(stream);
            let mut line = String::new();

            reader.read_line(&mut line).expect("protocolinfo");
            writer
                .write_all(b"250-PROTOCOLINFO 1\r\n250-AUTH METHODS=COOKIE\r\n250 OK\r\n")
                .expect("protocolinfo reply");
            line.clear();

            reader.read_line(&mut line).expect("authenticate");
            if reject {
                writer
                    .write_all(b"515 Authentication failed\r\n")
                    .expect("auth reject");
                return;
            }
            writer.write_all(b"250 OK\r\n").expect("auth ok");
            line.clear();

            reader.read_line(&mut line).expect("bootstrap");
            writer
                .write_all(
                    b"250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=100 TAG=done\r\n250 OK\r\n",
                )
                .expect("bootstrap reply");
            line.clear();

            reader.read_line(&mut line).expect("circuit");
            writer
                .write_all(b"250-status/circuit-established=1\r\n250 OK\r\n")
                .expect("circuit reply");
            line.clear();

            reader.read_line(&mut line).expect("newnym");
            writer.write_all(b"250 OK\r\n").expect("newnym reply");
        })
    }

    #[test]
    fn authenticates_and_reads_state() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let cookie = cookie_file();
        let server = run_mock(listener, false);

        let mut tor = TorController::connect(&address.to_string(), &cookie).expect("connect");
        assert_eq!(tor.bootstrap().expect("bootstrap"), 100);
        assert!(tor.circuit().expect("circuit"));
        tor.new_identity().expect("new identity");

        server.join().expect("server");
        let _ = std::fs::remove_file(cookie);
    }

    #[test]
    fn rejects_a_refused_cookie() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let cookie = cookie_file();
        let server = run_mock(listener, true);

        let error = TorController::connect(&address.to_string(), &cookie).expect_err("must fail");
        assert!(matches!(error, TorError::Auth));

        server.join().expect("server");
        let _ = std::fs::remove_file(cookie);
    }
}
