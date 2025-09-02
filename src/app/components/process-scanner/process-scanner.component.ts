// src/app/components/process-scanner/process-scanner.component.ts
import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ReactiveFormsModule, FormBuilder, FormGroup } from '@angular/forms';
import { FormsModule } from '@angular/forms';
import { MatSnackBar } from '@angular/material/snack-bar';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatTableModule } from '@angular/material/table';
import { MatIconModule } from '@angular/material/icon';
import { MatChipsModule } from '@angular/material/chips';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { MatMenuModule } from '@angular/material/menu';
import { MatSnackBarModule } from '@angular/material/snack-bar';
import { MatTabsModule } from '@angular/material/tabs';
import { TauriService } from '../../services/tauri.service';


interface ProcessInfo {
  pid: number;
  name: string;
  path?: string;
  command: string[];
  hash?: string;
  isWhitelisted: boolean;
  isBlacklisted: boolean;
  matchedRule?: string;
  cpuUsage: number;
  memoryUsage: number;
  status: 'running' | 'terminated' | 'unknown';
}

@Component({
  selector: 'app-process-scanner',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    FormsModule,
    MatCardModule,
    MatButtonModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatTableModule,
    MatIconModule,
    MatChipsModule,
    MatSlideToggleModule,
    MatMenuModule,
    MatSnackBarModule,
    MatTabsModule
  ],

  templateUrl: './process-scanner.component.html',
  styleUrl: './process-scanner.component.css'
})
export class ProcessScannerComponent implements OnInit, OnDestroy {
  processes: ProcessInfo[] = [];
  filteredProcesses: ProcessInfo[] = [];
  filterForm: FormGroup;
  isScanning = false;
  isAutoRefresh = false;
  isDryRun = false;
  refreshInterval: any;
   settingsForm!: FormGroup; // formu tanımla
  isElevated = false;        // Admin durumu
  Platform = '';             // platform bilgisi

  
  displayedColumns: string[] = ['status', 'pid', 'name', 'path', 'rule', 'cpu', 'memory', 'actions'];

  constructor(
    private fb: FormBuilder,
    private tauriService: TauriService,
    private snackBar: MatSnackBar
  ) {
    this.filterForm = this.fb.group({
      processName: [''],
      status: ['all'],
      rule: ['all']
    });

     this.settingsForm = this.fb.group({
      autoRefreshInterval: [5],      // default 5 saniye
      theme: ['light'],              // default light theme
      enableNotifications: [true],   // default true
      confirmBeforeKill: [true]      // default true
    });
  }
  saveSettings() {
  console.log('Settings saved:', this.settingsForm.value);
  this.snackBar.open('Settings saved', 'Close', { duration: 2000 });
}

  ngOnInit() {
    this.loadProcesses();
    this.setupFiltering();
  }

