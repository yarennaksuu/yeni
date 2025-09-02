// killer.rs
use crate::config::KillPolicy;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::CloseHandle,
    System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
};

#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid as NixPid;

pub struct ProcessKiller {
    policy: KillPolicy,
    cooldown_tracker: HashMap<u32, Instant>,
    retry_tracker: HashMap<u32, u32>,
}

#[derive(Debug)]
pub enum KillError {
    InCooldown,
    MaxRetriesReached,
    InvalidHandle,
    SystemError(String),
    NotImplemented,
    #[allow(dead_code)]
    AccessDenied,
}

impl std::fmt::Display for KillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KillError::InCooldown => write!(f, "Process is in cooldown period"),
            KillError::MaxRetriesReached => write!(f, "Maximum retry attempts reached"),
            KillError::InvalidHandle => write!(f, "Invalid process handle"),
            KillError::SystemError(e) => write!(f, "System error: {}", e),
            KillError::NotImplemented => write!(f, "Feature not implemented"),
            KillError::AccessDenied => write!(f, "Access denied (requires admin privileges)"),
        }
    }
}

impl std::error::Error for KillError {}

impl ProcessKiller {
    pub fn new(policy: KillPolicy) -> Self {
        Self {
            policy,
            cooldown_tracker: HashMap::new(),
            retry_tracker: HashMap::new(),
        }
    }

    pub fn kill_process(&mut self, pid: u32, name: &str) -> Result<(), KillError> {
        // Check cooldown
        if let Some(last_attempt) = self.cooldown_tracker.get(&pid) {
            let elapsed = Instant::now().duration_since(*last_attempt);
            if elapsed < Duration::from_millis(self.policy.cooldown_ms) {
                debug!("Process {} (PID: {}) is in cooldown period", name, pid);
                return Err(KillError::InCooldown);
            }
        }

        // Check retry count
        let retries = self.retry_tracker.get(&pid).copied().unwrap_or(0);
        if retries >= self.policy.max_retry_attempts {
            warn!("Max retry attempts reached for process {} (PID: {})", name, pid);
            return Err(KillError::MaxRetriesReached);
        }

        info!("Attempting to kill process: {} (PID: {})", name, pid);

        // Graceful termination
        if self.policy.graceful_kill {
            if let Ok(_) = self.graceful_terminate(pid) {
                info!("Process {} gracefully terminated", name);
                self.cooldown_tracker.insert(pid, Instant::now());
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(self.policy.force_kill_timeout_ms));
        }

        // Force termination
        match self.force_terminate(pid) {
            Ok(_) => {
                info!("Process {} forcefully terminated", name);
                self.cooldown_tracker.insert(pid, Instant::now());
                Ok(())
            }
            Err(e) => {
                error!("Failed to kill process {} (PID: {}): {:?}", name, pid, e);
                self.retry_tracker.insert(pid, retries + 1);
                self.cooldown_tracker.insert(pid, Instant::now());
                Err(e)
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn graceful_terminate(&self, _pid: u32) -> Result<(), KillError> {
        // WM_CLOSE implementation can be added here in the future
        Err(KillError::NotImplemented)
    }

    #[cfg(unix)]
    fn graceful_terminate(&self, pid: u32) -> Result<(), KillError> {
        let nix_pid = NixPid::from_raw(pid as i32);
        kill(nix_pid, Signal::SIGTERM).map_err(|e| KillError::SystemError(e.to_string()))
    }

    #[cfg(target_os = "windows")]
    fn force_terminate(&self, pid: u32) -> Result<(), KillError> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
                .map_err(|e| KillError::SystemError(e.to_string()))?;
            
            if handle.is_invalid() {
                return Err(KillError::InvalidHandle);
            }
            
            let result = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
            
            if result.is_ok() {
                Ok(())
            } else {
                Err(KillError::SystemError("TerminateProcess failed".to_string()))
            }
        }
    }

    #[cfg(unix)]
    fn force_terminate(&self, pid: u32) -> Result<(), KillError> {
        let nix_pid = NixPid::from_raw(pid as i32);
        kill(nix_pid, Signal::SIGKILL).map_err(|e| KillError::SystemError(e.to_string()))
    }

    /// Cleanup old cooldown/retry entries
    pub fn cleanup_old_entries(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(300);

        self.cooldown_tracker.retain(|_, last| now.duration_since(*last) < timeout);
        let active_pids: Vec<u32> = self.cooldown_tracker.keys().copied().collect();
        self.retry_tracker.retain(|pid, _| active_pids.contains(pid));
    }
}