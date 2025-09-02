import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

export type Language = 'en' | 'tr' | 'de' | 'fr' | 'es';

export interface LanguageOption {
  code: Language;
  name: string;
  nativeName: string;
  flag: string;
}

@Injectable({
  providedIn: 'root'
})
export class LanguageService {
  private readonly LANGUAGE_KEY = 'edr-language-preference';
  private currentLanguage = new BehaviorSubject<Language>('en');
  
  language$ = this.currentLanguage.asObservable();

  readonly supportedLanguages: LanguageOption[] = [
    { code: 'en', name: 'English', nativeName: 'English', flag: '🇺🇸' },
    { code: 'tr', name: 'Turkish', nativeName: 'Türkçe', flag: '🇹🇷' },
    { code: 'de', name: 'German', nativeName: 'Deutsch', flag: '🇩🇪' },
    { code: 'fr', name: 'French', nativeName: 'Français', flag: '🇫🇷' },
    { code: 'es', name: 'Spanish', nativeName: 'Español', flag: '🇪🇸' }
  ];

  private translations: { [key: string]: { [lang: string]: string } } = {
    // Navigation
    'nav.dashboard': {
      'en': 'Dashboard',
      'tr': 'Kontrol Paneli',
      'de': 'Dashboard',
      'fr': 'Tableau de bord',
      'es': 'Panel de control'
    },
    'nav.scanner': {
      'en': 'Process Scanner',
      'tr': 'İşlem Tarayıcısı',
      'de': 'Prozess-Scanner',
      'fr': 'Scanner de processus',
      'es': 'Escáner de procesos'
    },
    'nav.policies': {
      'en': 'Policies',
      'tr': 'Politikalar',
      'de': 'Richtlinien',
      'fr': 'Politiques',
      'es': 'Políticas'
    },
    'nav.logs': {
      'en': 'System Logs',
      'tr': 'Sistem Kayıtları',
      'de': 'Systemprotokolle',
      'fr': 'Journaux système',
      'es': 'Registros del sistema'
    },
    'nav.settings': {
      'en': 'Settings',
      'tr': 'Ayarlar',
      'de': 'Einstellungen',
      'fr': 'Paramètres',
      'es': 'Configuración'
    },
    
    // Common actions
    'action.save': {
      'en': 'Save',
      'tr': 'Kaydet',
      'de': 'Speichern',
      'fr': 'Enregistrer',
      'es': 'Guardar'
    },
    'action.cancel': {
      'en': 'Cancel',
      'tr': 'İptal',
      'de': 'Abbrechen',
      'fr': 'Annuler',
      'es': 'Cancelar'
    },
    'action.delete': {
      'en': 'Delete',
      'tr': 'Sil',
      'de': 'Löschen',
      'fr': 'Supprimer',
      'es': 'Eliminar'
    },
    'action.edit': {
      'en': 'Edit',
      'tr': 'Düzenle',
      'de': 'Bearbeiten',
      'fr': 'Modifier',
      'es': 'Editar'
    },
    'action.refresh': {
      'en': 'Refresh',
      'tr': 'Yenile',
      'de': 'Aktualisieren',
      'fr': 'Actualiser',
      'es': 'Actualizar'
    },
    
    // Dashboard
    'dashboard.totalProcesses': {
      'en': 'Total Processes',
      'tr': 'Toplam İşlemler',
      'de': 'Gesamtprozesse',
      'fr': 'Processus totaux',
      'es': 'Procesos totales'
    },
    'dashboard.threatsDetected': {
      'en': 'Threats Detected',
      'tr': 'Tespit Edilen Tehditler',
      'de': 'Erkannte Bedrohungen',
      'fr': 'Menaces détectées',
      'es': 'Amenazas detectadas'
    },
    'dashboard.processesTerminated': {
      'en': 'Processes Terminated',
      'tr': 'Sonlandırılan İşlemler',
      'de': 'Beendete Prozesse',
      'fr': 'Processus terminés',
      'es': 'Procesos terminados'
    },
    'dashboard.systemUptime': {
      'en': 'System Uptime',
      'tr': 'Sistem Çalışma Süresi',
      'de': 'System-Betriebszeit',
      'fr': 'Temps de fonctionnement',
      'es': 'Tiempo de actividad'
    },
    
    // Process Scanner
    'scanner.startScan': {
      'en': 'Start Scan',
      'tr': 'Taramayı Başlat',
      'de': 'Scan starten',
      'fr': 'Démarrer le scan',
      'es': 'Iniciar escaneo'
    },
    'scanner.killProcess': {
      'en': 'Terminate Process',
      'tr': 'İşlemi Sonlandır',
      'de': 'Prozess beenden',
      'fr': 'Terminer le processus',
      'es': 'Terminar proceso'
    },
    
    // Settings
    'settings.general': {
      'en': 'General',
      'tr': 'Genel',
      'de': 'Allgemein',
      'fr': 'Général',
      'es': 'General'
    },
    'settings.language': {
      'en': 'Language',
      'tr': 'Dil',
      'de': 'Sprache',
      'fr': 'Langue',
      'es': 'Idioma'
    },
    'settings.theme': {
      'en': 'Theme',
      'tr': 'Tema',
      'de': 'Design',
      'fr': 'Thème',
      'es': 'Tema'
    },
    'settings.notifications': {
      'en': 'Notifications',
      'tr': 'Bildirimler',
      'de': 'Benachrichtigungen',
      'fr': 'Notifications',
      'es': 'Notificaciones'
    },
    
    // Status
    'status.active': {
      'en': 'System Active',
      'tr': 'Sistem Aktif',
      'de': 'System Aktiv',
      'fr': 'Système Actif',
      'es': 'Sistema Activo'
    },
    'status.inactive': {
      'en': 'System Inactive',
      'tr': 'Sistem İnaktif',
      'de': 'System Inaktiv',
      'fr': 'Système Inactif',
      'es': 'Sistema Inactivo'
    },

    // Notifications
    'notifications.title': {
      'en': 'Notifications',
      'tr': 'Bildirimler',
      'de': 'Benachrichtigungen',
      'fr': 'Notifications',
      'es': 'Notificaciones'
    },
    'notifications.empty': {
      'en': 'No notifications',
      'tr': 'Bildirim yok',
      'de': 'Keine Benachrichtigungen',
      'fr': 'Aucune notification',
      'es': 'Sin notificaciones'
    },
    'notifications.info': {
      'en': 'Information',
      'tr': 'Bilgi',
      'de': 'Information',
      'fr': 'Information',
      'es': 'Información'
    },
    'notifications.success': {
      'en': 'Success',
      'tr': 'Başarılı',
      'de': 'Erfolg',
      'fr': 'Succès',
      'es': 'Éxito'
    },
    'notifications.warning': {
      'en': 'Warning',
      'tr': 'Uyarı',
      'de': 'Warnung',
      'fr': 'Avertissement',
      'es': 'Advertencia'
    },
    'notifications.error': {
      'en': 'Error',
      'tr': 'Hata',
      'de': 'Fehler',
      'fr': 'Erreur',
      'es': 'Error'
    }
  };

