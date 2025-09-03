#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};

// ===== Shared data shapes returned to the frontend =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub command: Vec<String>,
    pub hash: Option<String>,
    pub is_whitelisted: bool,
    pub is_blacklisted: bool,
    pub matched_rule: Option<String>,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessDetails {
    pub basic_info: ProcessInfo,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
    pub threads_count: u32,
    pub handles_count: u32,
}

// ===== Windows-specific helpers =====
#[cfg(target_os = "windows")]
mod win {
    use super::{ProcessDetails, ProcessInfo};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::*,
        Win32::Foundation::*,
        Win32::System::Diagnostics::ToolHelp::*,
        Win32::System::ProcessStatus::*,
        Win32::System::Threading::*,
        Win32::System::WindowsProgramming::GetProcessHandleCount,
    };

    pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
        let mut items: Vec<ProcessInfo> = Vec::new();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| format!("CreateToolhelp32Snapshot failed: {e}"))?;
            if snapshot.is_invalid() {
                return Err("Invalid snapshot handle".to_string());
            }

            let mut pe32 = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snapshot, &mut pe32).is_ok() {
                loop {
                    let exe = String::from_utf16_lossy(&pe32.szExeFile).trim_end_matches('\0').to_string();
                    let pid = pe32.th32ProcessID;

                    let path = get_process_path(pid);
                    let memory_usage = get_process_memory_usage(pid).unwrap_or(0);

                    items.push(ProcessInfo {
                        pid,
                        name: exe,
                        path,
                        command: Vec::new(),
                        hash: None,
                        is_whitelisted: false,
                        is_blacklisted: false,
                        matched_rule: None,
                        cpu_usage: 0.0,
                        memory_usage,
                        status: "Running".to_string(),
                    });

                    if Process32NextW(snapshot, &mut pe32).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        Ok(items)
    }

    pub fn kill_process(pid: u32) -> Result<bool, String> {
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, false, pid);
            if h.is_invalid() {
                return Err(format!("OpenProcess failed for PID {pid}"));
            }
            let res = TerminateProcess(h, 1);
            let _ = CloseHandle(h);
            if res.is_ok() { Ok(true) } else { Err("TerminateProcess failed".to_string()) }
        }
    }

    pub fn get_details(pid: u32) -> Result<ProcessDetails, String> {
        // Basic info
        let list = list_processes()?;
        let basic = list.into_iter().find(|p| p.pid == pid).ok_or("Process not found")?;

        // Threads count
        let threads = count_threads(pid);

        // Handle count
        let handles = count_handles(pid);

        Ok(ProcessDetails {
            basic_info: basic,
            command_line: None,
            working_directory: None,
            threads_count: threads,
            handles_count: handles,
        })
    }

    pub fn start_process(executable_path: String, arguments: Option<String>) -> Result<u32, String> {
        unsafe {
            let mut si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

            let cmd = if let Some(args) = arguments { format!("\"{executable_path}\" {args}") } else { executable_path };
            let mut wide: Vec<u16> = OsStr::new(&cmd).encode_wide().chain(Some(0)).collect();

            let ok = CreateProcessW(
                PCWSTR::null(),
                PWSTR::from_raw(wide.as_mut_ptr()),
                None,
                None,
                false,
                PROCESS_CREATION_FLAGS(0),
                None,
                PCWSTR::null(),
                &si,
                &mut pi,
            );
            if ok.is_ok() {
                let pid = pi.dwProcessId;
                let _ = CloseHandle(pi.hProcess);
                let _ = CloseHandle(pi.hThread);
                Ok(pid)
            } else {
                Err("CreateProcessW failed".to_string())
            }
        }
    }

    fn get_process_path(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
            if h.is_invalid() {
                return None;
            }
            let mut buf = [0u16; 260];
            let len = GetModuleFileNameExW(h, HMODULE::default(), &mut buf);
            let _ = CloseHandle(h);
            if len > 0 { Some(String::from_utf16_lossy(&buf[..len as usize])) } else { None }
        }
    }

    fn get_process_memory_usage(pid: u32) -> Option<u64> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
            if h.is_invalid() {
                return None;
            }
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(h, &mut counters, counters.cb);
            let _ = CloseHandle(h);
            if ok.is_ok() { Some(counters.WorkingSetSize as u64) } else { None }
        }
    }

    fn count_threads(pid: u32) -> u32 {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap.is_invalid() {
                return 0;
            }
            let mut te = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
            let mut count = 0;
            if Thread32First(snap, &mut te).is_ok() {
                loop {
                    if te.th32OwnerProcessID == pid { count += 1; }
                    if Thread32Next(snap, &mut te).is_err() { break; }
                }
            }
            let _ = CloseHandle(snap);
            count
        }
    }

    fn count_handles(pid: u32) -> u32 {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid);
            if h.is_invalid() {
                return 0;
            }
            let mut count: u32 = 0;
            let ok = GetProcessHandleCount(h, &mut count);
            let _ = CloseHandle(h);
            if ok.is_ok() { count } else { 0 }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win {
    use super::{ProcessDetails, ProcessInfo};
    pub fn list_processes() -> Result<Vec<ProcessInfo>, String> { Err("Windows-only".into()) }
    pub fn kill_process(_pid: u32) -> Result<bool, String> { Err("Windows-only".into()) }
    pub fn get_details(_pid: u32) -> Result<ProcessDetails, String> { Err("Windows-only".into()) }
    pub fn start_process(_p: String, _a: Option<String>) -> Result<u32, String> { Err("Windows-only".into()) }
}

// ===== Tauri commands =====
#[tauri::command]
pub fn get_all_processes() -> Result<Vec<ProcessInfo>, String> {
    win::list_processes()
}

// For compatibility with some frontend calls that use a different name.
#[tauri::command]
pub fn get_process_list() -> Result<Vec<ProcessInfo>, String> {
    win::list_processes()
}

#[tauri::command]
pub fn get_process_details(pid: u32) -> Result<ProcessDetails, String> {
    win::get_details(pid)
}

#[tauri::command]
pub fn kill_process(pid: u32, _name: Option<String>) -> Result<bool, String> {
    win::kill_process(pid)
}

#[tauri::command]
pub fn start_process(executable_path: String, arguments: Option<String>) -> Result<u32, String> {
    win::start_process(executable_path, arguments)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_all_processes,
            get_process_list,
            get_process_details,
            kill_process,
            start_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
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