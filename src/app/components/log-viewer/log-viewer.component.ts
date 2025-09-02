import { Component, OnInit, OnDestroy, ViewChild, ElementRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface LogEntry {
  id: string;
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug' | 'trace';
  event: string;
  message: string;
  pid?: number;
  processName?: string;
  ruleName?: string;
  details?: any;
}

interface LogFilter {
  level: string[];
  event: string[];
  search: string;
  dateFrom?: string;
  dateTo?: string;
  processName?: string;
}
@Component({
  selector: 'app-log-viewer',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './log-viewer.component.html',
  styleUrls: ['./log-viewer.component.css']
})

export class LogViewerComponent implements OnInit, OnDestroy {
  @ViewChild('logsContainer') logsContainer!: ElementRef;

  allLogs: LogEntry[] = [];
  filteredLogs: LogEntry[] = [];
  selectedLog: LogEntry | null = null;

  filter: LogFilter = {
    level: ['info', 'warn', 'error'],
    event: [],
    search: '',
    processName: ''
  };

  logLevels = ['info', 'warn', 'error', 'debug', 'trace'];
  eventTypes = ['SCAN_START', 'DETECTED', 'KILL_SUCCESS', 'KILL_FAIL', 'ERROR', 'CONFIG_RELOAD'];

  autoScroll = true;
  filtersCollapsed = false;
  showLogDetails = false;
  isConnected = false;

  private eventUnlisteners: UnlistenFn[] = [];
  private maxLogs = 10000; // Maximum number of logs to keep in memory

  async ngOnInit() {
    await this.loadExistingLogs();
    this.setupRealTimeLogging();
    this.applyFilters();
  }

  ngOnDestroy() {
    this.eventUnlisteners.forEach(unlisten => unlisten());
  }

  private async loadExistingLogs() {
    try {
      const logs = await invoke('get_all_logs') as LogEntry[];
      this.allLogs = logs.slice(-this.maxLogs); // Keep only recent logs
      this.applyFilters();
    } catch (error) {
      console.error('Failed to load existing logs:', error);
    }
  }

  private async setupRealTimeLogging() {
    try {
      // Listen for new log entries
      const logUnlisten = await listen('new_log_entry', (event: any) => {
        const newLog = event.payload as LogEntry;
        this.addNewLog(newLog);
      });
      this.eventUnlisteners.push(logUnlisten);

      // Listen for connection status
      const statusUnlisten = await listen('log_stream_status', (event: any) => {
        this.isConnected = event.payload.connected;
      });
      this.eventUnlisteners.push(statusUnlisten);

      this.isConnected = true;
    } catch (error) {
      console.error('Failed to setup real-time logging:', error);
      this.isConnected = false;
    }
  }

  private addNewLog(log: LogEntry) {
    this.allLogs.push(log);
    
    // Maintain max logs limit
    if (this.allLogs.length > this.maxLogs) {
      this.allLogs = this.allLogs.slice(-this.maxLogs);
    }

    this.applyFilters();

    // Auto-scroll to bottom if enabled
    if (this.autoScroll) {
      setTimeout(() => this.scrollToBottom(), 100);
    }
  }

  applyFilters() {
    this.filteredLogs = this.allLogs.filter(log => {
      // Level filter
      if (!this.filter.level.includes(log.level)) {
        return false;
      }

      // Event filter
      if (this.filter.event.length > 0 && !this.filter.event.includes(log.event)) {
        return false;
      }

      // Search filter
      if (this.filter.search) {
        const searchTerm = this.filter.search.toLowerCase();
        const searchableText = `${log.message} ${log.processName || ''} ${log.event}`.toLowerCase();
        if (!searchableText.includes(searchTerm)) {
          return false;
        }
      }

      // Process name filter
      if (this.filter.processName && log.processName) {
        if (!log.processName.toLowerCase().includes(this.filter.processName.toLowerCase())) {
          return false;
        }
      }

      // Date range filter
      if (this.filter.dateFrom) {
        const logDate = new Date(log.timestamp);
        const fromDate = new Date(this.filter.dateFrom);
        if (logDate < fromDate) {
          return false;
        }
      }

      if (this.filter.dateTo) {
        const logDate = new Date(log.timestamp);
        const toDate = new Date(this.filter.dateTo);
        if (logDate > toDate) {
          return false;
        }
      }

      return true;
    });
  }

  toggleLevelFilter(level: string) {
    const index = this.filter.level.indexOf(level);
    if (index === -1) {
      this.filter.level.push(level);
    } else {
      this.filter.level.splice(index, 1);
    }
    this.applyFilters();
  }

  toggleEventFilter(event: string) {
    const index = this.filter.event.indexOf(event);
    if (index === -1) {
      this.filter.event.push(event);
    } else {
      this.filter.event.splice(index, 1);
    }
    this.applyFilters();
  }

  resetFilters() {
    this.filter = {
      level: ['info', 'warn', 'error'],
      event: [],
      search: '',
      processName: ''
    };
    this.applyFilters();
  }

  toggleFilters() {
    this.filtersCollapsed = !this.filtersCollapsed;
  }

  toggleAutoScroll() {
    this.autoScroll = !this.autoScroll;
    if (this.autoScroll) {
      this.scrollToBottom();
    }
  }

  selectLog(log: LogEntry) {
    this.selectedLog = log;
  }

  viewLogDetails(log: LogEntry, event: Event) {
    event.stopPropagation();
    this.selectedLog = log;
    this.showLogDetails = true;
  }

  closeLogDetails() {
    this.showLogDetails = false;
  }

  async copyLogEntry(log: LogEntry, event?: Event) {
    if (event) event.stopPropagation();
    
    const logText = `[${log.timestamp}] ${log.level.toUpperCase()} ${log.event}: ${log.message}`;
    
    try {
      await navigator.clipboard.writeText(logText);
      console.log('Log entry copied to clipboard');
    } catch (error) {
      console.error('Failed to copy log entry:', error);
    }
  }

  filterByProcess(log: LogEntry) {
    if (log.processName) {
      this.filter.processName = log.processName;
      this.applyFilters();
      this.closeLogDetails();
    }
  }

  async refreshLogs() {
    await this.loadExistingLogs();
  }

  async exportLogs() {
    try {
      const logsJson = JSON.stringify(this.filteredLogs, null, 2);
      await invoke('export_logs', { 
        logs: logsJson,
        filename: `edr_logs_${new Date().toISOString().split('T')[0]}.json`
      });
      console.log('Logs exported successfully');
    } catch (error) {
      console.error('Failed to export logs:', error);
    }
  }

  async clearLogs() {
    if (confirm('Are you sure you want to clear all logs? This action cannot be undone.')) {
      try {
        await invoke('clear_all_logs');
        this.allLogs = [];
        this.filteredLogs = [];
        this.selectedLog = null;
        console.log('All logs cleared');
      } catch (error) {
        console.error('Failed to clear logs:', error);
      }
    }
  }

  private scrollToBottom() {
    if (this.logsContainer) {
      const container = this.logsContainer.nativeElement;
      container.scrollTop = container.scrollHeight;
    }
  }

  getLogCountByLevel(level: string): number {
    return this.allLogs.filter(log => log.level === level).length;
  }

  getEventClass(event: string): string {
    const eventClasses: { [key: string]: string } = {
      'SCAN_START': 'event-scan',
      'DETECTED': 'event-detection',
      'KILL_SUCCESS': 'event-success',
      'KILL_FAIL': 'event-error',
      'ERROR': 'event-error',
      'CONFIG_RELOAD': 'event-config'
    };
    return eventClasses[event] || 'event-default';
  }

  highlightSearch(text: string): string {
    if (!this.filter.search) return text;
    
    const searchTerm = this.filter.search;
    const regex = new RegExp(`(${searchTerm})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }

  formatTimestamp(timestamp: string): string {
    return new Date(timestamp).toLocaleString();
  }

  formatTime(timestamp: string): string {
    return new Date(timestamp).toLocaleTimeString();
  }

  formatDetails(details: any): string {
    return JSON.stringify(details, null, 2);
  }

  trackByLogId(index: number, log: LogEntry): string {
    return log.id;
  }
}