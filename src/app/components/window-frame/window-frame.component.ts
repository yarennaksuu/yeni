import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

@Component({
  selector: 'app-window-frame',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="window-frame">
      <div class="titlebar" data-tauri-drag-region>
        <div class="titlebar-content">
          <div class="window-title">
            <img src="assets/icons/edr-logo.svg" alt="EDR" class="app-icon">
            <span>EDR Kill Switch</span>
          </div>
          <div class="window-controls">
            <button 
              class="control-button minimize-btn"
              (click)="minimizeWindow()"
              title="Minimize">
              <svg width="10" height="1" viewBox="0 0 10 1">
                <rect width="10" height="1" fill="currentColor"/>
              </svg>
            </button>
            <button 
              class="control-button close-btn"
              (click)="closeWindow()"
              title="Close">
              <svg width="10" height="10" viewBox="0 0 10 10">
                <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1"/>
                <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
      <ng-content></ng-content>
    </div>
  `,
  styles: [`
    .window-frame {
      width: 100%;
      height: 100vh;
      background: #1a1a1a;
      border-radius: 12px;
      overflow: hidden;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
      display: flex;
      flex-direction: column;
    }

    .titlebar {
      height: 32px;
      background: rgba(255, 255, 255, 0.05);
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
      display: flex;
      align-items: center;
      user-select: none;
      cursor: grab;
    }

    .titlebar:active {
      cursor: grabbing;
    }

    .titlebar-content {
      width: 100%;
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0 16px;
    }

    .window-title {
      display: flex;
      align-items: center;
      gap: 8px;
      color: #fff;
      font-size: 13px;
      font-weight: 500;
    }

    .app-icon {
      width: 16px;
      height: 16px;
    }

    .window-controls {
      display: flex;
      gap: 8px;
    }

    .control-button {
      width: 12px;
      height: 12px;
      border: none;
      background: transparent;
      color: #999;
      cursor: pointer;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .minimize-btn:hover {
      background: #ffc107;
      color: #000;
    }

    .close-btn:hover {
      background: #dc3545;
      color: #fff;
    }

    .control-button svg {
      pointer-events: none;
    }
  `]
})
export class WindowFrameComponent implements OnInit {
  
  ngOnInit() {
    // Initialize window
  }

  async minimizeWindow() {
    const appWindow = getCurrentWindow();
    await appWindow.minimize();
  }

  async closeWindow() {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  }
}