  ngOnDestroy() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }
  }

  setupFiltering() {
    this.filterForm.valueChanges.subscribe(() => {
      this.applyFilter();
    });
  }

  async loadProcesses() {
    try {
      this.isScanning = true;
      const processes = await this.tauriService.getProcessList() as any[];
      this.processes = processes.map(this.mapProcessInfo);
      this.applyFilter();
      this.snackBar.open(`Loaded ${processes.length} processes`, 'Close', { duration: 2000 });
    } catch (error) {
      this.snackBar.open('Failed to load processes', 'Close', { duration: 3000 });
      console.error('Error loading processes:', error);
    } finally {
      this.isScanning = false;
    }
  }

  mapProcessInfo(process: any): ProcessInfo {
    return {
      pid: process.pid,
      name: process.name || 'Unknown',
      path: process.path,
      command: process.command || [],
      hash: process.hash,
      isWhitelisted: process.is_whitelisted || false,
      isBlacklisted: process.is_blacklisted || false,
      matchedRule: process.matched_rule,
      cpuUsage: process.cpu_usage || 0,
      memoryUsage: process.memory_usage || 0,
      status: process.status || 'running'
    };
  }

  applyFilter() {
    const filters = this.filterForm.value;
    
    this.filteredProcesses = this.processes.filter(process => {
      // Process name filter
      if (filters.processName && 
          !process.name.toLowerCase().includes(filters.processName.toLowerCase())) {
        return false;
      }
      
      // Status filter
      if (filters.status !== 'all') {
        switch (filters.status) {
          case 'whitelisted':
            if (!process.isWhitelisted) return false;
            break;
          case 'blacklisted':
            if (!process.isBlacklisted) return false;
            break;
          case 'unmatched':
            if (process.isWhitelisted || process.isBlacklisted) return false;
            break;
          case 'terminated':
            if (process.status !== 'terminated') return false;
            break;
        }
      }
      
      return true;
    });
  }

  toggleAutoRefresh() {
    if (this.isAutoRefresh) {
      clearInterval(this.refreshInterval);
      this.isAutoRefresh = false;
      this.snackBar.open('Auto-refresh disabled', 'Close', { duration: 2000 });
    } else {
      this.refreshInterval = setInterval(() => {
        this.loadProcesses();
      }, 5000);
      this.isAutoRefresh = true;
      this.snackBar.open('Auto-refresh enabled (5s interval)', 'Close', { duration: 2000 });
    }
  }

  async killProcess(process: ProcessInfo) {
    if (!this.isDryRun) {
      const confirmed = confirm(`Are you sure you want to terminate ${process.name} (PID: ${process.pid})?`);
      if (!confirmed) return;
    }

    try {
      if (this.isDryRun) {
        this.snackBar.open(`DRY RUN: Would kill ${process.name} (PID: ${process.pid})`, 'Close', { duration: 3000 });
        return;
      }

      await this.tauriService.killProcess(process.pid, process.name);
      this.snackBar.open(`Process ${process.name} terminated successfully`, 'Close', { duration: 3000 });
      
      // Update process status in table
      const index = this.processes.findIndex(p => p.pid === process.pid);
      if (index !== -1) {
        this.processes[index].status = 'terminated';
        this.applyFilter();
      }
    } catch (error) {
      this.snackBar.open(`Failed to terminate ${process.name}: ${error}`, 'Close', { duration: 5000 });
      console.error('Error killing process:', error);
    }
  }

  async runSingleScan() {
    try {
      this.isScanning = true;
      await this.tauriService.startScanner(this.isDryRun);
      this.snackBar.open('Scan completed', 'Close', { duration: 3000 });
      // Refresh process list after scan
      setTimeout(() => this.loadProcesses(), 1000);
    } catch (error) {
      this.snackBar.open('Failed to run scan', 'Close', { duration: 3000 });
      console.error('Error running scan:', error);
    } finally {
      this.isScanning = false;
    }
  }

  getStatusIcon(process: ProcessInfo): string {
    if (process.status === 'terminated') return 'cancel';
    if (process.isBlacklisted) return 'dangerous';
    if (process.isWhitelisted) return 'verified';
    return 'help_outline';
  }

  getStatusColor(process: ProcessInfo): string {
    if (process.status === 'terminated') return 'warn';
    if (process.isBlacklisted) return 'warn';
    if (process.isWhitelisted) return 'primary';
    return '';
  }

  getStatusText(process: ProcessInfo): string {
    if (process.status === 'terminated') return 'Terminated';
    if (process.isBlacklisted) return 'Blacklisted';
    if (process.isWhitelisted) return 'Whitelisted';
    return 'Unmatched';
  }

  formatCommand(command: string[]): string {
    const fullCommand = command.join(' ');
    return fullCommand.substring(0, 100) + (fullCommand.length > 100 ? '...' : '');
  }

  formatMemoryUsage(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  formatCpuUsage(usage: number): string {
    return `${usage.toFixed(1)}%`;
  }

  exportProcessList() {
    try {
      const csvContent = this.convertProcessesToCSV(this.filteredProcesses);
      const blob = new Blob([csvContent], { type: 'text/csv' });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `processes_${new Date().toISOString().split('T')[0]}.csv`;
      a.click();
      window.URL.revokeObjectURL(url);
      this.snackBar.open('Process list exported', 'Close', { duration: 3000 });
    } catch (error) {
      this.snackBar.open('Failed to export process list', 'Close', { duration: 3000 });
      console.error('Error exporting processes:', error);
    }
  }

  convertProcessesToCSV(processes: ProcessInfo[]): string {
    const headers = ['PID', 'Name', 'Path', 'Status', 'Rule', 'CPU %', 'Memory', 'Command'];
    const csvRows = [headers.join(',')];
    
    for (const process of processes) {
      const row = [
        process.pid,
        `"${process.name}"`,
        `"${process.path || ''}"`,
        this.getStatusText(process),
        `"${process.matchedRule || ''}"`,
        this.formatCpuUsage(process.cpuUsage),
        this.formatMemoryUsage(process.memoryUsage),
        `"${this.formatCommand(process.command)}"`
      ];
      csvRows.push(row.join(','));
    }
    
    return csvRows.join('\n');
  }

  clearFilters() {
    this.filterForm.reset({
      processName: '',
      status: 'all',
      rule: 'all'
    });
  }
}
