#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::{CloseHandle, HMODULE},
    Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
        PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    },
    Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    Win32::System::ProcessStatus::{GetModuleFileNameExW, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub executable_path: Option<String>,
    pub memory_usage: u64,
    pub cpu_usage: f32,
    pub status: String,
    pub parent_pid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessDetails {
    pub basic_info: ProcessInfo,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
    pub threads_count: u32,
    pub handles_count: u32,
}

#[tauri::command]
fn get_process_list() -> Vec<ProcessInfo> {
    #[cfg(target_os = "windows")]
    {
        get_windows_processes()
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![]
    }
}

#[cfg(target_os = "windows")]
fn get_windows_processes() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();
    
    unsafe {
        match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(snapshot) => {
                let mut pe = PROCESSENTRY32W {
                    dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                    ..Default::default()
                };
                
                if Process32FirstW(snapshot, &mut pe).is_ok() {
                    loop {
                        let name = String::from_utf16_lossy(&pe.szExeFile)
                            .trim_end_matches('\0')
                            .to_string();
                        
                        processes.push(ProcessInfo {
                            pid: pe.th32ProcessID,
                            name,
                            executable_path: get_process_path(pe.th32ProcessID),
                            memory_usage: get_process_memory(pe.th32ProcessID),
                            cpu_usage: 0.0,
                            status: "Running".to_string(),
                            parent_pid: pe.th32ParentProcessID,
                        });
                        
                        if Process32NextW(snapshot, &mut pe).is_err() {
                            break;
                        }
                    }
                }
                let _ = CloseHandle(snapshot);
            }
            Err(_) => {}
        }
    }
    
    processes
}

#[cfg(target_os = "windows")]
fn get_process_path(pid: u32) -> Option<String> {
    unsafe {
        match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            Ok(handle) => {
                let mut buf = [0u16; 260];
                let len = GetModuleFileNameExW(handle, HMODULE::default(), &mut buf);
                let _ = CloseHandle(handle);
                if len > 0 {
                    Some(String::from_utf16_lossy(&buf[..len as usize]))
                } else {
                    None
                }
            }
            Err(_) => None
        }
    }
}

#[cfg(target_os = "windows")]
fn get_process_memory(pid: u32) -> u64 {
    unsafe {
        match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            Ok(handle) => {
                let mut counters = PROCESS_MEMORY_COUNTERS::default();
                counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                let result = GetProcessMemoryInfo(handle, &mut counters, counters.cb);
                let _ = CloseHandle(handle);
                if result.is_ok() {
                    counters.WorkingSetSize as u64
                } else {
                    0
                }
            }
            Err(_) => 0
        }
    }
}

#[tauri::command]
fn get_process_details(pid: u32) -> Result<ProcessDetails, String> {
    let processes = get_process_list();
    let basic_info = processes
        .into_iter()
        .find(|p| p.pid == pid)
        .ok_or("Process not found")?;
    
    Ok(ProcessDetails {
        basic_info,
        command_line: None,
        working_directory: None,
        threads_count: 0,
        handles_count: 0,
    })
}

#[tauri::command]
fn kill_process(pid: u32) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            match OpenProcess(PROCESS_TERMINATE, false, pid) {
                Ok(handle) => {
                    let result = TerminateProcess(handle, 1);
                    let _ = CloseHandle(handle);
                    match result {
                        Ok(_) => Ok(true),
                        Err(e) => Err(format!("Failed to terminate: {}", e))
                    }
                }
                Err(e) => Err(format!("Failed to open process: {}", e))
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(true)
    }
}

#[tauri::command]
fn start_process(executable_path: String, _arguments: Option<String>) -> Result<u32, String> {
    println!("Mock: Starting {}", executable_path);
    Ok(12345)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_process_list,
            get_process_details,
            kill_process,
            start_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}