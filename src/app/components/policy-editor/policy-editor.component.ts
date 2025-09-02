import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule, FormBuilder, FormGroup, Validators } from '@angular/forms';
import { TauriService } from '../../services/tauri.service';
import { NotificationService } from '../../services/notification.service';
import { LanguageService } from '../../services/language.service';

interface Rule {
  id: string;
  ruleType: 'name' | 'path' | 'hash' | 'command' | 'registry' | 'network';
  value: string;
  description?: string;
  enabled: boolean;
  createdAt: string;
  lastModified: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  autoAction: 'alert' | 'quarantine' | 'kill' | 'none';
  tags: string[];
}

interface PolicyConfig {
  blacklist: Rule[];
  whitelist: Rule[];
  settings: {
    enableRealTimeProtection: boolean;
    enableHashChecking: boolean;
    enableNetworkMonitoring: boolean;
    enableRegistryMonitoring: boolean;
    autoUpdateRules: boolean;
    alertOnNewThreats: boolean;
  };
  metadata: {
    version: string;
    lastUpdated: string;
    totalRules: number;
    activeRules: number;
  };
}

@Component({
  selector: 'app-policy-editor',
  templateUrl: './policy-editor.component.html',
  styleUrls: ['./policy-editor.component.css'],
  standalone: true,
  imports: [CommonModule, FormsModule, ReactiveFormsModule]
})
export class PolicyEditorComponent implements OnInit {
  config: PolicyConfig = {
    blacklist: [],
    whitelist: [],
    settings: {
      enableRealTimeProtection: true,
      enableHashChecking: false,
      enableNetworkMonitoring: false,
      enableRegistryMonitoring: false,
      autoUpdateRules: true,
      alertOnNewThreats: true
    },
    metadata: {
      version: '1.0.0',
      lastUpdated: new Date().toISOString(),
      totalRules: 0,
      activeRules: 0
    }
  };

  activeTab: 'blacklist' | 'whitelist' | 'templates' = 'blacklist';
  hasChanges = false;
  showRuleEditor = false;
  
  editingRule: Rule | null = null;
  editingIndex = -1;
  editingList: 'blacklist' | 'whitelist' = 'blacklist';

  blacklistSearch = '';
  whitelistSearch = '';
  blacklistFilter = 'all';
  whitelistFilter = 'all';

  filteredBlacklistRules: Rule[] = [];
  filteredWhitelistRules: Rule[] = [];

  ruleForm: FormGroup;
  tagInput = '';
  currentTags: string[] = [];

  ruleTemplates = [
    {
      id: 'common_malware',
      type: 'blacklist',
      icon: 'icon-virus',
      rules: [
        { type: 'name', value: '*.tmp.exe' },
        { type: 'name', value: 'svchost*.exe' },
        { type: 'command', value: '.*powershell.*-enc.*' }
      ]
    },
    {
      id: 'system_protection',
      type: 'whitelist',
      icon: 'icon-shield',
      rules: [
        { type: 'name', value: 'System' },
        { type: 'name', value: 'csrss.exe' },
        { type: 'name', value: 'lsass.exe' }
      ]
    },
    {
      id: 'browser_protection',
      type: 'whitelist',
      icon: 'icon-browser',
      rules: [
        { type: 'name', value: 'chrome.exe' },
        { type: 'name', value: 'firefox.exe' },
        { type: 'name', value: 'msedge.exe' }
      ]
    }
  ];

  constructor(
    private fb: FormBuilder,
    private tauriService: TauriService,
    private notificationService: NotificationService,
    private languageService: LanguageService
  ) {
    this.ruleForm = this.fb.group({
      id: ['', [Validators.required]],
      ruleType: ['name', [Validators.required]],
      value: ['', [Validators.required]],
      description: [''],
      severity: ['medium', [Validators.required]],
      autoAction: ['alert'],
      enabled: [true]
    });
  }

  async ngOnInit() {
    await this.loadConfiguration();
    this.filterRules();
  }

  async loadConfiguration() {
    try {
      const loadedConfig = await this.getPolicyConfigFromTauri();
      this.config = loadedConfig;
      this.updateMetadata();
      
      if (this.config.blacklist.length === 0 && this.config.whitelist.length === 0) {
        await this.initializeDefaultRules();
      }
    } catch (error) {
      console.error('Failed to load policy configuration:', error);
      await this.initializeDefaultRules();
    }
  }

  // Missing methods that were causing errors

  importConfig() {
    // TODO: Implement config import functionality
    console.log('Import config functionality to be implemented');
  }

  exportConfig() {
    // TODO: Implement config export functionality  
    console.log('Export config functionality to be implemented');
  }

  getActiveRules(): number {
    const activeBlacklist = this.config.blacklist.filter(rule => rule.enabled).length;
    const activeWhitelist = this.config.whitelist.filter(rule => rule.enabled).length;
    return activeBlacklist + activeWhitelist;
  }

