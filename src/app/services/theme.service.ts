import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

export type Theme = 'light' | 'dark' | 'auto';

@Injectable({
  providedIn: 'root'
})
export class ThemeService {
  private readonly THEME_KEY = 'edr-theme-preference';
  private currentTheme = new BehaviorSubject<Theme>('dark');
  
  theme$ = this.currentTheme.asObservable();

  constructor() {
    this.loadTheme();
    this.setupSystemThemeListener();
  }

  private loadTheme() {
    const savedTheme = localStorage.getItem(this.THEME_KEY) as Theme;
    if (savedTheme) {
      this.setTheme(savedTheme);
    } else {
      this.setTheme('dark');
    }
  }

  setTheme(theme: Theme) {
    this.currentTheme.next(theme);
    localStorage.setItem(this.THEME_KEY, theme);
    this.applyTheme(theme);
  }

  private applyTheme(theme: Theme) {
    const root = document.documentElement;
    root.classList.remove('light-theme', 'dark-theme');
    
    if (theme === 'auto') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.add(prefersDark ? 'dark-theme' : 'light-theme');
    } else {
      root.classList.add(`${theme}-theme`);
    }
  }

  private setupSystemThemeListener() {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      if (this.currentTheme.value === 'auto') {
        this.applyTheme('auto');
      }
    });
  }

  getCurrentTheme(): Theme {
    return this.currentTheme.value;
  }

  toggleTheme() {
    const current = this.currentTheme.value;
    const next = current === 'light' ? 'dark' : current === 'dark' ? 'auto' : 'light';
    this.setTheme(next);
  }
}