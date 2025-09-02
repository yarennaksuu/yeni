// src/app/services/process.service.ts
import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { BehaviorSubject, Observable } from 'rxjs';

export interface ProcessInfo {
  pid: number;
  name: string;
  executable_path?: string;
  memory_usage: number;
  cpu_usage: number;
  status: string;
  parent_pid: number;
}

export interface ProcessDetails {
  basic_info: ProcessInfo;
  command_line?: string;
  working_directory?: string;
  threads_count: number;
  handles_count: number;
}

@Injectable({
  providedIn: 'root'
})
export class ProcessService {
  private processesSubject = new BehaviorSubject<ProcessInfo[]>([]);
  public processes$ = this.processesSubject.asObservable();
  
  private selectedProcessSubject = new BehaviorSubject<ProcessInfo | null>(null);
  public selectedProcess$ = this.selectedProcessSubject.asObservable();
  
  private refreshInterval: any;

  constructor() {
    this.startAutoRefresh();
  }

  async getAllProcesses(): Promise<ProcessInfo[]> {
    try {
      const processes = await invoke<ProcessInfo[]>('get_all_processes');
      this.processesSubject.next(processes);
      return processes;
    } catch (error) {
      console.error('Process listesi alınamadı:', error);
      throw error;
    }
  }

  async killProcess(pid: number): Promise<boolean> {
    try {
      const success = await invoke<boolean>('kill_process', { pid });
      if (success) {
        // Process listesini güncelle
        await this.getAllProcesses();
        
        // Seçili process silinmişse temizle
        const selectedProcess = this.selectedProcessSubject.value;
        if (selectedProcess && selectedProcess.pid === pid) {
          this.selectedProcessSubject.next(null);
        }
      }
      return success;
    } catch (error) {
      console.error('Process sonlandırılamadı:', error);
      return false;
    }
  }

  async getProcessDetails(pid: number): Promise<ProcessDetails> {
    try {
      return await invoke<ProcessDetails>('get_process_details', { pid });
    } catch (error) {
      console.error('Process detayları alınamadı:', error);
      throw error;
    }
  }

  async startProcess(executablePath: string, args?: string): Promise<number> {
    try {
      const pid = await invoke<number>('start_process', { 
        executablePath,
        arguments: args || undefined
      });
      
      // Process listesini güncelle
      setTimeout(() => this.getAllProcesses(), 500);
      
      return pid;
    } catch (error) {
      console.error('Process başlatılamadı:', error);
      throw error;
    }
  }

  selectProcess(process: ProcessInfo | null): void {
    this.selectedProcessSubject.next(process);
  }

  getSelectedProcess(): ProcessInfo | null {
    return this.selectedProcessSubject.value;
  }

  filterProcesses(searchTerm: string): Observable<ProcessInfo[]> {
    return new Observable(observer => {
      this.processes$.subscribe(processes => {
        if (!searchTerm) {
          observer.next(processes);
          return;
        }
        
        const filtered = processes.filter(process => 
          process.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          process.pid.toString().includes(searchTerm) ||
          (process.executable_path && process.executable_path.toLowerCase().includes(searchTerm.toLowerCase()))
        );
        
        observer.next(filtered);
      });
    });
  }

  formatMemoryUsage(bytes: number): string {
    if (bytes === 0) return '0 B';
    
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  private startAutoRefresh(): void {
    // İlk yükleme
    this.getAllProcesses();
    
    // Her 5 saniyede bir güncelle
    this.refreshInterval = setInterval(() => {
      this.getAllProcesses();
    }, 5000);
  }

  stopAutoRefresh(): void {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = null;
    }
  }

  ngOnDestroy(): void {
    this.stopAutoRefresh();
  }
}