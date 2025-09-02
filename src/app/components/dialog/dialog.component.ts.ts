import { Component, Input, Output, EventEmitter, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { trigger, transition, style, animate } from '@angular/animations';

export interface DialogConfig {
  title?: string;
  message?: string;
  type?: 'info' | 'warning' | 'error' | 'success' | 'confirm';
  showCancel?: boolean;
  confirmText?: string;
  cancelText?: string;
  width?: string;
  height?: string;
  icon?: string;
}

@Component({
  selector: 'app-dialog',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div 
      class="dialog-overlay" 
      [@slideIn]="'in'"
      (click)="onOverlayClick($event)">
      <div 
        class="dialog-container"
        [style.width]="config.width || 'auto'"
        [style.height]="config.height || 'auto'"
        (click)="$event.stopPropagation()">
        
        <div class="dialog-header">
          <div class="dialog-title-section">
            <div class="dialog-icon" *ngIf="getIcon()">
              <i [class]="getIcon()" [ngClass]="getIconClass()"></i>
            </div>
            <h3 class="dialog-title">{{ config.title || 'Dialog' }}</h3>
          </div>
          <button 
            class="dialog-close-btn"
            (click)="onCancel()"
            title="Close">
            ×
          </button>
        </div>

        <div class="dialog-content">
          <div class="dialog-message" *ngIf="config.message">
            {{ config.message }}
          </div>
          <ng-content></ng-content>
        </div>

        <div class="dialog-actions" *ngIf="config.showCancel || config.confirmText">
          <button 
            class="dialog-btn dialog-btn-cancel"
            *ngIf="config.showCancel"
            (click)="onCancel()">
            {{ config.cancelText || 'Cancel' }}
          </button>
          <button 
            class="dialog-btn dialog-btn-confirm"
            [ngClass]="getConfirmButtonClass()"
            (click)="onConfirm()">
            {{ config.confirmText || 'OK' }}
          </button>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .dialog-overlay {
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: rgba(0, 0, 0, 0.7);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 10000;
      backdrop-filter: blur(4px);
    }

    .dialog-container {
      background: #2d2d2d;
      border-radius: 12px;
      min-width: 320px;
      max-width: 90vw;
      max-height: 90vh;
      overflow: hidden;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
      border: 1px solid #404040;
    }

    .dialog-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 20px 24px;
      background: rgba(255, 255, 255, 0.03);
      border-bottom: 1px solid #404040;
    }

    .dialog-title-section {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .dialog-icon {
      width: 24px;
      height: 24px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 50%;
      font-size: 14px;
    }

    .dialog-icon.info { background: rgba(23, 162, 184, 0.2); color: #17a2b8; }
    .dialog-icon.warning { background: rgba(255, 193, 7, 0.2); color: #ffc107; }
    .dialog-icon.error { background: rgba(220, 53, 69, 0.2); color: #dc3545; }
    .dialog-icon.success { background: rgba(40, 167, 69, 0.2); color: #28a745; }

    .dialog-title {
      margin: 0;
      color: #fff;
      font-size: 18px;
      font-weight: 600;
    }

    .dialog-close-btn {
      width: 28px;
      height: 28px;
      border: none;
      background: transparent;
      color: #999;
      cursor: pointer;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 20px;
      line-height: 1;
      transition: all 0.2s ease;
    }

    .dialog-close-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      color: #fff;
    }

    .dialog-content {
      padding: 24px;
      color: #ccc;
      line-height: 1.6;
      overflow-y: auto;
      max-height: 60vh;
    }

    .dialog-message {
      font-size: 16px;
      margin-bottom: 16px;
    }

    .dialog-actions {
      display: flex;
      justify-content: flex-end;
      gap: 12px;
      padding: 20px 24px;
      border-top: 1px solid #404040;
      background: rgba(255, 255, 255, 0.02);
    }

    .dialog-btn {
      padding: 10px 20px;
      border: none;
      border-radius: 6px;
      cursor: pointer;
      font-weight: 600;
      font-size: 14px;
      transition: all 0.2s ease;
      min-width: 80px;
    }

    .dialog-btn-cancel {
      background: #666;
      color: white;
    }

    .dialog-btn-cancel:hover {
      background: #777;
    }

    .dialog-btn-confirm {
      background: #ff6b35;
      color: white;
    }

    .dialog-btn-confirm:hover {
      background: #e55a2b;
    }

    .dialog-btn-confirm.info {
      background: #17a2b8;
    }

    .dialog-btn-confirm.info:hover {
      background: #138496;
    }

    .dialog-btn-confirm.success {
      background: #28a745;
    }

    .dialog-btn-confirm.success:hover {
      background: #218838;
    }

    .dialog-btn-confirm.warning {
      background: #ffc107;
      color: #000;
    }

    .dialog-btn-confirm.warning:hover {
      background: #e0a800;
    }

    .dialog-btn-confirm.error {
      background: #dc3545;
    }

    .dialog-btn-confirm.error:hover {
      background: #c82333;
    }
  `],
  animations: [
    trigger('slideIn', [
      transition(':enter', [
        style({ opacity: 0, transform: 'scale(0.9)' }),
        animate('200ms ease-out', style({ opacity: 1, transform: 'scale(1)' }))
      ]),
      transition(':leave', [
        animate('150ms ease-in', style({ opacity: 0, transform: 'scale(0.9)' }))
      ])
    ])
  ]
})
export class DialogComponent implements OnDestroy {
  @Input() config: DialogConfig = {};
  @Output() confirmed = new EventEmitter<any>();
  @Output() cancelled = new EventEmitter<void>();
  @Output() closed = new EventEmitter<void>();

  ngOnDestroy() {
    document.body.style.overflow = 'auto';
  }

  onConfirm() {
    this.confirmed.emit(true);
    this.closed.emit();
  }

  onCancel() {
    this.cancelled.emit();
    this.closed.emit();
  }

  onOverlayClick(event: Event) {
    if (event.target === event.currentTarget) {
      this.onCancel();
    }
  }

  getIcon(): string {
    const icons = {
      info: 'fas fa-info',
      warning: 'fas fa-exclamation-triangle',
      error: 'fas fa-times-circle',
      success: 'fas fa-check-circle',
      confirm: 'fas fa-question-circle'
    };
    return this.config.icon || icons[this.config.type || 'info'];
  }

  getIconClass(): string {
    return this.config.type || 'info';
  }

  getConfirmButtonClass(): string {
    return this.config.type || 'info';
  }
}

@Component({
  selector: 'app-dialog-service',
  template: ''
})
export class DialogService {
  private dialogContainer: HTMLElement | null = null;

  constructor() {
    this.createDialogContainer();
  }

  private createDialogContainer() {
    this.dialogContainer = document.createElement('div');
    this.dialogContainer.id = 'dialog-container';
    document.body.appendChild(this.dialogContainer);
  }

  show(config: DialogConfig): Promise<boolean> {
    return new Promise((resolve) => {
      const dialogRef = this.createDialog(config);
      
      dialogRef.confirmed.subscribe(() => {
        resolve(true);
        this.destroyDialog(dialogRef);
      });

      dialogRef.cancelled.subscribe(() => {
        resolve(false);
        this.destroyDialog(dialogRef);
      });
    });
  }

  info(message: string, title?: string): Promise<boolean> {
    return this.show({
      type: 'info',
      title: title || 'Information',
      message,
      confirmText: 'OK'
    });
  }

  warning(message: string, title?: string): Promise<boolean> {
    return this.show({
      type: 'warning',
      title: title || 'Warning',
      message,
      confirmText: 'OK'
    });
  }

  error(message: string, title?: string): Promise<boolean> {
    return this.show({
      type: 'error',
      title: title || 'Error',
      message,
      confirmText: 'OK'
    });
  }

  success(message: string, title?: string): Promise<boolean> {
    return this.show({
      type: 'success',
      title: title || 'Success',
      message,
      confirmText: 'OK'
    });
  }

  confirm(message: string, title?: string): Promise<boolean> {
    return this.show({
      type: 'confirm',
      title: title || 'Confirm',
      message,
      showCancel: true,
      confirmText: 'Yes',
      cancelText: 'No'
    });
  }

  private createDialog(config: DialogConfig): DialogComponent {
    // Bu implementasyon Angular'ın dynamic component creation 
    // yöntemlerini kullanarak yapılacak
    const dialog = new DialogComponent();
    dialog.config = config;
    return dialog;
  }

  private destroyDialog(dialogRef: DialogComponent) {
    // Dialog'u DOM'dan kaldır
    setTimeout(() => {
      // Cleanup logic
    }, 200);
  }
}