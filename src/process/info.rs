//! Process information retrieval.

use sysinfo::{Pid, System};
use chrono::{DateTime, Utc};

/// Process information.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub user: Option<String>,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub started_at: Option<DateTime<Utc>>,
    pub parent_pid: Option<u32>,
    pub parent_name: Option<String>,
}

/// Get detailed process info.
pub fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let pid = Pid::from_u32(pid);
    let process = system.process(pid)?;
    
    let parent_info = process.parent().and_then(|ppid| {
        system.process(ppid).map(|p| (ppid.as_u32(), p.name().to_string_lossy().to_string()))
    });
    
    let cmd_parts: Vec<String> = process.cmd()
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect();
    
    Some(ProcessInfo {
        pid: pid.as_u32(),
        name: process.name().to_string_lossy().to_string(),
        command: cmd_parts.join(" "),
        user: None, // sysinfo doesn't expose user directly on all platforms
        memory_bytes: process.memory(),
        cpu_percent: process.cpu_usage(),
        started_at: Some(DateTime::from_timestamp(process.start_time() as i64, 0)?),
        parent_pid: parent_info.as_ref().map(|(pid, _)| *pid),
        parent_name: parent_info.map(|(_, name)| name),
    })
}
