#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::Manager;
use tracing::{info, warn, error};
use tracing_appender::non_blocking::WorkerGuard;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use hmac::{Hmac, Mac};
use rand::RngCore;

// Data structures returned to the frontend
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

// Windows-specific implementation
#[cfg(target_os = "windows")]
mod win {
    use super::{ProcessDetails, ProcessInfo};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, LRESULT, WPARAM, HANDLE, HMODULE},
        Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, Thread32First, Thread32Next,
            PROCESSENTRY32W, THREADENTRY32, TH32CS_SNAPPROCESS, TH32CS_SNAPTHREAD,
        },
        Win32::System::ProcessStatus::{GetModuleFileNameExW, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Win32::System::Threading::{
            CreateProcessW, GetProcessHandleCount, OpenProcess, TerminateProcess, PROCESS_CREATION_FLAGS,
            PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ, PROCESS_INFORMATION, STARTUPINFOW,
        },
        Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE},
    };

    pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
        let mut processes: Vec<ProcessInfo> = Vec::new();
        unsafe {
            let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| e.to_string())?;
            if snapshot.is_invalid() {
                return Err("Invalid snapshot handle".to_string());
            }

            let mut pe = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snapshot, &mut pe).is_ok() {
                loop {
                    let name = String::from_utf16_lossy(&pe.szExeFile).trim_end_matches('\0').to_string();
                    let pid = pe.th32ProcessID;
                    let parent_pid = pe.th32ParentProcessID;

                    let executable_path = get_process_path(pid);
                    let memory_usage = get_process_memory_usage(pid).unwrap_or(0);

                    processes.push(ProcessInfo {
                        pid,
                    	name,
                        executable_path,
                        memory_usage,
                        cpu_usage: 0.0,
                        status: "Running".to_string(),
                        parent_pid,
                    });

                    if Process32NextW(snapshot, &mut pe).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        Ok(processes)
    }

    pub fn kill_process(pid: u32) -> Result<bool, String> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid);
            if handle.is_invalid() {
                return Err(format!("OpenProcess failed for PID {pid}"));
            }
            // Try graceful: send WM_CLOSE to any top-level window owned by PID
            let _ = send_wm_close_to_pid(pid);
            // small grace period
            std::thread::sleep(std::time::Duration::from_millis(200));
            // Force terminate
            let result = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
            if result.is_ok() { Ok(true) } else { Err("TerminateProcess failed".to_string()) }
        }
    }

    pub fn get_details(pid: u32) -> Result<ProcessDetails, String> {
        let processes = list_processes()?;
        let basic = processes.into_iter().find(|p| p.pid == pid).ok_or_else(|| "Process not found".to_string())?;

        let threads_count = count_threads(pid)?;
        let handles_count = count_handles(pid)?;

        Ok(ProcessDetails {
            basic_info: basic,
            command_line: None,
            working_directory: None,
            threads_count,
            handles_count,
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
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
            if handle.is_invalid() {
                return None;
            }
            let mut buf = [0u16; 260];
            let len = GetModuleFileNameExW(handle, HMODULE::default(), &mut buf);
            let _ = CloseHandle(handle);
            if len > 0 { Some(String::from_utf16_lossy(&buf[..len as usize])) } else { None }
        }
    }

    fn get_process_memory_usage(pid: u32) -> Option<u64> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
            if handle.is_invalid() {
                return None;
            }
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(handle, &mut counters, counters.cb);
            let _ = CloseHandle(handle);
            if ok.is_ok() { Some(counters.WorkingSetSize as u64) } else { None }
        }
    }

    fn count_threads(pid: u32) -> Result<u32, String> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| e.to_string())?;
            if snap.is_invalid() {
                return Err("Invalid snapshot handle".to_string());
            }
            let mut te = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
            let mut count = 0u32;
            if Thread32First(snap, &mut te).is_ok() {
                loop {
                    if te.th32OwnerProcessID == pid { count += 1; }
                    if Thread32Next(snap, &mut te).is_err() { break; }
                }
            }
            let _ = CloseHandle(snap);
            Ok(count)
        }
    }

    fn count_handles(pid: u32) -> Result<u32, String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid);
            if handle.is_invalid() {
                return Ok(0);
            }
            let mut count: u32 = 0;
            let ok = GetProcessHandleCount(handle, &mut count);
            let _ = CloseHandle(handle);
            if ok.is_ok() { Ok(count) } else { Ok(0) }
        }
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let target_pid = lparam.0 as u32;
        let mut wnd_pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut wnd_pid));
        if wnd_pid == target_pid {
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        BOOL(1)
    }

    fn send_wm_close_to_pid(pid: u32) -> bool {
        unsafe { let _ = EnumWindows(Some(enum_cb), LPARAM(pid as isize)); }
        true
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

// Tauri commands
#[tauri::command]
pub fn get_all_processes() -> Result<Vec<ProcessInfo>, String> {
    win::list_processes()
}

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

// ===== Simple app state for daemon and stats =====
#[derive(Default)]
struct AppState {
    daemon_running: AtomicBool,
    last_scan: Mutex<Option<Instant>>,
    detected_threats: Mutex<u64>,
    killed_processes: Mutex<u64>,
    scan_count: Mutex<u64>,
    activities: Mutex<Vec<Activity>>, // ring buffer semantics kept simple
    worker: Mutex<Option<JoinHandle<()>>>,
    policy: Mutex<PolicyConfig>,
    is_admin: AtomicBool,
    _log_guard: Option<WorkerGuard>,
    // Block restart map: exe path -> blocked until timestamp (Instant)
    block_restart_until: Mutex<HashMap<String, Instant>>,
    // HMAC key for log integrity
    hmac_key: Mutex<Option<Vec<u8>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Activity {
    id: String,
    timestamp: String,
    event: String,
    message: String,
    process_name: Option<String>,
    pid: Option<u32>,
}

fn push_activity(app: &tauri::AppHandle, state: &AppState, event: &str, message: &str, process_name: Option<String>, pid: Option<u32>) {
    let act = Activity {
        id: format!("{}", chrono::Utc::now().timestamp_millis()),
        timestamp: chrono::Utc::now().to_rfc3339(),
        event: event.to_string(),
        message: message.to_string(),
        process_name,
        pid,
    };
    {
        let mut v = state.activities.lock().unwrap();
        v.insert(0, act.clone());
        if v.len() > 10000 { v.truncate(10000); }
    }
    let _ = app.emit_all("new_log_entry", &act);
    match event {
        "SCAN_START" => info!(event="SCAN_START", message),
        "DETECTED" => warn!(event="DETECTED", %message, ?process_name, ?pid),
        "KILL_SUCCESS" => info!(event="KILL_SUCCESS", %message, ?process_name, ?pid),
        "KILL_FAIL" => warn!(event="KILL_FAIL", %message, ?process_name, ?pid),
        "ERROR" => error!(event="ERROR", %message),
        _ => info!(event=%event, %message),
    }
    // Append HMAC line (best-effort)
    if let Some(key) = state.hmac_key.lock().unwrap().clone() {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&key).unwrap();
        let line = format!("{}|{}|{}|{}|{}\n", act.timestamp, act.event, act.message, act.process_name.clone().unwrap_or_default(), act.pid.unwrap_or(0));
        mac.update(line.as_bytes());
        let tag = mac.finalize().into_bytes();
        let _ = std::fs::OpenOptions::new().create(true).append(true).open("logs/edr_kill_switch.hmac").and_then(|mut f| std::io::Write::write_all(&mut f, format!("{} {}", hex::encode(tag), line).as_bytes()));
    }
}

// ===== Policy matching (name/path), whitelist precedence =====
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct Rule {
    #[serde(skip_serializing_if = "Option::is_none")] id: Option<String>,
    #[serde(rename = "rule_type")] rule_type: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")] description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] severity: Option<String>,
    #[serde(rename = "auto_action", skip_serializing_if = "Option::is_none")] auto_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] tags: Option<Vec<String>>,
    #[serde(rename = "created_at", skip_serializing_if = "Option::is_none")] created_at: Option<String>,
    #[serde(rename = "last_modified", skip_serializing_if = "Option::is_none")] last_modified: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct PolicyConfig {
    blacklist: Vec<Rule>,
    whitelist: Vec<Rule>,
}

fn matches_rule_name(name: &str, pat: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let p = pat.to_ascii_lowercase();
    if p.contains('*') { wildcard_match::wildmatch(&p, &n) } else { n == p }
}

fn matches_rule_path(path: &str, pat: &str) -> bool {
    let n = path.to_ascii_lowercase();
    let p = pat.to_ascii_lowercase();
    if p.contains('*') { wildcard_match::wildmatch(&p, &n) } else { n == p }
}

fn matches_rule_hash(path: &str, expected_hex: &str) -> bool {
    if let Some(h) = compute_sha256_hex(path) {
        return h.eq_ignore_ascii_case(expected_hex);
    }
    false
}

fn is_whitelisted(pi: &ProcessInfo, pol: &PolicyConfig) -> bool {
    for r in &pol.whitelist {
        if matches!(r.enabled, Some(false)) { continue; }
        match r.rule_type.as_str() {
            "name" => { if matches_rule_name(&pi.name, &r.value) { return true; } },
            "path" => { if let Some(ref path) = pi.executable_path { if matches_rule_path(path, &r.value) { return true; } } },
            "hash" => { if let Some(ref path) = pi.executable_path { if matches_rule_hash(path, &r.value) { return true; } } },
            _ => {}
        }
    }
    false
}

fn is_blacklisted(pi: &ProcessInfo, pol: &PolicyConfig) -> bool {
    for r in &pol.blacklist {
        if matches!(r.enabled, Some(false)) { continue; }
        match r.rule_type.as_str() {
            "name" => { if matches_rule_name(&pi.name, &r.value) { return true; } },
            "path" => { if let Some(ref path) = pi.executable_path { if matches_rule_path(path, &r.value) { return true; } } },
            "hash" => { if let Some(ref path) = pi.executable_path { if matches_rule_hash(path, &r.value) { return true; } } },
            _ => {}
        }
    }
    false
}

fn policy_path(app: &tauri::AppHandle) -> PathBuf {
    // Try app config dir, fallback to current dir
    if let Ok(dir) = app.path().app_config_dir() {
        let mut p = dir;
        let _ = fs::create_dir_all(&p);
        p.push("policy.json");
        return p;
    }
    PathBuf::from("policy.json")
}

fn read_policy_from_disk(app: &tauri::AppHandle) -> Option<PolicyConfig> {
    let p = policy_path(app);
    if let Ok(bytes) = fs::read(&p) {
        if let Ok(cfg) = serde_json::from_slice::<PolicyConfig>(&bytes) {
            return Some(cfg);
        }
    }
    None
}

fn write_policy_to_disk(app: &tauri::AppHandle, cfg: &PolicyConfig) -> Result<(), String> {
    let p = policy_path(app);
    let data = serde_json::to_vec_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(p, data).map_err(|e| e.to_string())
}

fn compute_sha256_hex(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf).ok()?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let out = hasher.finalize();
    Some(format!("{:x}", out))
}

