use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::config::ConfigManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum IpcMessage {
    Trigger {
        #[serde(default)]
        file: Option<PathBuf>,
        #[serde(default)]
        files: Option<Vec<PathBuf>>,
    },
    Toggle,
    Settings,
    Ping,
}

pub struct IpcClient;

impl IpcClient {
    /// Pings the running daemon to verify socket responsiveness.
    pub fn ping() -> Result<bool> {
        Self::send(&IpcMessage::Ping)
    }

    /// Attempts to send an IPC message to the running daemon.
    /// Returns Ok(true) if connected and sent, Ok(false) or Err if no daemon.
    pub fn send(message: &IpcMessage) -> Result<bool> {
        let sock_path = ConfigManager::socket_path();
        if !sock_path.exists() {
            return Ok(false);
        }

        let mut stream = match UnixStream::connect(&sock_path) {
            Ok(s) => s,
            Err(_) => {
                // Stale socket file
                let _ = std::fs::remove_file(&sock_path);
                return Ok(false);
            }
        };

        let mut json = serde_json::to_string(message)?;
        json.push('\n');
        stream.write_all(json.as_bytes())?;
        Ok(true)
    }
}

pub struct IpcServer;

impl IpcServer {
    /// Starts the IPC listener loop and returns the UnixListener.
    pub fn bind() -> Result<UnixListener> {
        let sock_path = ConfigManager::socket_path();
        if sock_path.exists() {
            let _ = std::fs::remove_file(&sock_path);
        }

        if let Some(parent) = sock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(&sock_path)
            .with_context(|| format!("Failed to bind IPC socket at {}", sock_path.display()))?;

        listener.set_nonblocking(true)?;
        Ok(listener)
    }

    /// Reads next message from non-blocking listener if available.
    pub fn try_recv(listener: &UnixListener) -> Option<IpcMessage> {
        match listener.accept() {
            Ok((stream, _)) => {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    serde_json::from_str::<IpcMessage>(line.trim()).ok()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
