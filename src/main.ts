import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { provideRouter } from '@angular/router';
import { importProvidersFrom } from '@angular/core';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';
import { routes } from './app/app.routes';

bootstrapApplication(AppComponent, {
  providers: [
  provideRouter(routes),
    importProvidersFrom(FormsModule, ReactiveFormsModule, CommonModule)  ]
});