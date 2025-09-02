use crate::edr::config::{LoggingConfig, LogFormat};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Mutex,
};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum LogEvent {
    ProcessKilled {
        pid: u32,
        timestamp: DateTime<Utc>,
        success: bool,
        reason: String,
    },
    ThreatDetected {
        pid: u32,
        process_name: String,
        threat_type: String,
        timestamp: DateTime<Utc>,
        action_taken: String,
    },
    ScanStarted {
        timestamp: DateTime<Utc>,
        scan_type: String,
    },
    ScanCompleted {
        timestamp: DateTime<Utc>,
        processes_scanned: u32,
        threats_found: u32,
        duration_ms: u64,
    },
    ConfigurationChanged {
        timestamp: DateTime<Utc>,
        changed_by: String,
        changes: Vec<String>,
    },
    SystemEvent {
        timestamp: DateTime<Utc>,
        event_type: String,
        message: String,
        severity: String,
    },
    ApplicationStarted {
        timestamp: DateTime<Utc>,
        version: String,
        is_admin: bool,
    },
    ApplicationStopped {
        timestamp: DateTime<Utc>,
        reason: String,
    },
}

pub struct Logger {
    config: LoggingConfig,
    log_file: Option<BufWriter<File>>,
    in_memory_logs: Mutex<VecDeque<LogEvent>>,
    max_memory_logs: usize,
}

impl Logger {
    pub fn new(config: &LoggingConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let log_file = if config.enabled && config.log_file.is_some() {
            let log_path = config.log_file.as_ref().unwrap();
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)?;
            Some(BufWriter::new(file))
        } else {
            None
        };
        