  constructor() {
    this.loadLanguage();
  }

  private loadLanguage() {
    const savedLanguage = localStorage.getItem(this.LANGUAGE_KEY) as Language;
    if (savedLanguage && this.supportedLanguages.find(l => l.code === savedLanguage)) {
      this.setLanguage(savedLanguage);
    } else {
      // Auto-detect browser language
      const browserLang = navigator.language.split('-')[0] as Language;
      const supportedLang = this.supportedLanguages.find(l => l.code === browserLang);
      this.setLanguage(supportedLang?.code || 'en');
    }
  }

  setLanguage(language: Language) {
    this.currentLanguage.next(language);
    localStorage.setItem(this.LANGUAGE_KEY, language);
    document.documentElement.lang = language;
  }

  getCurrentLanguage(): Language {
    return this.currentLanguage.value;
  }

  translate(key: string, params?: { [key: string]: string }): string {
    const translation = this.translations[key];
    if (!translation) {
      console.warn(`Translation missing for key: ${key}`);
      return key;
    }

    const currentLang = this.getCurrentLanguage();
    let text = translation[currentLang] || translation['en'] || key;

    // Replace parameters
    if (params) {
      Object.keys(params).forEach(param => {
        text = text.replace(`{{${param}}}`, params[param]);
      });
    }

    return text;
  }

  getLanguageInfo(code: Language): LanguageOption | undefined {
    return this.supportedLanguages.find(lang => lang.code === code);
  }
}