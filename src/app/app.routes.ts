import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    redirectTo: '/dashboard',
    pathMatch: 'full'
  },
  {
    path: 'dashboard',
    loadComponent: () => import('./components/dashboard/dashboard.component').then(m => m.DashboardComponent)
  },
  {
    path: 'scanner',
    loadComponent: () => import('./components/process-scanner/process-scanner.component').then(m => m.ProcessScannerComponent)
  },
  {
    path: 'policies',
    loadComponent: () => import('./components/policy-editor/policy-editor.component').then(m => m.PolicyEditorComponent)
  },
  {
    path: 'logs',
    loadComponent: () => import('./components/log-viewer/log-viewer.component').then(m => m.LogViewerComponent)
  },
  {
    path: 'settings',
    loadComponent: () => import('./components/settings/settings.component').then(m => m.SettingsComponent)
  },
  {
    path: '**',
    redirectTo: '/dashboard'
  }
];