        Ok(Self {
            config: config.clone(),
            log_file,
            in_memory_logs: Mutex::new(VecDeque::new()),
            max_memory_logs: 1000, // Keep last 1000 events in memory
        })
    }
    
    pub fn log_event(&mut self, event: &LogEvent) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Add to memory storage
        {
            let mut logs = self.in_memory_logs.lock().unwrap();
            logs.push_back(event.clone());
            
            // Maintain maximum memory log count
            while logs.len() > self.max_memory_logs {
                logs.pop_front();
            }
        }
        
        // Write to file if configured
        if let Some(ref mut file) = self.log_file {
            let log_line = self.format_log_event(event)?;
            writeln!(file, "{}", log_line)?;
            file.flush()?;
        }
        
        Ok(())
    }
    
    fn format_log_event(&self, event: &LogEvent) -> Result<String, Box<dyn std::error::Error>> {
        match self.config.format {
            LogFormat::Json => {
                Ok(serde_json::to_string(event)?)
            }
            LogFormat::Text => {
                Ok(self.format_text_log(event))
            }
            LogFormat::Compact => {
                Ok(self.format_compact_log(event))
            }
        }
    }
    
    fn format_text_log(&self, event: &LogEvent) -> String {
        match event {
            LogEvent::ProcessKilled { pid, timestamp, success, reason } => {
                format!("[{}] PROCESS_KILLED - PID: {}, Success: {}, Reason: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), pid, success, reason)
            }
            LogEvent::ThreatDetected { pid, process_name, threat_type, timestamp, action_taken } => {
                format!("[{}] THREAT_DETECTED - PID: {}, Process: {}, Type: {}, Action: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), pid, process_name, threat_type, action_taken)
            }
            LogEvent::ScanStarted { timestamp, scan_type } => {
                format!("[{}] SCAN_STARTED - Type: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), scan_type)
            }
            LogEvent::ScanCompleted { timestamp, processes_scanned, threats_found, duration_ms } => {
                format!("[{}] SCAN_COMPLETED - Scanned: {}, Threats: {}, Duration: {}ms", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), processes_scanned, threats_found, duration_ms)
            }
            LogEvent::ConfigurationChanged { timestamp, changed_by, changes } => {
                format!("[{}] CONFIG_CHANGED - By: {}, Changes: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), changed_by, changes.join(", "))
            }
            LogEvent::SystemEvent { timestamp, event_type, message, severity } => {
                format!("[{}] SYSTEM_EVENT - Type: {}, Severity: {}, Message: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), event_type, severity, message)
            }
            LogEvent::ApplicationStarted { timestamp, version, is_admin } => {
                format!("[{}] APP_STARTED - Version: {}, Admin: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), version, is_admin)
            }
            LogEvent::ApplicationStopped { timestamp, reason } => {
                format!("[{}] APP_STOPPED - Reason: {}", 
                       timestamp.format("%Y-%m-%d %H:%M:%S UTC"), reason)
            }
        }
    }
    
    fn format_compact_log(&self, event: &LogEvent) -> String {
        match event {
            LogEvent::ProcessKilled { pid, timestamp, success, .. } => {
                format!("{} KILL {} {}", 
                       timestamp.format("%H:%M:%S"), pid, if *success { "OK" } else { "FAIL" })
            }
            LogEvent::ThreatDetected { pid, threat_type, timestamp, .. } => {
                format!("{} THREAT {} {}", 
                       timestamp.format("%H:%M:%S"), pid, threat_type)
            }
            LogEvent::ScanStarted { timestamp, scan_type } => {
                format!("{} SCAN_START {}", 
                       timestamp.format("%H:%M:%S"), scan_type)
            }
            LogEvent::ScanCompleted { timestamp, processes_scanned, threats_found, duration_ms } => {
                format!("{} SCAN_END {}/{} {}ms", 
                       timestamp.format("%H:%M:%S"), threats_found, processes_scanned, duration_ms)
            }
            LogEvent::ConfigurationChanged { timestamp, changed_by, .. } => {
                format!("{} CONFIG {}", 
                       timestamp.format("%H:%M:%S"), changed_by)
            }
            LogEvent::SystemEvent { timestamp, event_type, severity, .. } => {
                format!("{} SYS {} {}", 
                       timestamp.format("%H:%M:%S"), event_type, severity)
            }
            LogEvent::ApplicationStarted { timestamp, version, is_admin } => {
                format!("{} START {} {}", 
                       timestamp.format("%H:%M:%S"), version, if *is_admin { "ADMIN" } else { "USER" })
            }
            LogEvent::ApplicationStopped { timestamp, .. } => {
                format!("{} STOP", 
                       timestamp.format("%H:%M:%S"))
            }
        }
    }
    
    pub fn get_recent_logs(&self, limit: u32) -> Result<Vec<LogEvent>, Box<dyn std::error::Error>> {
        let logs = self.in_memory_logs.lock().unwrap();
        let start_index = if logs.len() > limit as usize {
            logs.len() - limit as usize
        } else {
            0
        };
        
        Ok(logs.range(start_index..).cloned().collect())
    }
    
    pub fn get_logs_by_type(&self, event_type: &str, limit: u32) -> Result<Vec<LogEvent>, Box<dyn std::error::Error>> {
        let logs = self.in_memory_logs.lock().unwrap();
        let filtered: Vec<LogEvent> = logs
            .iter()
            .filter(|log| self.matches_event_type(log, event_type))
            .rev()
            .take(limit as usize)
            .cloned()
            .collect();
        
        Ok(filtered)
    }
    
    fn matches_event_type(&self, event: &LogEvent, event_type: &str) -> bool {
        match (event, event_type.to_lowercase().as_str()) {
            (LogEvent::ProcessKilled { .. }, "process_killed" | "kill") => true,
            (LogEvent::ThreatDetected { .. }, "threat_detected" | "threat") => true,
            (LogEvent::ScanStarted { .. }, "scan_started" | "scan_start") => true,
            (LogEvent::ScanCompleted { .. }, "scan_completed" | "scan_end") => true,
            (LogEvent::ConfigurationChanged { .. }, "configuration_changed" | "config") => true,
            (LogEvent::SystemEvent { .. }, "system_event" | "system") => true,
            (LogEvent::ApplicationStarted { .. }, "application_started" | "start") => true,
            (LogEvent::ApplicationStopped { .. }, "application_stopped" | "stop") => true,
            _ => false,
        }
    }
    
    pub fn get_logs_by_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>
    ) -> Result<Vec<LogEvent>, Box<dyn std::error::Error>> {
        let logs = self.in_memory_logs.lock().unwrap();
        let filtered: Vec<LogEvent> = logs
            .iter()
            .filter(|log| {
                let timestamp = self.get_event_timestamp(log);
                timestamp >= start_time && timestamp <= end_time
            })
            .cloned()
            .collect();
        
        Ok(filtered)
    }
    
    fn get_event_timestamp(&self, event: &LogEvent) -> DateTime<Utc> {
        match event {
            LogEvent::ProcessKilled { timestamp, .. } |
            LogEvent::ThreatDetected { timestamp, .. } |
            LogEvent::ScanStarted { timestamp, .. } |
            LogEvent::ScanCompleted { timestamp, .. } |
            LogEvent::ConfigurationChanged { timestamp, .. } |
            LogEvent::SystemEvent { timestamp, .. } |
            LogEvent::ApplicationStarted { timestamp, .. } |
            LogEvent::ApplicationStopped { timestamp, .. } => *timestamp,
        }
    }
    
    pub fn clear_logs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut logs = self.in_memory_logs.lock().unwrap();
        logs.clear();
        
        // Also clear the log file if it exists
        if let Some(ref log_file_path) = self.config.log_file {
            if let Err(e) = std::fs::write(log_file_path, "") {
                error!("Failed to clear log file: {}", e);
            }
        }
        
        Ok(())
    }
    
    pub fn export_logs(&self, export_path: &str, format: &LogFormat) -> Result<(), Box<dyn std::error::Error>> {
        let logs = self.in_memory_logs.lock().unwrap();
        let mut file = File::create(export_path)?;
        
        match format {
            LogFormat::Json => {
                let json_logs = serde_json::to_string_pretty(&*logs)?;
                write!(file, "{}", json_logs)?;
            }
            LogFormat::Text | LogFormat::Compact => {
                for log in logs.iter() {
                    let line = match format {
                        LogFormat::Text => self.format_text_log(log),
                        LogFormat::Compact => self.format_compact_log(log),
                        _ => unreachable!(),
                    };
                    writeln!(file, "{}", line)?;
                }
            }
        }
        
        file.flush()?;
        Ok(())
    }
    
    pub fn get_log_stats(&self) -> LogStats {
        let logs = self.in_memory_logs.lock().unwrap();
        let mut stats = LogStats::default();
        
        for log in logs.iter() {
            match log {
                LogEvent::ProcessKilled { success, .. } => {
                    stats.processes_killed += 1;
                    if *success {
                        stats.successful_kills += 1;
                    }
                }
                LogEvent::ThreatDetected { .. } => {
                    stats.threats_detected += 1;
                }
                LogEvent::ScanCompleted { .. } => {
                    stats.scans_performed += 1;
                }
                LogEvent::ConfigurationChanged { .. } => {
                    stats.config_changes += 1;
                }
                _ => {}
            }
        }
        
        stats
    }
}

#[derive(Debug, Default)]
pub struct LogStats {
    pub processes_killed: u32,
    pub successful_kills: u32,
    pub threats_detected: u32,
    pub scans_performed: u32,
    pub config_changes: u32,
}canStarted { .. } => {
                    stats.scans_performed += 1;
                }
                LogEvent::S