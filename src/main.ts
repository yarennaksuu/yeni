import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { provideRouter } from '@angular/router';
import { importProvidersFrom } from '@angular/core';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';

import { SettingsComponent } from './app/components/settings/settings.component';

const routes = [
  { 
    path: '', 
    redirectTo: '/settings', 
    pathMatch: 'full' 
  },
  {
    path: 'settings',
    component: SettingsComponent,
    title: 'Settings'
  },
  {
    path: '**',
    redirectTo: '/settings'
  }
];

bootstrapApplication(AppComponent, {
  providers: [
  importProvidersFrom(FormsModule, ReactiveFormsModule, CommonModule)
  ]
});