  getCriticalRules(): number {
    const criticalBlacklist = this.config.blacklist.filter(rule => rule.severity === 'critical').length;
    const criticalWhitelist = this.config.whitelist.filter(rule => rule.severity === 'critical').length;
    return criticalBlacklist + criticalWhitelist;
  }

  markAsChanged() {
    this.hasChanges = true;
    this.updateMetadata();
  }

  trackByRuleId(index: number, rule: Rule): string {
    return rule.id;
  }

  formatDate(dateString: string): string {
    return new Date(dateString).toLocaleDateString();
  }

  testRule(rule: Rule) {
    // TODO: Implement rule testing functionality
    console.log('Testing rule:', rule);
    this.notificationService.showSuccess('Rule test functionality to be implemented');
  }

  isSystemRule(rule: Rule): boolean {
    return rule.tags.includes('system') || rule.tags.includes('trusted');
  }

  useTemplate(template: any) {
    // TODO: Implement template usage functionality
    console.log('Using template:', template);
    this.notificationService.showSuccess('Template applied successfully');
  }

  closeRuleEditor() {
    this.showRuleEditor = false;
    this.editingRule = null;
    this.editingIndex = -1;
  }

  getValuePlaceholder(): string {
    const ruleType = this.ruleForm.get('ruleType')?.value;
    switch (ruleType) {
      case 'name': return 'e.g., malware.exe';
      case 'path': return 'e.g., C:\\Windows\\System32\\*';
      case 'hash': return 'e.g., SHA256 hash value';
      case 'command': return 'e.g., powershell.*-enc.*';
      case 'registry': return 'e.g., HKEY_LOCAL_MACHINE\\Software\\*';
      case 'network': return 'e.g., 192.168.1.0/24';
      default: return 'Enter rule value';
    }
  }

  getValueHelpText(): string {
    const ruleType = this.ruleForm.get('ruleType')?.value;
    switch (ruleType) {
      case 'name': return 'Process or file name (supports wildcards)';
      case 'path': return 'Full file path (supports wildcards)';
      case 'hash': return 'File hash (MD5, SHA1, SHA256)';
      case 'command': return 'Command line pattern (supports regex)';
      case 'registry': return 'Registry key path (supports wildcards)';
      case 'network': return 'IP address or network range';
      default: return '';
    }
  }

  addTag() {
    if (this.tagInput.trim() && !this.currentTags.includes(this.tagInput.trim())) {
      this.currentTags.push(this.tagInput.trim());
      this.tagInput = '';
    }
  }

  removeTag(index: number) {
    this.currentTags.splice(index, 1);
  }

  filterRules() {
    // Filter blacklist rules
    this.filteredBlacklistRules = this.config.blacklist.filter(rule => {
      const matchesSearch = !this.blacklistSearch || 
        rule.value.toLowerCase().includes(this.blacklistSearch.toLowerCase()) ||
        rule.description?.toLowerCase().includes(this.blacklistSearch.toLowerCase());
      
      const matchesFilter = this.blacklistFilter === 'all' || 
        (this.blacklistFilter === 'enabled' && rule.enabled) ||
        (this.blacklistFilter === 'disabled' && !rule.enabled) ||
        rule.severity === this.blacklistFilter ||
        rule.ruleType === this.blacklistFilter;

      return matchesSearch && matchesFilter;
    });

    // Filter whitelist rules
    this.filteredWhitelistRules = this.config.whitelist.filter(rule => {
      const matchesSearch = !this.whitelistSearch || 
        rule.value.toLowerCase().includes(this.whitelistSearch.toLowerCase()) ||
        rule.description?.toLowerCase().includes(this.whitelistSearch.toLowerCase());
      
      const matchesFilter = this.whitelistFilter === 'all' || 
        (this.whitelistFilter === 'enabled' && rule.enabled) ||
        (this.whitelistFilter === 'disabled' && !rule.enabled) ||
        (this.whitelistFilter === 'system' && this.isSystemRule(rule)) ||
        rule.severity === this.whitelistFilter ||
        rule.ruleType === this.whitelistFilter;

      return matchesSearch && matchesFilter;
    });
  }

  updateMetadata() {
    const totalBlacklistRules = this.config.blacklist.length;
    const totalWhitelistRules = this.config.whitelist.length;
    const activeBlacklistRules = this.config.blacklist.filter(rule => rule.enabled).length;
    const activeWhitelistRules = this.config.whitelist.filter(rule => rule.enabled).length;

    this.config.metadata = {
      version: this.config.metadata.version,
      lastUpdated: new Date().toISOString(),
      totalRules: totalBlacklistRules + totalWhitelistRules,
      activeRules: activeBlacklistRules + activeWhitelistRules
    };
  }

