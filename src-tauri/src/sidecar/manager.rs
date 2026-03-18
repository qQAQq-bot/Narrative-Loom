use super::protocol::{RpcRequest, RpcResponse, RpcError};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use thiserror::Error;

/// Maximum number of restart attempts before giving up
const MAX_RESTART_ATTEMPTS: u32 = 3;

/// Delay between restart attempts in milliseconds
const RESTART_DELAY_MS: u64 = 1000;

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error("Failed to start sidecar: {0}")]
    StartFailed(String),

    #[error("Sidecar not running")]
    NotRunning,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("RPC error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Request timeout")]
    Timeout,

    #[error("Sidecar crashed")]
    Crashed,

    #[error("Max restart attempts exceeded")]
    MaxRestartsExceeded,
}

pub struct SidecarManager {
    process: Arc<Mutex<Option<Child>>>,
    request_id: AtomicU64,
    restart_count: AtomicU32,
    python_path: String,
}

impl SidecarManager {
    pub fn new(python_path: Option<String>) -> Self {
        // Priority: 1. Passed parameter, 2. Config file, 3. Environment variable, 4. Default "python"
        let python_path = python_path
            .or_else(|| {
                // Try to read from config file
                crate::storage::config::ConfigStore::new()
                    .ok()
                    .and_then(|store| store.get_python_exe().ok().flatten())
            })
            .or_else(|| std::env::var("NARRATIVE_LOOM_PYTHON_EXE").ok())
            .unwrap_or_else(|| "python".to_string());

        tracing::debug!("SidecarManager using Python interpreter: {}", python_path);

        Self {
            process: Arc::new(Mutex::new(None)),
            request_id: AtomicU64::new(1),
            restart_count: AtomicU32::new(0),
            python_path,
        }
    }

