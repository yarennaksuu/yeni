// src/app/services/tauri.service.ts
import { Injectable } from '@angular/core';

// Tauri import kontrolü
let tauriInvoke: any;
try {
  import('@tauri-apps/api/core').then(tauri => {
    tauriInvoke = tauri.invoke;
  });
} catch {
  tauriInvoke = null;
}

@Injectable({
  providedIn: 'root'
})
export class TauriService {
  
  async getProcessList() {
    if (tauriInvoke) {
      return await tauriInvoke('get_process_list');
    } else {
      // Mock data
      return [
        { 
          pid: 1234, 
          name: 'chrome.exe', 
          path: 'C:\\Program Files\\Google\\Chrome\\chrome.exe',
          cpu_usage: 2.5,
          memory_usage: 150000000,
          is_whitelisted: false,
          is_blacklisted: false,
          status: 'running'
        },
        { 
          pid: 5678, 
          name: 'notepad.exe', 
          path: 'C:\\Windows\\System32\\notepad.exe',
          cpu_usage: 0.1,
          memory_usage: 5000000,
          is_whitelisted: true,
          is_blacklisted: false,
          status: 'running'
        }
      ];
    }
  }

  async killProcess(pid: number, name: string) {
    if (tauriInvoke) {
      return await tauriInvoke('kill_process', { pid, name });
    } else {
      console.log(`Mock: Killed process ${name} (${pid})`);
      return true;
    }
  }

  async startScanner(dryRun: boolean) {
    if (tauriInvoke) {
      return await tauriInvoke('start_scanner', { dryRun });
    } else {
      console.log(`Mock: Scanner started (dry run: ${dryRun})`);
      return { processes_scanned: 10, threats_found: 2 };
    }
  }

  async getSystemStatus() {
    if (tauriInvoke) {
      return await tauriInvoke('get_system_status');
    } else {
      return { status: 'running', uptime: 3600 };
    }
  }

  // Dashboard için gerekli metodlar
  async getSystemStats() {
    return {
      cpu_usage: Math.random() * 100,
      memory_usage: Math.random() * 8000000000,
      total_memory: 16000000000,
      process_count: 150,
      last_scan_time: new Date().toISOString(),
      threats_detected: 0,
      processes_killed: 0,
      daemon_status: 'running'
    };
  }

  async getKillStats() {
    return [0, 1, 2, 0, 3, 1, 2];
  }

  async checkAdminPrivileges() {
    return false;
  }

  async getVersion() {
    return '1.0.0';
  }

  // Policy Editor için boş metodlar
  async getBlacklist() { return []; }
  async getWhitelist() { return []; }
  async addBlacklistRule(rule: any) { return true; }
  async addWhitelistRule(rule: any) { return true; }
  async removeRule(id: string, isBlacklist: boolean) { return true; }
}