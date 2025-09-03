// src/main.rs
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// Commands are defined in this file below; no external module import needed.

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_all_processes,
            kill_process,
            get_process_details,
            start_process
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}

// src/process_manager.rs
use windows::{
    core::*,
    Win32::System::Diagnostics::ToolHelp::*,
    Win32::Foundation::*,
    Win32::System::Threading::*,
    Win32::System::ProcessStatus::*,
    Win32::System::SystemServices::*,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessDetails {
    pub basic_info: ProcessInfo,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
    pub threads_count: u32,
    pub handles_count: u32,
}

#[tauri::command]
pub fn get_all_processes() -> Result<Vec<ProcessInfo>, String> {
    let mut processes = Vec::new();
    
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| format!("Snapshot oluşturulamadı: {}", e))?;
        
        if snapshot.is_invalid() {
            return Err("Geçersiz snapshot handle".to_string());
        }
        
        let mut process_entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        
        if Process32FirstW(snapshot, &mut process_entry).is_ok() {
            loop {
                let process_name = String::from_utf16_lossy(&process_entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_string();
                
                let executable_path = get_process_path(process_entry.th32ProcessID);
                let memory_usage = get_process_memory_usage(process_entry.th32ProcessID);
                
                processes.push(ProcessInfo {
                    pid: process_entry.th32ProcessID,
                    name: process_name,
                    executable_path,
                    memory_usage: memory_usage.unwrap_or(0),
                    cpu_usage: 0.0, // CPU kullanımı için ayrı hesaplama gerekli
                    status: "Running".to_string(),
                    parent_pid: process_entry.th32ParentProcessID,
                });
                
                if Process32NextW(snapshot, &mut process_entry).is_err() {
                    break;
                }
            }
        }
        
        let _ = CloseHandle(snapshot);
    }
    
    Ok(processes)
}

#[tauri::command]
pub fn kill_process(pid: u32) -> Result<bool, String> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid);
        
        if handle.is_invalid() {
            return Err(format!("Process açılamadı: PID {}", pid));
        }
        
        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);
        
        if result.is_ok() {
            Ok(true)
        } else {
            Err("Process sonlandırılamadı".to_string())
        }
    }
}

#[tauri::command]
pub fn get_process_details(pid: u32) -> Result<ProcessDetails, String> {
    let processes = get_all_processes()?;
    let basic_info = processes
        .into_iter()
        .find(|p| p.pid == pid)
        .ok_or("Process bulunamadı")?;
    
    Ok(ProcessDetails {
        basic_info,
        command_line: get_process_command_line(pid),
        working_directory: get_process_working_directory(pid),
        threads_count: get_process_thread_count(pid),
        handles_count: get_process_handle_count(pid),
    })
}

#[tauri::command]
pub fn start_process(executable_path: String, arguments: Option<String>) -> Result<u32, String> {
    let mut startup_info = STARTUPINFOW::default();
    let mut process_info = PROCESS_INFORMATION::default();
    
    startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    
    let command_line = if let Some(args) = arguments {
        format!("\"{}\" {}", executable_path, args)
    } else {
        executable_path.clone()
    };
    
    let command_line_wide: Vec<u16> = OsStr::new(&command_line)
        .encode_wide()
        .chain(Some(0))
        .collect();
    
    unsafe {
        let result = CreateProcessW(
            PCWSTR::null(),
            PWSTR::from_raw(command_line_wide.as_ptr() as *mut u16),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            &startup_info,
            &mut process_info,
        );
        
        if result.is_ok() {
            let pid = process_info.dwProcessId;
            let _ = CloseHandle(process_info.hProcess);
            let _ = CloseHandle(process_info.hThread);
            Ok(pid)
        } else {
            Err("Process başlatılamadı".to_string())
        }
    }
}

fn get_process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid
        );
        
        if handle.is_invalid() {
            return None;
        }
        
        let mut path_buffer = [0u16; 260];
        let result = GetModuleFileNameExW(
            handle,
            HMODULE::default(),
            &mut path_buffer,
        );
        
        let _ = CloseHandle(handle);
        
        if result > 0 {
            Some(String::from_utf16_lossy(&path_buffer[..result as usize]))
        } else {
            None
        }
    }
}

fn get_process_memory_usage(pid: u32) -> Option<u64> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
        
        if handle.is_invalid() {
            return None;
        }
        
        let mut mem_counters = PROCESS_MEMORY_COUNTERS::default();
        mem_counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        
        let result = GetProcessMemoryInfo(handle, &mut mem_counters, mem_counters.cb);
        let _ = CloseHandle(handle);
        
        if result.is_ok() {
            Some(mem_counters.WorkingSetSize as u64)
        } else {
            None
        }
    }
}

fn get_process_command_line(_pid: u32) -> Option<String> {
    // WMI veya Registry üzerinden command line bilgisi alınabilir
    // Bu implementation basit tutuldu
    None
}

fn get_process_working_directory(_pid: u32) -> Option<String> {
    // Process working directory bilgisi
    None
}

fn get_process_thread_count(pid: u32) -> u32 {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot.is_invalid() {
            return 0;
        }
        
        let mut thread_entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        
        let mut count = 0;
        if Thread32First(snapshot, &mut thread_entry).is_ok() {
            loop {
                if thread_entry.th32OwnerProcessID == pid {
                    count += 1;
                }
                
                if Thread32Next(snapshot, &mut thread_entry).is_err() {
                    break;
                }
            }
        }
        
        let _ = CloseHandle(snapshot);
        count
    }
}

fn get_process_handle_count(_pid: u32) -> u32 {
    // Handle count için GetProcessHandleCount API kullanılabilir
    0
}