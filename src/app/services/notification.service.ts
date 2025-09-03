import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';
import { sendNotification } from '@tauri-apps/plugin-notification';

export interface NotificationConfig {
  title: string;
  body: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  persistent?: boolean;
  showInApp?: boolean;
  showNative?: boolean;
  icon?: string;
  actions?: NotificationAction[];
}

export interface NotificationAction {
  id: string;
  title: string;
  icon?: string;
}

export interface InAppNotification extends NotificationConfig {
  id: string;
  timestamp: Date;
  read: boolean;
}

@Injectable({
  providedIn: 'root'
})
export class NotificationService {
showSuccess(message: string) {
    return this.success('Success', message);
  }
  showError(message: string) {
    return this.error('Error', message);
  }
  private readonly SETTINGS_KEY = 'edr-notification-settings';
  private notifications = new BehaviorSubject<InAppNotification[]>([]);
  private settings = new BehaviorSubject({
    enabled: true,
    native: true,
    inApp: true,
    sound: true,
    types: {
      info: true,
      success: true,
      warning: true,
      error: true
    }
  });

  notifications$ = this.notifications.asObservable();
  settings$ = this.settings.asObservable();

  constructor() {
    this.loadSettings();
    this.requestPermission();
  }

  private loadSettings() {
    const saved = localStorage.getItem(this.SETTINGS_KEY);
    if (saved) {
      this.settings.next({ ...this.settings.value, ...JSON.parse(saved) });
    }
  }

  private saveSettings() {
    localStorage.setItem(this.SETTINGS_KEY, JSON.stringify(this.settings.value));
  }

  async requestPermission(): Promise<boolean> {
    if ('Notification' in window) {
      const permission = await Notification.requestPermission();
      return permission === 'granted';
    }
    return false;
  }

  async show(config: NotificationConfig): Promise<void> {
    const settings = this.settings.value;
    
    if (!settings.enabled || !settings.types[config.type || 'info']) {
      return;
    }

    // Generate unique ID
    const notification: InAppNotification = {
      ...config,
      id: Date.now().toString(),
      timestamp: new Date(),
      read: false,
      showInApp: config.showInApp ?? settings.inApp,
      showNative: config.showNative ?? settings.native,
      duration: config.duration || 5000
    };

    // Show in-app notification
    if (notification.showInApp) {
      this.addInAppNotification(notification);
    }

    // Show native notification
    if (notification.showNative) {
      await this.showNativeNotification(notification);
    }

    // Play sound
    if (settings.sound) {
      this.playNotificationSound(config.type || 'info');
    }
  }

  private addInAppNotification(notification: InAppNotification) {
    const current = this.notifications.value;
    this.notifications.next([notification, ...current]);

    // Auto-remove after duration
    if (!notification.persistent && notification.duration) {
      setTimeout(() => {
        this.removeNotification(notification.id);
      }, notification.duration);
    }
  }

  private async showNativeNotification(notification: InAppNotification) {
    try {
      await sendNotification({
        title: notification.title,
        body: notification.body,
        icon: notification.icon || this.getDefaultIcon(notification.type || 'info')
      });
    } catch (error) {
      console.warn('Native notification failed:', error);
    }
  }

  private playNotificationSound(type: string) {
    const sounds = {
      info: '/assets/sounds/info.mp3',
      success: '/assets/sounds/success.mp3',
      warning: '/assets/sounds/warning.mp3',
      error: '/assets/sounds/error.mp3'
    };

    const audio = new Audio(sounds[type as keyof typeof sounds] || sounds.info);
    audio.volume = 0.3;
    audio.play().catch(() => {
      // Sound play failed - ignore silently
    });
  }

  private getDefaultIcon(type: string): string {
    const icons = {
      info: '/assets/icons/info.png',
      success: '/assets/icons/success.png',
      warning: '/assets/icons/warning.png',
      error: '/assets/icons/error.png'
    };
    return icons[type as keyof typeof icons] || icons.info;
  }

  removeNotification(id: string) {
    const current = this.notifications.value;
    this.notifications.next(current.filter(n => n.id !== id));
  }

  markAsRead(id: string) {
    const current = this.notifications.value;
    const updated = current.map(n => 
      n.id === id ? { ...n, read: true } : n
    );
    this.notifications.next(updated);
  }

  markAllAsRead() {
    const current = this.notifications.value;
    const updated = current.map(n => ({ ...n, read: true }));
    this.notifications.next(updated);
  }

  clearAll() {
    this.notifications.next([]);
  }

  clearRead() {
    const current = this.notifications.value;
    this.notifications.next(current.filter(n => !n.read));
  }

  updateSettings(newSettings: Partial<typeof this.settings.value>) {
    this.settings.next({ ...this.settings.value, ...newSettings });
    this.saveSettings();
  }

  getUnreadCount(): number {
    return this.notifications.value.filter(n => !n.read).length;
  }

  // Convenience methods
  info(title: string, body: string, options?: Partial<NotificationConfig>) {
    return this.show({ title, body, type: 'info', ...options });
  }

  success(title: string, body: string, options?: Partial<NotificationConfig>) {
    return this.show({ title, body, type: 'success', ...options });
  }

  warning(title: string, body: string, options?: Partial<NotificationConfig>) {
    return this.show({ title, body, type: 'warning', ...options });
  }

  error(title: string, body: string, options?: Partial<NotificationConfig>) {
    return this.show({ title, body, type: 'error', persistent: true, ...options });
  }

  threatDetected(processName: string, ruleName: string) {
    return this.error(
      'Threat Detected',
      `Process "${processName}" matched rule: ${ruleName}`,
      {
        persistent: true,
        actions: [
          { id: 'view_logs', title: 'View Logs' },
          { id: 'kill_process', title: 'Terminate' }
        ]
      }
    );
  }

  processKilled(processName: string, success: boolean) {
    if (success) {
      return this.success(
        'Process Terminated',
        `Successfully terminated "${processName}"`
      );
    } else {
      return this.error(
        'Termination Failed',
        `Failed to terminate "${processName}"`
      );
    }
  }

  scanComplete(scannedCount: number, threatsFound: number) {
    if (threatsFound > 0) {
      return this.warning(
        'Scan Complete',
        `Scanned ${scannedCount} processes, found ${threatsFound} threats`
      );
    } else {
      return this.success(
        'Scan Complete',
        `Scanned ${scannedCount} processes, no threats found`
      );
    }
  }
}