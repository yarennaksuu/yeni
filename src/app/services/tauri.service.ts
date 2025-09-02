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
}