    /// Get the Python module directory path
    fn get_python_dir() -> Option<PathBuf> {
        // Try multiple strategies to find the python directory

        // 1. Check relative to current executable (for packaged app)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // In packaged app, python folder might be next to executable
                let python_dir = exe_dir.join("python");
                if python_dir.exists() {
                    return Some(python_dir);
                }
                // Or in resources folder
                let resources_dir = exe_dir.join("_up_").join("Resources").join("python");
                if resources_dir.exists() {
                    return Some(resources_dir);
                }
            }
        }

        // 2. Check relative to current working directory (for development)
        if let Ok(cwd) = std::env::current_dir() {
            let python_dir = cwd.join("python");
            if python_dir.exists() {
                return Some(python_dir);
            }
            // Try parent directory (when running from src-tauri)
            let parent_python_dir = cwd.parent().and_then(|p| {
                let dir = p.join("python");
                if dir.exists() { Some(dir) } else { None }
            });
            if parent_python_dir.is_some() {
                return parent_python_dir;
            }
        }

        // 3. Check NARRATIVE_LOOM_PYTHON_PATH environment variable
        if let Ok(path) = std::env::var("NARRATIVE_LOOM_PYTHON_PATH") {
            let python_dir = PathBuf::from(path);
            if python_dir.exists() {
                return Some(python_dir);
            }
        }

        None
    }

    pub fn start(&self) -> Result<(), SidecarError> {
        let mut guard = self.process.lock().unwrap();

        if guard.is_some() {
            return Ok(());
        }

        let python_dir = Self::get_python_dir();

        let mut command = Command::new(&self.python_path);
        command
            .args(["-m", "loom"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set PYTHONPATH to include our loom module
        if let Some(ref python_dir) = python_dir {
            tracing::debug!("Setting PYTHONPATH to: {:?}", python_dir);
            command.env("PYTHONPATH", python_dir);
        } else {
            tracing::warn!("Could not find python directory, sidecar may fail to start");
        }

        // Pass the logs directory path to Python via environment variable
        if let Ok(logs_dir) = crate::storage::paths::get_logs_dir() {
            tracing::debug!("Setting NARRATIVE_LOOM_LOGS_DIR to: {:?}", logs_dir);
            command.env("NARRATIVE_LOOM_LOGS_DIR", logs_dir);
        }

        let child = command
            .spawn()
            .map_err(|e| {
                let msg = format!(
                    "Failed to start Python sidecar: {}. Python dir: {:?}",
                    e, python_dir
                );
                tracing::error!("{}", msg);
                SidecarError::StartFailed(msg)
            })?;

        *guard = Some(child);
        self.restart_count.store(0, Ordering::SeqCst);
        tracing::debug!("Python sidecar started successfully");
        Ok(())
    }

    pub fn stop(&self) -> Result<(), SidecarError> {
        let mut guard = self.process.lock().unwrap();

        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
            tracing::info!("Python sidecar stopped");
        }

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.process.lock().unwrap();
        if let Some(ref mut child) = *guard {
            // Check if process is still alive
            match child.try_wait() {
                Ok(None) => true, // Still running
                Ok(Some(status)) => {
                    tracing::warn!("Sidecar process exited with status: {:?}", status);
                    false
                }
                Err(e) => {
                    tracing::error!("Failed to check sidecar status: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Check if process has crashed and clean up if so
    fn check_and_cleanup(&self) -> bool {
        let mut guard = self.process.lock().unwrap();
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(None) => return true, // Still running
                Ok(Some(status)) => {
                    // Try to read stderr for error information
                    if let Some(ref mut stderr) = child.stderr {
                        let mut stderr_output = String::new();
                        let mut reader = BufReader::new(stderr);
                        // Read available stderr (non-blocking read of what's available)
                        loop {
                            let mut line = String::new();
                            match reader.read_line(&mut line) {
                                Ok(0) => break,
                                Ok(_) => stderr_output.push_str(&line),
                                Err(_) => break,
                            }
                            if stderr_output.len() > 2000 {
                                break;
                            }
                        }
                        if !stderr_output.is_empty() {
                            tracing::error!("Sidecar stderr output:\n{}", stderr_output);
                        }
                    }
                    tracing::warn!("Sidecar process exited with status: {:?}", status);
                    // Clean up the dead process
                    *guard = None;
                    return false;
                }
                Err(e) => {
                    tracing::error!("Failed to check sidecar status: {}", e);
                    *guard = None;
                    return false;
                }
            }
        }
        false
    }

    pub fn restart(&self) -> Result<(), SidecarError> {
        self.stop()?;
        std::thread::sleep(std::time::Duration::from_millis(RESTART_DELAY_MS));
        self.start()
    }

    /// Attempt to restart the sidecar with retry limits (P2-014)
    fn try_auto_restart(&self) -> Result<(), SidecarError> {
        let current_count = self.restart_count.fetch_add(1, Ordering::SeqCst);

        if current_count >= MAX_RESTART_ATTEMPTS {
            tracing::error!("Max restart attempts ({}) exceeded", MAX_RESTART_ATTEMPTS);
            return Err(SidecarError::MaxRestartsExceeded);
        }

        tracing::warn!(
            "Attempting auto-restart ({}/{})",
            current_count + 1,
            MAX_RESTART_ATTEMPTS
        );

        self.restart()
    }

    /// Reset the restart counter (call after successful operation)
    fn reset_restart_count(&self) {
        self.restart_count.store(0, Ordering::SeqCst);
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn call_sync(&self, method: &str, params: Value) -> Result<Value, SidecarError> {
        let mut guard = self.process.lock().unwrap();
        
        let child = guard.as_mut().ok_or(SidecarError::NotRunning)?;
        
        let request = RpcRequest::new(method, params, self.next_id());
        let request_json = serde_json::to_string(&request)?;
        
        let stdin = child.stdin.as_mut().ok_or(SidecarError::NotRunning)?;
        writeln!(stdin, "{}", request_json)?;
        stdin.flush()?;
        
        let stdout = child.stdout.as_mut().ok_or(SidecarError::NotRunning)?;
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        
        let response: RpcResponse = serde_json::from_str(&response_line)?;
        response.into_result().map_err(SidecarError::RpcError)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, SidecarError> {
        // Log the API request to file (no console output)
        super::api_logger::log_api_request(method, &params);
        let start_time = std::time::Instant::now();

        // Check if sidecar is running and try auto-restart if not
        if !self.check_and_cleanup() {
            tracing::warn!("Sidecar not running, attempting auto-restart...");
            self.try_auto_restart()?;
        }

        // Clone params for potential retry
        let params_clone = params.clone();
        let method_str = method.to_string();
        let process = self.process.clone();
        let id = self.next_id();

        let result = tokio::task::spawn_blocking(move || {
            let mut guard = process.lock().unwrap();
            let child = guard.as_mut().ok_or(SidecarError::NotRunning)?;

            let request = RpcRequest::new(&method_str, params, id);
            let request_json = serde_json::to_string(&request)?;

            let stdin = child.stdin.as_mut().ok_or(SidecarError::NotRunning)?;
            writeln!(stdin, "{}", request_json)?;
            stdin.flush()?;

            let stdout = child.stdout.as_mut().ok_or(SidecarError::NotRunning)?;
            let mut reader = BufReader::new(stdout);
            let mut response_line = String::new();
            reader.read_line(&mut response_line)?;

            if response_line.is_empty() {
                return Err(SidecarError::Crashed);
            }

            let response: RpcResponse = serde_json::from_str(&response_line)?;
            response.into_result().map_err(SidecarError::RpcError)
        })
        .await
        .map_err(|_| SidecarError::Crashed)?;

        let final_result = match result {
            Ok(value) => {
                // Reset restart counter on success
                self.reset_restart_count();
                Ok(value)
            }
            Err(SidecarError::IoError(_)) | Err(SidecarError::Crashed) => {
                // IO error or crash might indicate sidecar died, try auto-restart
                tracing::warn!("Sidecar communication failed, attempting auto-restart...");
                if self.try_auto_restart().is_ok() {
                    // Retry the call once after restart
                    self.call_internal(method, params_clone).await
                } else {
                    result
                }
            }
            Err(e) => Err(e),
        };

        // Log the API response
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let log_result = final_result.as_ref().map(|v| v.clone()).map_err(|e| e.to_string());
        super::api_logger::log_api_response(method, &log_result, duration_ms);

        final_result
    }

    /// Internal call without auto-restart (to prevent infinite recursion)
    async fn call_internal(&self, method: &str, params: Value) -> Result<Value, SidecarError> {
        let method_str = method.to_string();
        let process = self.process.clone();
        let id = self.next_id();

        tokio::task::spawn_blocking(move || {
            let mut guard = process.lock().unwrap();
            let child = guard.as_mut().ok_or(SidecarError::NotRunning)?;

            let request = RpcRequest::new(&method_str, params, id);
            let request_json = serde_json::to_string(&request)?;

            let stdin = child.stdin.as_mut().ok_or(SidecarError::NotRunning)?;
            writeln!(stdin, "{}", request_json)?;
            stdin.flush()?;

            let stdout = child.stdout.as_mut().ok_or(SidecarError::NotRunning)?;
            let mut reader = BufReader::new(stdout);
            let mut response_line = String::new();
            reader.read_line(&mut response_line)?;

            if response_line.is_empty() {
                return Err(SidecarError::Crashed);
            }

            let response: RpcResponse = serde_json::from_str(&response_line)?;
            response.into_result().map_err(SidecarError::RpcError)
        })
        .await
        .map_err(|_| SidecarError::Crashed)?
    }

    pub async fn health_check(&self) -> Result<bool, SidecarError> {
        match self.call("ping", Value::Null).await {
            Ok(v) => Ok(v.as_str() == Some("pong")),
            Err(_) => Ok(false),
        }
    }
}

impl Drop for SidecarManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_manager_new() {
        let manager = SidecarManager::new(None);
        assert!(!manager.is_running());
    }
}
