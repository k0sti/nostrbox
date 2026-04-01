//! Read FIPS daemon status via the control socket.
//!
//! Protocol: send `{"command":"<cmd>"}\n`, read one line of JSON back.
//! Same protocol as `fipsctl`.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use crate::FipsError;

/// Client for querying FIPS daemon status via control socket.
pub struct FipsClient {
    socket_path: PathBuf,
    timeout: Duration,
}

/// Node status overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FipsStatus {
    /// Raw JSON from the daemon (we pass through whatever FIPS returns).
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// A connected peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FipsPeer {
    pub npub: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// An active link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FipsLink {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl FipsClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            timeout: Duration::from_secs(5),
        }
    }

    /// Check if the FIPS daemon is reachable.
    pub fn is_running(&self) -> bool {
        self.send_query("show_status").is_ok()
    }

    /// Get node status.
    pub fn status(&self) -> Result<FipsStatus, FipsError> {
        let resp = self.send_query("show_status")?;
        let data = self.extract_data(resp)?;
        Ok(FipsStatus { data })
    }

    /// Get connected peers.
    pub fn peers(&self) -> Result<Vec<FipsPeer>, FipsError> {
        let resp = self.send_query("show_peers")?;
        let data = self.extract_data(resp)?;
        // Data is typically an array of peer objects
        serde_json::from_value(data.clone()).unwrap_or_else(|_| {
            // If it's not an array, wrap in vec
            vec![FipsPeer {
                npub: None,
                extra: data,
            }]
        })
        .pipe_ok()
    }

    /// Get active links.
    pub fn links(&self) -> Result<Vec<FipsLink>, FipsError> {
        let resp = self.send_query("show_links")?;
        let data = self.extract_data(resp)?;
        serde_json::from_value(data.clone()).unwrap_or_else(|_| vec![FipsLink { data }]).pipe_ok()
    }

    /// Send a raw command and return the response.
    pub fn raw_command(&self, command: &str) -> Result<serde_json::Value, FipsError> {
        self.send_query(command)
    }

    fn send_query(&self, command: &str) -> Result<serde_json::Value, FipsError> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            FipsError::Connection(format!(
                "cannot connect to {}: {e}",
                self.socket_path.display()
            ))
        })?;

        stream.set_read_timeout(Some(self.timeout)).ok();
        stream.set_write_timeout(Some(self.timeout)).ok();

        let request = format!("{{\"command\":\"{command}\"}}\n");
        stream
            .write_all(request.as_bytes())
            .map_err(|e| FipsError::Connection(format!("failed to send: {e}")))?;
        let _ = stream.shutdown(std::net::Shutdown::Write);

        let reader = BufReader::new(&stream);
        let line = reader
            .lines()
            .next()
            .ok_or_else(|| FipsError::Connection("no response from daemon".into()))?
            .map_err(|e| FipsError::Connection(format!("failed to read: {e}")))?;

        serde_json::from_str(&line)
            .map_err(|e| FipsError::Protocol(format!("invalid JSON response: {e}")))
    }

    fn extract_data(&self, resp: serde_json::Value) -> Result<serde_json::Value, FipsError> {
        let status = resp
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        if status == "error" {
            let msg = resp
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(FipsError::Daemon(msg.to_string()));
        }

        Ok(resp
            .get("data")
            .cloned()
            .unwrap_or(resp))
    }
}

/// Helper trait to wrap a value in Ok.
trait PipeOk: Sized {
    fn pipe_ok(self) -> Result<Self, FipsError> {
        Ok(self)
    }
}
impl<T> PipeOk for T {}
