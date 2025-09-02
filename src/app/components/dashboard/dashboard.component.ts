import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface SystemStats {
  totalProcesses: number;
  detectedThreats: number;
  killedProcesses: number;
  uptime: number;
  scanCount: number;
}

interface RecentActivity {
  id: string;
  timestamp: string;
  type: 'scan' | 'detection' | 'kill' | 'error';
  message: string;
  processName?: string;
  pid?: number;
}
@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [CommonModule],
  styleUrls: ['./dashboard.component.css'],
  templateUrl: './dashboard.component.html'
})
export class DashboardComponent implements OnInit, OnDestroy {
  stats: SystemStats = {
    totalProcesses: 0,
    detectedThreats: 0,
    killedProcesses: 0,
    uptime: 0,
    scanCount: 0
  };

  recentActivities: RecentActivity[] = [];
  systemStatus = 'IDLE';
  isScanning = false;
  isDaemonRunning = false;
  isDryRun = false;
  scanInterval = 2000;

  // Health indicators
  scannerHealth = 'HEALTHY';
  policyEngineHealth = 'HEALTHY';
  killSystemHealth = 'HEALTHY';
  loggingHealth = 'HEALTHY';

  private eventUnlisteners: UnlistenFn[] = [];
  private statsUpdateInterval: any;

  async ngOnInit() {
    await this.initializeComponent();
    this.setupEventListeners();
    this.startStatsUpdate();
  }

  ngOnDestroy() {
    this.eventUnlisteners.forEach(unlisten => unlisten());
    if (this.statsUpdateInterval) {
      clearInterval(this.statsUpdateInterval);
    }
  }

  private async initializeComponent() {
    try {
      await this.refreshStats();
      await this.loadRecentActivity();
      await this.checkSystemHealth();
    } catch (error) {
      console.error('Failed to initialize dashboard:', error);
      this.addActivity('error', 'Failed to initialize dashboard: ' + error);
    }
  }

  private async setupEventListeners() {
    try {
      // Listen for scan events
      const scanUnlisten = await listen('scan-event', (event: any) => {
        const data = event.payload;
        this.handleScanEvent(data);
      });
      this.eventUnlisteners.push(scanUnlisten);

      // Listen for daemon status changes
      const daemonUnlisten = await listen('daemon-status', (event: any) => {
        this.isDaemonRunning = event.payload.running;
        this.systemStatus = this.isDaemonRunning ? 'MONITORING' : 'IDLE';
      });
      this.eventUnlisteners.push(daemonUnlisten);

      // Listen for health updates
      const healthUnlisten = await listen('health-update', (event: any) => {
        this.updateHealthMetrics(event.payload);
      });
      this.eventUnlisteners.push(healthUnlisten);

    } catch (error) {
      console.error('Failed to setup event listeners:', error);
    }
  }

  private startStatsUpdate() {
    this.statsUpdateInterval = setInterval(async () => {
      if (!this.isScanning) {
        await this.refreshStats();
      }
    }, 5000); // Update every 5 seconds
  }

  async startScan() {
    if (this.isScanning) return;

    this.isScanning = true;
    this.systemStatus = 'SCANNING';
    
    try {
      await invoke('start_single_scan', { dryRun: false });
      this.addActivity('scan', 'Manual scan initiated');
    } catch (error) {
      console.error('Scan failed:', error);
      this.addActivity('error', 'Scan failed: ' + error);
    } finally {
      this.isScanning = false;
      this.systemStatus = this.isDaemonRunning ? 'MONITORING' : 'IDLE';
    }
  }

  async toggleDaemon() {
    try {
      if (this.isDaemonRunning) {
        await invoke('stop_daemon');
        this.addActivity('scan', 'Daemon monitoring stopped');
      } else {
        await invoke('start_daemon', { 
          interval: this.scanInterval,
          dryRun: this.isDryRun 
        });
        this.addActivity('scan', 'Daemon monitoring started');
      }
    } catch (error) {
      console.error('Failed to toggle daemon:', error);
      this.addActivity('error', 'Daemon toggle failed: ' + error);
    }
  }

  async refreshStats() {
    try {
      const newStats = await invoke('get_system_stats') as SystemStats;
      this.stats = newStats;
    } catch (error) {
      console.error('Failed to refresh stats:', error);
    }
  }

  async emergencyStop() {
    try {
      await invoke('emergency_stop');
      this.isDaemonRunning = false;
      this.isScanning = false;
      this.systemStatus = 'STOPPED';
      this.addActivity('error', 'Emergency stop activated');
    } catch (error) {
      console.error('Emergency stop failed:', error);
    }
  }

  clearActivity() {
    this.recentActivities = [];
  }

  private async loadRecentActivity() {
    try {
      const activities = await invoke('get_recent_activities') as RecentActivity[];
      this.recentActivities = activities.slice(0, 50); // Keep last 50 activities
    } catch (error) {
      console.error('Failed to load recent activities:', error);
    }
  }

  private async checkSystemHealth() {
    try {
      const health = await invoke('get_system_health') as any;
      this.scannerHealth = health.scanner;
      this.policyEngineHealth = health.policy_engine;
      this.killSystemHealth = health.kill_system;
      this.loggingHealth = health.logging;
    } catch (error) {
      console.error('Failed to check system health:', error);
    }
  }

  private handleScanEvent(data: any) {
    switch (data.event) {
      case 'SCAN_START':
        this.addActivity('scan', 'System scan started');
        break;
      case 'DETECTED':
        this.stats.detectedThreats++;
        this.addActivity('detection', `Threat detected: ${data.name}`, data.name, data.pid);
        break;
      case 'KILL_SUCCESS':
        this.stats.killedProcesses++;
        this.addActivity('kill', `Process terminated: ${data.name}`, data.name, data.pid);
        break;
      case 'KILL_FAIL':
        this.addActivity('error', `Failed to terminate: ${data.name} - ${data.reason}`, data.name, data.pid);
        break;
    }
  }

  private updateHealthMetrics(health: any) {
    this.scannerHealth = health.scanner || 'UNKNOWN';
    this.policyEngineHealth = health.policy_engine || 'UNKNOWN';
    this.killSystemHealth = health.kill_system || 'UNKNOWN';
    this.loggingHealth = health.logging || 'UNKNOWN';
  }

  private addActivity(type: RecentActivity['type'], message: string, processName?: string, pid?: number) {
    const activity: RecentActivity = {
      id: Date.now().toString(),
      timestamp: new Date().toISOString(),
      type,
      message,
      processName,
      pid
    };

    this.recentActivities.unshift(activity);
    
    // Keep only last 100 activities
    if (this.recentActivities.length > 100) {
      this.recentActivities = this.recentActivities.slice(0, 100);
    }
  }

  formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  }

  formatTime(timestamp: string): string {
    return new Date(timestamp).toLocaleTimeString();
  }
 } 