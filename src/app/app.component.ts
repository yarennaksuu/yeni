// src/app/app.component.ts (Standalone için)
import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { RouterOutlet, RouterLink } from '@angular/router';
import { ProcessService, ProcessInfo, ProcessDetails } from './services/process.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-root',
  imports: [CommonModule, FormsModule, RouterOutlet, RouterLink],
  templateUrl: './app.component.html',
  styleUrls: ['./app.component.css']
})
export class AppComponent implements OnInit, OnDestroy {
  processes: ProcessInfo[] = [];
  filteredProcesses: ProcessInfo[] = [];
  selectedProcess: ProcessInfo | null = null;
  processDetails: ProcessDetails | null = null;
  
  searchTerm: string = '';
  isLoading: boolean = false;
  errorMessage: string = '';
  
  // Add Process Modal
  showAddModal: boolean = false;
  newProcessPath: string = '';
  newProcessArgs: string = '';
  
  private subscriptions: Subscription = new Subscription();

  constructor(private processService: ProcessService) {}

  ngOnInit(): void {
    this.subscribeToProcesses();
    this.subscribeToSelectedProcess();
    this.loadProcesses();
  }

  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
    this.processService.stopAutoRefresh();
  }

  private subscribeToProcesses(): void {
    const processesSubscription = this.processService.processes$.subscribe(processes => {
      this.processes = processes;
      this.filterProcesses();
    });
    this.subscriptions.add(processesSubscription);
  }

  private subscribeToSelectedProcess(): void {
    const selectedProcessSubscription = this.processService.selectedProcess$.subscribe(process => {
      this.selectedProcess = process;
      if (process) {
        this.loadProcessDetails(process.pid);
      } else {
        this.processDetails = null;
      }
    });
    this.subscriptions.add(selectedProcessSubscription);
  }

  async loadProcesses(): Promise<void> {
    try {
      this.isLoading = true;
      this.errorMessage = '';
      await this.processService.getAllProcesses();
    } catch (error) {
      this.errorMessage = 'Process listesi yüklenirken hata oluştu: ' + error;
    } finally {
      this.isLoading = false;
    }
  }

  async loadProcessDetails(pid: number): Promise<void> {
    try {
      this.processDetails = await this.processService.getProcessDetails(pid);
    } catch (error) {
      console.error('Process detayları yüklenemedi:', error);
    }
  }

  selectProcess(process: ProcessInfo): void {
    this.processService.selectProcess(process);
  }

  async killSelectedProcess(): Promise<void> {
    if (!this.selectedProcess) return;
    
    if (!confirm(`${this.selectedProcess.name} (PID: ${this.selectedProcess.pid}) processini sonlandırmak istediğinizden emin misiniz?`)) {
      return;
    }

    try {
      const success = await this.processService.killProcess(this.selectedProcess.pid);
      if (success) {
        this.errorMessage = '';
        // selectedProcess service tarafından otomatik temizlenir
      } else {
        this.errorMessage = 'Process sonlandırılamadı';
      }
    } catch (error) {
      this.errorMessage = 'Process sonlandırılırken hata oluştu: ' + error;
    }
  }

  filterProcesses(): void {
    if (!this.searchTerm.trim()) {
      this.filteredProcesses = [...this.processes];
      return;
    }
    
    const term = this.searchTerm.toLowerCase();
    this.filteredProcesses = this.processes.filter(process =>
      process.name.toLowerCase().includes(term) ||
      process.pid.toString().includes(term) ||
      (process.executable_path && process.executable_path.toLowerCase().includes(term))
    );
  }

  onSearchChange(event: Event): void {
    const target = event.target as HTMLInputElement;
    this.searchTerm = target.value;
    this.filterProcesses();
  }

  // Add Process Modal Methods
  openAddModal(): void {
    this.showAddModal = true;
    this.newProcessPath = '';
    this.newProcessArgs = '';
    this.errorMessage = '';
  }

  closeAddModal(): void {
    this.showAddModal = false;
    this.newProcessPath = '';
    this.newProcessArgs = '';
  }

  async addNewProcess(): Promise<void> {
    if (!this.newProcessPath.trim()) {
      this.errorMessage = 'Executable path gerekli';
      return;
    }

    try {
      this.isLoading = true;
      this.errorMessage = '';
      
      const pid = await this.processService.startProcess(
        this.newProcessPath.trim(),
        this.newProcessArgs.trim() || undefined
      );
      
      console.log(`Yeni process başlatıldı: PID ${pid}`);
      this.closeAddModal();
    } catch (error) {
      this.errorMessage = 'Process başlatılırken hata oluştu: ' + error;
    } finally {
      this.isLoading = false;
    }
  }

  async selectExecutableFile(): Promise<void> {
    try {
      // File picker için dialog API kullanılabilir
      // Şimdilik manual input
      const path = prompt('Executable file path girin:');
      if (path) {
        this.newProcessPath = path;
      }
    } catch (error) {
      console.error('File seçiminde hata:', error);
    }
  }

  formatMemoryUsage(bytes: number): string {
    return this.processService.formatMemoryUsage(bytes);
  }

  refreshProcesses(): void {
    this.loadProcesses();
  }
}