//! Integrated Terminal Module
//!
//! Provides terminal emulation with support for PowerShell, WSL, and CMD on Windows.
//! Uses portable-pty for cross-platform PTY support with event-based output.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

/// Terminal ID type for identifying terminal sessions
pub type TerminalId = String;

/// Supported shell types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    PowerShell,
    Wsl,
    Cmd,
    Bash,
}

/// Terminal session data
pub struct TerminalSession {
    /// PTY master for resizing
    master: Box<dyn portable_pty::MasterPty + Send>,
    /// PTY writer (cloned from master)
    writer: Box<dyn Write + Send>,
    /// Shell type for this terminal
    shell_type: ShellType,
    /// Terminal size
    size: PtySize,
}

/// Terminal manager state
pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<TerminalId, TerminalSession>>>,
    next_id: Arc<Mutex<usize>>,
    app_handle: AppHandle,
}

/// Terminal errors
#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("Failed to spawn terminal: {0}")]
    SpawnFailed(String),

    #[error("Terminal not found: {0}")]
    NotFound(String),

    #[error("Failed to write to terminal: {0}")]
    WriteFailed(String),

    #[error("Failed to read from terminal: {0}")]
    ReadFailed(String),

    #[error("Failed to resize terminal: {0}")]
    ResizeFailed(String),

    #[error("Shell not available: {0}")]
    ShellNotAvailable(String),
}

