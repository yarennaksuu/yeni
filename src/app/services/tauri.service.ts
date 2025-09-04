// src/app/services/tauri.service.ts
import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

@Injectable({
  providedIn: 'root'
})
export class TauriService {
  async getProcessList() {
    return await invoke('get_process_list');
  }

  async killProcess(pid: number, name: string) {
    return await invoke('kill_process', { pid, name });
  }

  async startScanner(dryRun: boolean) {
    return await invoke('start_scanner', { dryRun });
  }

  async getSystemStatus() {
    return await invoke('get_system_status');
  }

  // New commands wired to backend
  async startSingleScan() {
    return await invoke('start_single_scan');
  }

  async startDaemon(interval: number, dryRun: boolean) {
    return await invoke('start_daemon', { interval, dryRun });
  }

  async stopDaemon() {
    return await invoke('stop_daemon');
  }

  async getSystemStats() {
    return await invoke('get_system_stats');
  }

  async getRecentActivities() {
    return await invoke('get_recent_activities');
  }

  async getSystemHealth() {
    return await invoke('get_system_health');
  }

  async emergencyStop() {
    return await invoke('emergency_stop');
  }

  async getPolicyConfig() {
    return await invoke('get_policy_config');
  }

  async savePolicyConfig(cfg: any) {
    return await invoke('save_policy_config', { cfg });
  }
}