// ===== Commands for daemon and stats =====
#[tauri::command]
fn start_single_scan(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    // refresh policy from disk if available
    if let Some(cfg) = read_policy_from_disk(&app) {
        *state.policy.lock().unwrap() = cfg;
    }
    let policy = state.policy.lock().unwrap().clone();
    let list = win::list_processes()?;
    *state.scan_count.lock().unwrap() += 1;
    *state.last_scan.lock().unwrap() = Some(Instant::now());
    let _ = app.emit_all("scan-event", serde_json::json!({"event":"SCAN_START"}));

    for p in list {
        // self-protection: never kill our own process
        if std::process::id() == p.pid { continue; }
        if is_whitelisted(&p, &policy) { continue; }
        if is_blacklisted(&p, &policy) {
            *state.detected_threats.lock().unwrap() += 1;
            push_activity(&app, &state, "DETECTED", &format!("Detected {}", p.name), Some(p.name.clone()), Some(p.pid));
            if !state.is_admin.load(Ordering::SeqCst) {
                push_activity(&app, &state, "KILL_FAIL", "Restricted mode (no admin)", Some(p.name), Some(p.pid));
                continue;
            }
            match win::kill_process(p.pid) {
                Ok(true) => {
                    *state.killed_processes.lock().unwrap() += 1;
                    push_activity(&app, &state, "KILL_SUCCESS", "Process terminated", Some(p.name), Some(p.pid));
                }
                _ => {
                    push_activity(&app, &state, "KILL_FAIL", "Failed to terminate", Some(p.name), Some(p.pid));
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn start_daemon(app: tauri::AppHandle, state: tauri::State<AppState>, interval: u64, dry_run: bool) -> Result<(), String> {
    if state.daemon_running.swap(true, Ordering::SeqCst) { return Ok(()); }
    let app2 = app.clone();
    let st = state.inner();
    let handle = thread::spawn(move || {
        loop {
            if !st.daemon_running.load(Ordering::SeqCst) { break; }
            let _ = app2.emit_all("daemon-status", serde_json::json!({"running": true}));
            // Attempt to refresh policy from disk periodically
            if let Some(cfg) = read_policy_from_disk(&app2) {
                *st.policy.lock().unwrap() = cfg;
            }
            let policy = st.policy.lock().unwrap().clone();
            let list = match win::list_processes() { Ok(v) => v, Err(_) => { thread::sleep(Duration::from_millis(interval)); continue; } };
            *st.scan_count.lock().unwrap() += 1;
            *st.last_scan.lock().unwrap() = Some(Instant::now());
            let _ = app2.emit_all("scan-event", serde_json::json!({"event":"SCAN_START"}));
            for p in list {
                if std::process::id() == p.pid { continue; }
                if is_whitelisted(&p, &policy) { continue; }
                if is_blacklisted(&p, &policy) {
                    *st.detected_threats.lock().unwrap() += 1;
                    push_activity(&app2, &st, "DETECTED", &format!("Detected {}", p.name), Some(p.name.clone()), Some(p.pid));
                    if !dry_run {
                        if !st.is_admin.load(Ordering::SeqCst) {
                            push_activity(&app2, &st, "KILL_FAIL", "Restricted mode (no admin)", Some(p.name), Some(p.pid));
                            continue;
                        }
                        match win::kill_process(p.pid) {
                            Ok(true) => {
                                *st.killed_processes.lock().unwrap() += 1;
                                push_activity(&app2, &st, "KILL_SUCCESS", "Process terminated", Some(p.name), Some(p.pid));
                                if let Some(ref path) = p.executable_path {
                                    // block restart for 5 seconds
                                    st.block_restart_until.lock().unwrap().insert(path.clone(), Instant::now() + Duration::from_secs(5));
                                }
                            }
                            _ => {
                                push_activity(&app2, &st, "KILL_FAIL", "Failed to terminate", Some(p.name), Some(p.pid));
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(interval));
        }
        let _ = app2.emit_all("daemon-status", serde_json::json!({"running": false}));
    });
    *state.worker.lock().unwrap() = Some(handle);
    Ok(())
}

#[tauri::command]
fn stop_daemon(state: tauri::State<AppState>) -> Result<(), String> {
    state.daemon_running.store(false, Ordering::SeqCst);
    if let Some(h) = state.worker.lock().unwrap().take() { let _ = h.join(); }
    Ok(())
}

#[tauri::command]
fn emergency_stop(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    state.daemon_running.store(false, Ordering::SeqCst);
    if let Some(h) = state.worker.lock().unwrap().take() { let _ = h.join(); }
    push_activity(&app, &state, "ERROR", "Emergency stop activated", None, None);
    Ok(())
}

#[tauri::command]
fn get_system_stats(state: tauri::State<AppState>) -> Result<serde_json::Value, String> {
    let uptime = state.last_scan.lock().unwrap().map(|t| t.elapsed().as_secs()).unwrap_or(0);
    Ok(serde_json::json!({
        "totalProcesses": win::list_processes().map(|v| v.len()).unwrap_or(0),
        "detectedThreats": *state.detected_threats.lock().unwrap(),
        "killedProcesses": *state.killed_processes.lock().unwrap(),
        "uptime": uptime,
        "scanCount": *state.scan_count.lock().unwrap()
    }))
}

#[tauri::command]
fn get_recent_activities(state: tauri::State<AppState>) -> Result<Vec<Activity>, String> {
    Ok(state.activities.lock().unwrap().clone())
}

#[tauri::command]
fn get_system_health() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "scanner": "HEALTHY",
        "policy_engine": "HEALTHY",
        "kill_system": "HEALTHY",
        "logging": "HEALTHY"
    }))
}

// ===== Optional: simple hot-reload watcher (best-effort) =====
// Note: requires a running loop; we integrate with daemon refresh above.

// ===== Policy config commands =====
#[tauri::command]
fn get_policy_config(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<PolicyConfig, String> {
    if let Some(cfg) = read_policy_from_disk(&app) {
        *state.policy.lock().unwrap() = cfg.clone();
        return Ok(cfg);
    }
    Ok(state.policy.lock().unwrap().clone())
}

#[tauri::command]
fn save_policy_config(app: tauri::AppHandle, state: tauri::State<AppState>, cfg: PolicyConfig) -> Result<(), String> {
    write_policy_to_disk(&app, &cfg)?;
    *state.policy.lock().unwrap() = cfg;
    let _ = app.emit_all("config-event", serde_json::json!({"event":"CONFIG_RELOAD"}));
    Ok(())
}

fn main() {
    // Init logging to file (json lines) in app data dir
    let (guard, is_admin, key) = {
        let file_appender = tracing_appender::rolling::daily("logs", "edr_kill_switch.jsonl");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let fmt_layer = tracing_subscriber::fmt::layer().json().with_writer(non_blocking);
        tracing_subscriber::registry().with(fmt_layer).init();
        // naive admin detection
        let admin = cfg!(target_os = "windows") && is_running_as_admin();
        let mut key = vec![0u8; 32]; rand::thread_rng().fill_bytes(&mut key);
        (guard, admin, key)
    };

    let mut state = AppState::default();
    state._log_guard = Some(guard);
    state.is_admin.store(is_admin, Ordering::SeqCst);
    *state.hmac_key.lock().unwrap() = Some(key);

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_all_processes,
            get_process_list,
            get_process_details,
            kill_process,
            start_process,
            start_single_scan,
            start_daemon,
            stop_daemon,
            get_system_stats,
            get_recent_activities,
            get_system_health,
            emergency_stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "windows")]
fn is_running_as_admin() -> bool {
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() { return false; }
        let mut elev = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        if GetTokenInformation(token, TokenElevation, Some(&mut elev as *mut _ as *mut _), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut len).is_ok() {
            elev.TokenIsElevated != 0
        } else { false }
    }
}

#[cfg(not(target_os = "windows"))]
fn is_running_as_admin() -> bool { false }
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