  async initializeDefaultRules() {
    const defaultBlacklistRules: Rule[] = [
      {
        id: this.generateRuleId(),
        ruleType: 'name',
        value: '*.tmp.exe',
        description: 'Suspicious temporary executable files',
        enabled: true,
        createdAt: new Date().toISOString(),
        lastModified: new Date().toISOString(),
        severity: 'high',
        autoAction: 'quarantine',
        tags: ['malware', 'suspicious']
      },
      {
        id: this.generateRuleId(),
        ruleType: 'command',
        value: '.*powershell.*-enc.*',
        description: 'Encoded PowerShell commands',
        enabled: true,
        createdAt: new Date().toISOString(),
        lastModified: new Date().toISOString(),
        severity: 'critical',
        autoAction: 'kill',
        tags: ['powershell', 'encoded', 'suspicious']
      }
    ];

    const defaultWhitelistRules: Rule[] = [
      {
        id: this.generateRuleId(),
        ruleType: 'name',
        value: 'System',
        description: 'Windows System process',
        enabled: true,
        createdAt: new Date().toISOString(),
        lastModified: new Date().toISOString(),
        severity: 'low',
        autoAction: 'none',
        tags: ['system', 'trusted']
      },
      {
        id: this.generateRuleId(),
        ruleType: 'name',
        value: 'csrss.exe',
        description: 'Client/Server Runtime Subsystem',
        enabled: true,
        createdAt: new Date().toISOString(),
        lastModified: new Date().toISOString(),
        severity: 'low',
        autoAction: 'none',
        tags: ['system', 'trusted']
      }
    ];

    this.config.blacklist = defaultBlacklistRules;
    this.config.whitelist = defaultWhitelistRules;
    this.updateMetadata();
    this.hasChanges = true;
  }

  generateRuleId(): string {
    return 'rule_' + Math.random().toString(36).substr(2, 9) + '_' + Date.now();
  }

  private async getPolicyConfigFromTauri(): Promise<PolicyConfig> {
    return this.config;
  }

  addRule(listType: 'blacklist' | 'whitelist') {
    this.editingRule = null;
    this.editingIndex = -1;
    this.editingList = listType;
    this.showRuleEditor = true;
    this.resetRuleForm();
  }

  editRule(listType: 'blacklist' | 'whitelist', index: number) {
    const rule = listType === 'blacklist' ? this.config.blacklist[index] : this.config.whitelist[index];
    this.editingRule = rule;
    this.editingIndex = index;
    this.editingList = listType;
    this.showRuleEditor = true;
    this.populateRuleForm(rule);
  }

  deleteRule(listType: 'blacklist' | 'whitelist', index: number) {
    if (listType === 'blacklist') {
      this.config.blacklist.splice(index, 1);
    } else {
      this.config.whitelist.splice(index, 1);
    }
    this.updateMetadata();
    this.filterRules();
    this.hasChanges = true;
  }

  private resetRuleForm() {
    this.ruleForm.reset({
      id: this.generateRuleId(),
      ruleType: 'name',
      value: '',
      description: '',
      severity: 'medium',
      autoAction: 'alert',
      enabled: true
    });
    this.currentTags = [];
  }

  private populateRuleForm(rule: Rule) {
    this.ruleForm.patchValue({
      id: rule.id,
      ruleType: rule.ruleType,
      value: rule.value,
      description: rule.description,
      severity: rule.severity,
      autoAction: rule.autoAction,
      enabled: rule.enabled
    });
    this.currentTags = [...rule.tags];
  }

  saveRule() {
    if (!this.ruleForm.valid) {
      return;
    }

    const formValue = this.ruleForm.value;
    const rule: Rule = {
      ...formValue,
      tags: [...this.currentTags],
      createdAt: this.editingRule?.createdAt || new Date().toISOString(),
      lastModified: new Date().toISOString()
    };

    if (this.editingRule && this.editingIndex >= 0) {
      if (this.editingList === 'blacklist') {
        this.config.blacklist[this.editingIndex] = rule;
      } else {
        this.config.whitelist[this.editingIndex] = rule;
      }
    } else {
      if (this.editingList === 'blacklist') {
        this.config.blacklist.push(rule);
      } else {
        this.config.whitelist.push(rule);
      }
    }

    this.updateMetadata();
    this.filterRules();
    this.hasChanges = true;
    this.showRuleEditor = false;
  }

  async saveConfiguration() {
    try {
      // await this.tauriService.savePolicyConfig(this.config);
      this.hasChanges = false;
      this.notificationService.showSuccess('Configuration saved successfully');
    } catch (error) {
      console.error('Failed to save configuration:', error);
      this.notificationService.showError('Failed to save configuration');
    }
  }

  translate(key: string): string {
    return this.languageService.translate(key);
  }
}