impl TerminalManager {
    /// Create a new terminal manager
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
            app_handle,
        }
    }

    /// Generate next terminal ID
    fn next_id(&self) -> TerminalId {
        let mut id = self.next_id.lock().unwrap();
        let terminal_id = format!("term-{}", *id);
        *id += 1;
        terminal_id
    }

    /// Detect available shells on the system
    pub fn detect_available_shells() -> Vec<ShellType> {
        let mut shells = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Check for PowerShell
            if std::process::Command::new("powershell")
                .arg("-Command")
                .arg("$PSVersionTable.PSVersion")
                .output()
                .is_ok()
            {
                shells.push(ShellType::PowerShell);
            }

            // Check for WSL
            if std::process::Command::new("wsl")
                .arg("--status")
                .output()
                .is_ok()
            {
                shells.push(ShellType::Wsl);
            }

            // CMD is always available on Windows
            shells.push(ShellType::Cmd);
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix systems, bash is typically available
            shells.push(ShellType::Bash);
        }

        shells
    }

    /// Get default shell for the platform
    pub fn default_shell() -> ShellType {
        #[cfg(target_os = "windows")]
        {
            ShellType::PowerShell
        }

        #[cfg(not(target_os = "windows"))]
        {
            ShellType::Bash
        }
    }

    /// Spawn a new terminal session
    pub fn spawn_terminal(
        &self,
        shell_type: ShellType,
        cols: u16,
        rows: u16,
        working_dir: Option<String>,
    ) -> Result<TerminalId, TerminalError> {
        let pty_system = native_pty_system();

        // Create PTY with specified size
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to create PTY: {}", e)))?;

        // Build command based on shell type
        let mut cmd = self.build_shell_command(&shell_type)?;

        // Set working directory if provided
        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        }

        // Spawn the shell process
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to spawn shell: {}", e)))?;

        // Drop the slave side of the PTY (child process has it)
        drop(pair.slave);

        // Create writer from master
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to get writer: {}", e)))?;

        // Get reader from master for background thread
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to get reader: {}", e)))?;

        // Generate ID
        let id = self.next_id();

        // Create session
        let session = TerminalSession {
            master: pair.master,
            writer,
            shell_type,
            size,
        };

        // Store session
        self.sessions.lock().unwrap().insert(id.clone(), session);

        // Spawn background thread to read output and emit events
        let terminal_id = id.clone();
        let app_handle = self.app_handle.clone();

        thread::spawn(move || {
            tracing::info!("Terminal {} reader thread started", terminal_id);
            let mut buffer = [0u8; 4096];

            loop {
                // Use the raw reader directly without BufReader
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - terminal closed
                        tracing::info!("Terminal {} closed (EOF)", terminal_id);
                        if let Err(e) = app_handle.emit_to("main", &format!("terminal-{}-closed", terminal_id), ()) {
                            tracing::error!("Failed to emit close event: {}", e);
                        }
                        break;
                    }
                    Ok(n) => {
                        // Got data - emit it to frontend
                        tracing::debug!("Terminal {} read {} bytes", terminal_id, n);

                        // Convert to string, replacing invalid UTF-8 with replacement character
                        let data = String::from_utf8_lossy(&buffer[..n]).to_string();

                        tracing::debug!("Terminal {} emitting output: {:?}", terminal_id, &data[..data.len().min(50)]);

                        // Emit to main window
                        if let Err(e) = app_handle.emit_to("main", &format!("terminal-{}-output", terminal_id), data) {
                            tracing::error!("Failed to emit output for terminal {}: {}", terminal_id, e);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, sleep briefly and try again
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Failed to read from terminal {}: {} (kind: {:?})", terminal_id, e, e.kind());
                        if let Err(emit_err) = app_handle.emit_to(
                            "main",
                            &format!("terminal-{}-error", terminal_id),
                            format!("Read error: {}", e),
                        ) {
                            tracing::error!("Failed to emit error event: {}", emit_err);
                        }
                        break;
                    }
                }
            }

            tracing::info!("Terminal {} reader thread exiting", terminal_id);
        });

        tracing::info!("Spawned terminal {} with shell {:?}", id, shell_type);
        Ok(id)
    }

    /// Build shell command based on shell type
    fn build_shell_command(&self, shell_type: &ShellType) -> Result<CommandBuilder, TerminalError> {
        let cmd = match shell_type {
            #[cfg(target_os = "windows")]
            ShellType::PowerShell => {
                let mut cmd = CommandBuilder::new("powershell.exe");
                cmd.arg("-NoLogo");
                cmd
            }

            #[cfg(target_os = "windows")]
            ShellType::Wsl => {
                // Check if WSL is available
                if !Self::detect_available_shells().contains(&ShellType::Wsl) {
                    return Err(TerminalError::ShellNotAvailable(
                        "WSL is not installed or not available".to_string(),
                    ));
                }
                CommandBuilder::new("wsl.exe")
            }

            #[cfg(target_os = "windows")]
            ShellType::Cmd => CommandBuilder::new("cmd.exe"),

            #[cfg(not(target_os = "windows"))]
            ShellType::Bash => {
                let mut cmd = CommandBuilder::new("bash");
                cmd.arg("-l"); // Login shell
                cmd
            }

            #[allow(unreachable_patterns)]
            _ => {
                return Err(TerminalError::ShellNotAvailable(format!(
                    "Shell type {:?} not available on this platform",
                    shell_type
                )))
            }
        };

        Ok(cmd)
    }

    /// Write data to terminal
    pub fn write_terminal(&self, id: &TerminalId, data: &str) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| TerminalError::NotFound(id.clone()))?;

        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| TerminalError::WriteFailed(e.to_string()))?;

        session
            .writer
            .flush()
            .map_err(|e| TerminalError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    /// Resize terminal
    pub fn resize_terminal(
        &self,
        id: &TerminalId,
        cols: u16,
        rows: u16,
    ) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| TerminalError::NotFound(id.clone()))?;

        let new_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        session
            .master
            .resize(new_size)
            .map_err(|e| TerminalError::ResizeFailed(e.to_string()))?;

        session.size = new_size;
        Ok(())
    }

    /// Close terminal session
    pub fn close_terminal(&self, id: &TerminalId) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .remove(id)
            .ok_or_else(|| TerminalError::NotFound(id.clone()))?;

        tracing::info!("Closed terminal {}", id);
        Ok(())
    }

    /// Get list of active terminal IDs
    pub fn list_terminals(&self) -> Vec<TerminalId> {
        self.sessions
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_available_shells() {
        let shells = TerminalManager::detect_available_shells();
        assert!(!shells.is_empty(), "Should detect at least one shell");

        #[cfg(target_os = "windows")]
        {
            // On Windows, we should at least have CMD
            assert!(
                shells.contains(&ShellType::Cmd)
                    || shells.contains(&ShellType::PowerShell),
                "Should detect CMD or PowerShell on Windows"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, we should have bash
            assert!(
                shells.contains(&ShellType::Bash),
                "Should detect bash on Unix systems"
            );
        }
    }

    #[test]
    fn test_default_shell() {
        let default = TerminalManager::default_shell();

        #[cfg(target_os = "windows")]
        assert_eq!(default, ShellType::PowerShell);

        #[cfg(not(target_os = "windows"))]
        assert_eq!(default, ShellType::Bash);
    }

    #[test]
    fn test_terminal_manager_creation() {
        let manager = TerminalManager::new();
        assert_eq!(manager.list_terminals().len(), 0);
    }
}
