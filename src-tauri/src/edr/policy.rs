// policy.rs
use crate::config::{Config, Rule, RuleType};
use crate::scanner::ProcessInfo;
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, trace};

pub struct PolicyEngine {
    blacklist: Vec<Rule>,
    whitelist: Vec<Rule>,
    regex_cache: HashMap<String, Regex>,
}

impl PolicyEngine {
    pub fn new(config: &Config) -> Self {
        PolicyEngine {
            blacklist: config.blacklist.clone(),
            whitelist: config.whitelist.clone(),
            regex_cache: HashMap::new(),
        }
    }

    /// Evaluate a process against policies
    /// Returns Some((rule_id, is_blacklisted)) if matched, None otherwise
    pub fn evaluate(&mut self, process: &ProcessInfo) -> Option<(Rule, bool)> {
        // Check whitelist first (higher priority)
        for rule in self.whitelist.clone() {
            if self.matches_rule(&rule, process) {
                debug!("Process '{}' matched whitelist rule: {}", process.name, rule.id);
                return Some((rule, false));
            }
        }

        // Check blacklist
        for rule in self.blacklist.clone() {
            if self.matches_rule(&rule, process) {
                debug!("Process '{}' matched blacklist rule: {}", process.name, rule.id);
                return Some((rule, true));
            }
        }

        None
    }

    fn matches_rule(&mut self, rule: &Rule, process: &ProcessInfo) -> bool {
        match &rule.rule_type {
            RuleType::Name { value } => {
                let matches = process.name.eq_ignore_ascii_case(value);
                trace!("Name rule '{}' vs '{}': {}", value, process.name, matches);
                matches
            }
            RuleType::Path { value } => {
                if let Some(path) = &process.path {
                    let matches = path.eq_ignore_ascii_case(value);
                    trace!("Path rule '{}' vs '{}': {}", value, path, matches);
                    matches
                } else {
                    false
                }
            }
            RuleType::Hash { sha256 } => {
                if let Some(hash) = &process.hash {
                    let matches = hash.eq_ignore_ascii_case(sha256);
                    trace!("Hash rule '{}' vs '{}': {}", sha256, hash, matches);
                    matches
                } else {
                    false
                }
            }
            RuleType::Command { pattern } => {
                let regex = self.get_or_compile_regex(pattern);
                let command_line = process.command.join(" ");
                let matches = regex.is_match(&command_line);
                trace!("Command pattern '{}' vs '{}': {}", pattern, command_line, matches);
                matches
            }
        }
    }

    fn get_or_compile_regex(&mut self, pattern: &str) -> &Regex {
        self.regex_cache.entry(pattern.to_string())
            .or_insert_with(|| {
                Regex::new(pattern).unwrap_or_else(|_| {
                    debug!("Invalid regex '{}', fallback to match-all", pattern);
                    Regex::new(".*").unwrap()
                })
            })
    }
}