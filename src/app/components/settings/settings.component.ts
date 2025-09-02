
import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule, FormBuilder, FormGroup } from '@angular/forms';
import { ThemeService, Theme } from '../../services/theme.service';
import { LanguageService, Language, LanguageOption } from '../../services/language.service';
import { NotificationService } from '../../services/notification.service';

@Component({
  selector: 'app-settings',
  standalone: true,
  templateUrl: './settings.component.html',
  styleUrls: ['./settings.component.css'],
  imports: [CommonModule, FormsModule, ReactiveFormsModule],
}) 

  export class SettingsComponent implements OnInit {
  currentTheme: Theme = 'dark';
  currentLanguage: Language = 'en';
  supportedLanguages: LanguageOption[] = [];

  notificationForm: FormGroup;
  applicationForm: FormGroup;

  constructor(
    private fb: FormBuilder,
    private themeService: ThemeService,
    private languageService: LanguageService,
    private notificationService: NotificationService
  ) {
    this.notificationForm = this.fb.group({
      enabled: [true],
      native: [true],
      sound: [true],
      infoEnabled: [true],
      successEnabled: [true],
      warningEnabled: [true],
      errorEnabled: [true]
    });

    this.applicationForm = this.fb.group({
      autoRefreshInterval: [30],
      scanTimeout: [5000],
      confirmActions: [true],
      showSystemProcesses: [false]
    });
  }

  ngOnInit() {
    this.initializeSettings();
    this.setupFormSubscriptions();
  }

  private initializeSettings() {
    // Theme
    this.themeService.theme$.subscribe(theme => {
      this.currentTheme = theme;
    });

    // Language
    this.languageService.language$.subscribe(language => {
      this.currentLanguage = language;
    });
    this.supportedLanguages = this.languageService.supportedLanguages;

    // Notification settings
    this.notificationService.settings$.subscribe(settings => {
      this.notificationForm.patchValue({
        enabled: settings.enabled,
        native: settings.native,
        sound: settings.sound,
        infoEnabled: settings.types.info,
        successEnabled: settings.types.success,
        warningEnabled: settings.types.warning,
        errorEnabled: settings.types.error
      });
    });

    // Load application settings from localStorage
    const appSettings = localStorage.getItem('edr-app-settings');
    if (appSettings) {
      this.applicationForm.patchValue(JSON.parse(appSettings));
    }
  }

  private setupFormSubscriptions() {
    // Notification form changes
    this.notificationForm.valueChanges.subscribe(values => {
      this.notificationService.updateSettings({
        enabled: values.enabled,
        native: values.native,
        sound: values.sound,
        types: {
          info: values.infoEnabled,
          success: values.successEnabled,
          warning: values.warningEnabled,
          error: values.errorEnabled
        }
      });
    });

    // Application form changes
    this.applicationForm.valueChanges.subscribe(values => {
      localStorage.setItem('edr-app-settings', JSON.stringify(values));
    });
  }

  setTheme(theme: Theme) {
    this.themeService.setTheme(theme);
  }

  setLanguage(language: Language) {
    this.languageService.setLanguage(language);
  }

  testNotification(type: 'info' | 'success' | 'warning' | 'error') {
    const messages = {
      info: {
        title: 'Test Information',
        body: 'This is a test information notification.'
      },
      success: {
        title: 'Test Success',
        body: 'This is a test success notification.'
      },
      warning: {
        title: 'Test Warning',
        body: 'This is a test warning notification.'
      },
      error: {
        title: 'Test Error',
        body: 'This is a test error notification.'
      }
    };

    const message = messages[type];
    this.notificationService.show({
      type,
      title: message.title,
      body: message.body,
      duration: 3000
    });
  }

  resetToDefaults() {
    if (confirm(this.translate('settings.confirmReset'))) {
      // Reset theme
      this.setTheme('dark');
      
      // Reset language
      this.setLanguage('en');
      
      // Reset notification settings
      this.notificationForm.reset({
        enabled: true,
        native: true,
        sound: true,
        infoEnabled: true,
        successEnabled: true,
        warningEnabled: true,
        errorEnabled: true
      });
      
      // Reset application settings
      this.applicationForm.reset({
        autoRefreshInterval: 30,
        scanTimeout: 5000,
        confirmActions: true,
        showSystemProcesses: false
      });

      this.notificationService.success(
        'Settings Reset',
        'All settings have been reset to defaults'
      );
    }
  }

  exportSettings() {
    const settings = {
      theme: this.currentTheme,
      language: this.currentLanguage,
      notifications: this.notificationForm.value,
      application: this.applicationForm.value,
      exportDate: new Date().toISOString()
    };

    const blob = new Blob([JSON.stringify(settings, null, 2)], {
      type: 'application/json'
    });
    
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `edr-settings-${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    window.URL.revokeObjectURL(url);

    this.notificationService.success(
      'Settings Exported',
      'Settings have been exported successfully'
    );
  }

  importSettings() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    
    input.onchange = (event: any) => {
      const file = event.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = (e: any) => {
        try {
          const settings = JSON.parse(e.target.result);
          
          // Validate and apply settings
          if (settings.theme) {
            this.setTheme(settings.theme);
          }
          
          if (settings.language) {
            this.setLanguage(settings.language);
          }
          
          if (settings.notifications) {
            this.notificationForm.patchValue(settings.notifications);
          }
          
          if (settings.application) {
            this.applicationForm.patchValue(settings.application);
          }

          this.notificationService.success(
            'Settings Imported',
            'Settings have been imported successfully'
          );
        } catch (error) {
          this.notificationService.error(
            'Import Failed',
            'Failed to import settings: Invalid file format'
          );
        }
      };
      
      reader.readAsText(file);
    };
    
    input.click();
  }

  translate(key: string): string {
    return this.languageService.translate(key);
  }
}