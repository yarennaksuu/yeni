use crate::edr::config::AppConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use sysinfo::{System, Pid};
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub cmd_line: Option<String>,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub file_hash: Option<String>,
    pub digital_signature: Option<SignatureInfo>,
    pub network_connections: Vec<NetworkConnection>,
    pub file_handles: Vec<String>,
    pub registry_keys: Vec<String>,
    pub user: Option<String>,
    pub integrity_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub is_signed: bool,
    pub is_valid: bool,
    pub issuer: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub protocol: String,
    pub state: String,
}

pub struct ProcessScanner {
    system: System,
    config: AppConfig,
    hash_cache: HashMap<String, String>,
}

impl ProcessScanner {
    pub fn new(config: &AppConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            config: config.clone(),
            hash_cache: HashMap::new(),
        }
    }
    
    pub fn scan_processes(&mut self) -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>> {
        debug!("Starting process scan");
        self.system.refresh_processes();
        
        let mut processes = Vec::new();
        
        for (pid, process) in self.system.processes() {
            let process_info = self.get_process_info(pid.as_u32(), process);
            processes.push(process_info);
        }
        
        info!("Scanned {} processes", processes.len());
        Ok(processes)
    }
    
    pub fn get_process_by_pid(&mut self, pid: u32) -> Option<ProcessInfo> {
        self.system.refresh_processes();
        
        if let Some(process) = self.system.process(Pid::from(pid as usize)) {
            Some(self.get_process_info(pid, process))
        } else {
            None
        }
    }
    
    fn get_process_info(&mut self, pid: u32, process: &sysinfo::Process) -> ProcessInfo {
        let exe_path = process.exe().and_then(|p| p.to_str()).map(|s| s.to_string());
        let cmd_line = if process.cmd().is_empty() {
            None
        } else {
            Some(process.cmd().join(" "))
        };
        
        // Calculate file hash
        let file_hash = if let Some(ref path) = exe_path {
            self.calculate_file_hash(path)
        } else {
            None
        };
        
        // Get digital signature info
        let digital_signature = if let Some(ref path) = exe_path {
            self.get_signature_info(path)
        } else {
            None
        };
        
        // Get network connections
        let network_connections = self.get_network_connections(pid);
        
        // Get file handles
        let file_handles = self.get_file_handles(pid);
        
        // Get registry keys
        let registry_keys = self.get_registry_keys(pid);
        
        // Get user info
        let user = self.get_process_user(pid);
        
        // Get integrity level
        let integrity_level = self.get_integrity_level(pid);
        
        ProcessInfo {
            pid,
            name: process.name().to_string(),
            exe_path,
            cmd_line,
            parent_pid: process.parent().map(|p| p.as_u32()),
            start_time: process.start_time(),
            cpu_usage: process.cpu_usage(),
            memory_usage: process.memory(),
            file_hash,
            digital_signature,
            network_connections,
            file_handles,
            registry_keys,
            user,
            integrity_level,
        }
    }
    
    fn calculate_file_hash(&mut self, file_path: &str) -> Option<String> {
        // Check cache first
        if let Some(cached_hash) = self.hash_cache.get(file_path) {
            return Some(cached_hash.clone());
        }
        
        match fs::read(file_path) {
            Ok(data) => {
                let mut hasher = Sha256::new();
                hasher.update(&data);
                let hash = format!("{:x}", hasher.finalize());
                
                // Cache the hash
                self.hash_cache.insert(file_path.to_string(), hash.clone());
                Some(hash)
            }
            Err(e) => {
                debug!("Failed to calculate hash for {}: {}", file_path, e);
                None
            }
        }
    }
    
    fn get_signature_info(&self, file_path: &str) -> Option<SignatureInfo> {
        #[cfg(windows)]
        {
            self.get_windows_signature_info(file_path)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
    
    #[cfg(windows)]
    fn get_windows_signature_info(&self, file_path: &str) -> Option<SignatureInfo> {
        use windows::Win32::Security::Cryptography::{
            CryptQueryObject, CERT_QUERY_OBJECT_FILE, CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
            CERT_QUERY_FORMAT_FLAG_BINARY, CERT_QUERY_CONTEXT_FLAG_ALL,
        };
        use windows::core::PCWSTR;
        
        unsafe {
            let wide_path: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
            let mut cert_context = std::ptr::null_mut();
            let mut msg_context = std::ptr::null_mut();
            
            let result = CryptQueryObject(
                CERT_QUERY_OBJECT_FILE.0,
                PCWSTR(wide_path.as_ptr()),
                CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                Some(&mut msg_context),
                Some(&mut cert_context),
            );
            
            if result.is_ok() && !cert_context.is_null() {
                Some(SignatureInfo {
                    is_signed: true,
                    is_valid: true, // Simplified validation
                    issuer: Some("Microsoft".to_string()), // Simplified
                    subject: Some("Unknown".to_string()), // Simplified
                })
            } else {
                Some(SignatureInfo {
                    is_signed: false,
                    is_valid: false,
                    issuer: None,
                    subject: None,
                })
            }
        }
    }
    
    fn get_network_connections(&self, pid: u32) -> Vec<NetworkConnection> {
        // Simplified implementation - would need platform-specific code
        // for actual network connection enumeration
        Vec::new()
    }
    
    fn get_file_handles(&self, pid: u32) -> Vec<String> {
        // Simplified implementation - would need platform-specific code
        // for actual file handle enumeration
        Vec::new()
    }
    
    fn get_registry_keys(&self, pid: u32) -> Vec<String> {
        // Simplified implementation - would need platform-specific code
        // for actual registry key enumeration
        Vec::new()
    }
    
    fn get_process_user(&self, pid: u32) -> Option<String> {
        #[cfg(windows)]
        {
            self.get_windows_process_user(pid)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
    
    #[cfg(windows)]
    fn get_windows_process_user(&self, pid: u32) -> Option<String> {
        use windows::Win32::Foundation::{HANDLE, CloseHandle};
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};
        use windows::Win32::Security::{
            OpenProcessToken, GetTokenInformation, TokenUser, TOKEN_QUERY,
        };
        
        unsafe {
            let process_handle = match OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
                Ok(handle) => handle,
                Err(_) => return None,
            };
            
            let mut token_handle = HANDLE::default();
            if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle).is_err() {
                let _ = CloseHandle(process_handle);
                return None;
            }
            
            // Simplified - would need to actually get user info
            let _ = CloseHandle(token_handle);
            let _ = CloseHandle(process_handle);
            
            Some("SYSTEM".to_string()) // Simplified
        }
    }
    
    fn get_integrity_level(&self, pid: u32) -> Option<String> {
        #[cfg(windows)]
        {
            Some("Medium".to_string()) // Simplified
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
    
    pub fn monitor_process_creation(&mut self) -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>> {
        // This would implement real-time process monitoring
        // For now, just return current processes
        self.scan_processes()
    }
    
    pub fn get_process_tree(&mut self, root_pid: u32) -> Result<ProcessTree, Box<dyn std::error::Error>> {
        let processes = self.scan_processes()?;
        let tree = ProcessTree::build_tree(&processes, root_pid);
        Ok(tree)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTree {
    pub root: ProcessInfo,
    pub children: Vec<ProcessTree>,
}

impl ProcessTree {
    pub fn build_tree(processes: &[ProcessInfo], root_pid: u32) -> ProcessTree {
        let root = processes.iter()
            .find(|p| p.pid == root_pid)
            .cloned()
            .unwrap_or_else(|| ProcessInfo {
                pid: root_pid,
                name: "Unknown".to_string(),
                exe_path: None,
                cmd_line: None,
                parent_pid: None,
                start_time: 0,
                cpu_usage: 0.0,
                memory_usage: 0,
                file_hash: None,
                digital_signature: None,
                network_connections: Vec::new(),
                file_handles: Vec::new(),
                registry_keys: Vec::new(),
                user: None,
                integrity_level: None,
            });
        
        let children: Vec<ProcessTree> = processes.iter()
            .filter(|p| p.parent_pid == Some(root_pid))
            .map(|p| Self::build_tree(processes, p.pid))
            .collect();
        
        ProcessTree { root, children }
    }
    
    pub fn find_process(&self, pid: u32) -> Option<&ProcessInfo> {
        if self.root.pid == pid {
            Some(&self.root)
        } else {
            self.children.iter()
                .find_map(|child| child.find_process(pid))
        }
    }
    
    pub fn get_all_pids(&self) -> Vec<u32> {
        let mut pids = vec![self.root.pid];
        for child in &self.children {
            pids.extend(child.get_all_pids());
        }
        pids
    }
}