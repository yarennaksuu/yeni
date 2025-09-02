use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub blacklist: Vec<Rule>,
    pub whitelist: Vec<Rule>,
    pub scanning: ScanningConfig,
    pub kill_policy: KillPolicy,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub rule_type: RuleType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleType {
    #[serde(rename = "name")]
    Name { value: String },
    #[serde(rename = "path")]
    Path { value: String },
    #[serde(rename = "hash")]
    Hash { sha256: String },
    #[serde(rename = "command")]
    Command { pattern: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanningConfig {
    pub scan_interval_ms: u64,
    pub enable_hash_check: bool,
    pub enable_command_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillPolicy {
    pub graceful_kill: bool,
    pub force_kill_timeout_ms: u64,
    pub cooldown_ms: u64,
    pub max_retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub file_path: String,
    pub format: LogFormat,
    pub rotation_size_mb: u64,
    pub max_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "text")]
    Text,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        // Self-protection: kendi processimizi whitelist'e ekle
        if let Ok(current_exe) = std::env::current_exe() {
            let exe_name = current_exe
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("edr-kill-switch.exe")
                .to_string();

            config.whitelist.push(Rule {
                id: "self_protection".to_string(),
                rule_type: RuleType::Name { value: exe_name },
                description: Some("Self protection rule".to_string()),
            });
        }

        // Windows kritik sistem süreçlerini whitelist'e ekle
        #[cfg(target_os = "windows")]
        {
            let system_processes = vec![
                "System", "smss.exe", "csrss.exe", "wininit.exe",
                "services.exe", "lsass.exe", "winlogon.exe", "dwm.exe",
            ];

            for proc_name in system_processes {
                config.whitelist.push(Rule {
                    id: format!("system_{}", proc_name),
                    rule_type: RuleType::Name { value: proc_name.to_string() },
                    description: Some(format!("Critical system process: {}", proc_name)),
                });
            }
        }

        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.blacklist.is_empty() && self.whitelist.is_empty() {
            return Err("Configuration must contain at least one rule".to_string());
        }

        if self.scanning.scan_interval_ms < 100 {
            return Err("Scan interval must be at least 100ms".to_string());
        }

        if self.kill_policy.force_kill_timeout_ms < 100 {
            return Err("Force kill timeout must be at least 100ms".to_string());
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            blacklist: vec![],
            whitelist: vec![],
            scanning: ScanningConfig {
                scan_interval_ms: 2000,
                enable_hash_check: false,
                enable_command_check: true,
            },
            kill_policy: KillPolicy {
                graceful_kill: true,
                force_kill_timeout_ms: 5000,
                cooldown_ms: 1000,
                max_retry_attempts: 3,
            },
            logging: LoggingConfig {
                file_path: "edr_kill_switch.log".to_string(),
                format: LogFormat::Json,
                rotation_size_mb: 10,
                max_files: 5,
            },
        }
    }
}
