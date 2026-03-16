//! Process killing functionality.

use crate::scanner::{get_scanner, Protocol, ScanOptions};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Kill operation errors.
#[derive(Debug, Error)]
pub enum KillError {
    #[error("Process {0} not found")]
    ProcessNotFound(u32),
    
    #[error("Permission denied to kill process {0}")]
    PermissionDenied(u32),
    
    #[error("Process {0} did not terminate within timeout")]
    Timeout(u32),
    
    #[error("Port {0} is not in use")]
    PortNotInUse(u16),
    
    #[error("Failed to kill process: {0}")]
    Failed(String),
}

/// Options for kill operation.
#[derive(Debug, Clone, Default)]
pub struct KillOptions {
    /// Use SIGKILL immediately instead of SIGTERM.
    pub force: bool,
    /// Timeout in seconds for graceful termination.
    pub timeout: u64,
}

impl KillOptions {
    pub fn new() -> Self {
        Self {
            force: false,
            timeout: 5,
        }
    }
    
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
    
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout = secs;
        self
    }
}

/// Send SIGTERM only to a process (no SIGKILL fallback).
pub fn send_sigterm(pid: u32) -> Result<(), KillError> {
    #[cfg(unix)]
    {
        use libc::{kill, SIGTERM};
        
        let pid = pid as libc::pid_t;
        
        // Check if process exists
        let exists = unsafe { kill(pid, 0) };
        if exists != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Err(KillError::ProcessNotFound(pid as u32));
            } else if err.raw_os_error() == Some(libc::EPERM) {
                return Err(KillError::PermissionDenied(pid as u32));
            }
        }
        
        let result = unsafe { kill(pid, SIGTERM) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            return Err(KillError::Failed(err.to_string()));
        }
        
        Ok(())
    }
    
    #[cfg(not(unix))]
    {
        Err(KillError::Failed("SIGTERM not supported on this platform".to_string()))
    }
}

/// Kill a process by PID.
pub fn kill_process(pid: u32, opts: &KillOptions) -> Result<(), KillError> {
    #[cfg(unix)]
    {
        use libc::{kill, SIGKILL, SIGTERM};
        
        let pid = pid as libc::pid_t;
        
        // Check if process exists
        let exists = unsafe { kill(pid, 0) };
        if exists != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Err(KillError::ProcessNotFound(pid as u32));
            } else if err.raw_os_error() == Some(libc::EPERM) {
                return Err(KillError::PermissionDenied(pid as u32));
            }
        }
        
        if opts.force {
            // SIGKILL immediately
            let result = unsafe { kill(pid, SIGKILL) };
            if result != 0 {
                let err = std::io::Error::last_os_error();
                return Err(KillError::Failed(err.to_string()));
            }
        } else {
            // SIGTERM first
            let result = unsafe { kill(pid, SIGTERM) };
            if result != 0 {
                let err = std::io::Error::last_os_error();
                return Err(KillError::Failed(err.to_string()));
            }
            
            // Wait for process to die
            let start = Instant::now();
            let timeout = Duration::from_secs(opts.timeout);
            
            loop {
                thread::sleep(Duration::from_millis(100));
                
                // Check if still running
                let still_running = unsafe { kill(pid, 0) } == 0;
                if !still_running {
                    break;
                }
                
                if start.elapsed() > timeout {
                    // Send SIGKILL
                    unsafe { kill(pid, SIGKILL) };
                    thread::sleep(Duration::from_millis(100));
                    
                    // Check again
                    if unsafe { kill(pid, 0) } == 0 {
                        return Err(KillError::Timeout(pid as u32));
                    }
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(not(unix))]
    {
        Err(KillError::Failed("Kill not supported on this platform".to_string()))
    }
}

/// Wait for a port to become free.
pub fn wait_for_port_free(port: u16, timeout_secs: u64) -> Result<(), KillError> {
    let scanner = get_scanner();
    let _opts = ScanOptions::default();
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    
    loop {
        // Check if port is in use
        if let Ok(Some(_)) = scanner.get_port_detail(port, Protocol::Tcp) {
            // Still in use
            if start.elapsed() > timeout {
                return Err(KillError::Timeout(0));
            }
            thread::sleep(Duration::from_millis(100));
        } else {
            // Port is free
            return Ok(());
        }
    }
}

/// Kill process using a specific port.
#[allow(dead_code)]
pub fn kill_port(port: u16, opts: &KillOptions) -> Result<(), KillError> {
    let scanner = get_scanner();
    let _scan_opts = ScanOptions::default();
    
    let entry = scanner.get_port_detail(port, Protocol::Tcp)
        .map_err(|e| KillError::Failed(e.to_string()))?;
    
    match entry {
        Some(entry) => {
            if let Some(pid) = entry.pid {
                kill_process(pid, opts)
            } else {
                Err(KillError::PermissionDenied(0))
            }
        }
        None => Err(KillError::PortNotInUse(port)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kill_options_builder() {
        let opts = KillOptions::new().force(true).timeout(10);
        assert!(opts.force);
        assert_eq!(opts.timeout, 10);
    }
    
    #[test]
    fn test_kill_nonexistent_process() {
        // Use a very high PID that's unlikely to exist
        let result = kill_process(99999999, &KillOptions::new());
        assert!(matches!(result, Err(KillError::ProcessNotFound(_)) | Err(KillError::PermissionDenied(_))));
    }
}
