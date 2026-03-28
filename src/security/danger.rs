use once_cell::sync::Lazy;
use regex::Regex;

/// A dangerous command pattern with its category.
#[derive(Debug)]
pub struct DangerPattern {
    pub name: &'static str,
    pub category: &'static str,
    pub regex: Regex,
}

static DANGER_PATTERNS: Lazy<Vec<DangerPattern>> = Lazy::new(|| {
    vec![
        // Destructive file operations
        DangerPattern {
            name: "rm_recursive",
            category: "filesystem",
            regex: Regex::new(r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*f|(-[a-zA-Z]*f[a-zA-Z]*r))\b").unwrap(),
        },
        DangerPattern {
            name: "rm_force",
            category: "filesystem",
            regex: Regex::new(r"rm\s+-[a-zA-Z]*f").unwrap(),
        },
        DangerPattern {
            name: "mkfs",
            category: "filesystem",
            regex: Regex::new(r"\bmkfs\b").unwrap(),
        },
        DangerPattern {
            name: "dd_device",
            category: "filesystem",
            regex: Regex::new(r"\bdd\b.*\bof=/dev/").unwrap(),
        },
        DangerPattern {
            name: "format_disk",
            category: "filesystem",
            regex: Regex::new(r"\bformat\b.*[A-Z]:").unwrap(),
        },
        // SQL destruction
        DangerPattern {
            name: "drop_table",
            category: "sql",
            regex: Regex::new(r"(?i)\bDROP\s+(TABLE|DATABASE|SCHEMA)\b").unwrap(),
        },
        DangerPattern {
            name: "truncate",
            category: "sql",
            regex: Regex::new(r"(?i)\bTRUNCATE\s+TABLE\b").unwrap(),
        },
        DangerPattern {
            name: "delete_no_where",
            category: "sql",
            regex: Regex::new(r"(?i)\bDELETE\s+FROM\s+\w+\s*(;|$|\s*--)").unwrap(),
        },
        // Network piping
        DangerPattern {
            name: "curl_pipe_sh",
            category: "network",
            regex: Regex::new(r"curl\b.*\|\s*(ba)?sh\b").unwrap(),
        },
        DangerPattern {
            name: "wget_pipe_sh",
            category: "network",
            regex: Regex::new(r"wget\b.*\|\s*(ba)?sh\b").unwrap(),
        },
        // Permission escalation
        DangerPattern {
            name: "chmod_777",
            category: "permissions",
            regex: Regex::new(r"\bchmod\s+777\b").unwrap(),
        },
        DangerPattern {
            name: "chmod_recursive_777",
            category: "permissions",
            regex: Regex::new(r"\bchmod\s+-R\s+777\b").unwrap(),
        },
        // Process killing
        DangerPattern {
            name: "kill_9",
            category: "process",
            regex: Regex::new(r"\bkill\s+-9\b").unwrap(),
        },
        DangerPattern {
            name: "killall",
            category: "process",
            regex: Regex::new(r"\bkillall\b").unwrap(),
        },
        // Fork bomb
        DangerPattern {
            name: "fork_bomb",
            category: "system",
            regex: Regex::new(r":\(\)\{.*\|.*&\s*\};:").unwrap(),
        },
        // System config writes
        DangerPattern {
            name: "etc_write",
            category: "system",
            regex: Regex::new(r">\s*/etc/").unwrap(),
        },
        DangerPattern {
            name: "passwd_write",
            category: "system",
            regex: Regex::new(r">\s*/etc/passwd|>\s*/etc/shadow").unwrap(),
        },
        // Git destructive
        DangerPattern {
            name: "git_force_push",
            category: "git",
            regex: Regex::new(r"\bgit\s+push\s+.*--force\b|\bgit\s+push\s+-f\b").unwrap(),
        },
        DangerPattern {
            name: "git_reset_hard",
            category: "git",
            regex: Regex::new(r"\bgit\s+reset\s+--hard\b").unwrap(),
        },
        DangerPattern {
            name: "git_clean_force",
            category: "git",
            regex: Regex::new(r"\bgit\s+clean\s+-[a-zA-Z]*f").unwrap(),
        },
        // Container destruction
        DangerPattern {
            name: "docker_system_prune",
            category: "container",
            regex: Regex::new(r"\bdocker\s+system\s+prune\b").unwrap(),
        },
        // Kubernetes
        DangerPattern {
            name: "kubectl_delete_all",
            category: "kubernetes",
            regex: Regex::new(r"\bkubectl\s+delete\s+.*--all\b").unwrap(),
        },
    ]
});

/// Result of dangerous command detection.
#[derive(Debug, Clone)]
pub struct DangerDetection {
    pub pattern_name: String,
    pub category: String,
    pub command: String,
}

/// Dangerous command detector.
#[derive(Debug)]
pub struct DangerDetector {
    enabled: bool,
    /// Commands that have been confirmed (session_name:command hash).
    confirmed: std::collections::HashSet<String>,
}

impl DangerDetector {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            confirmed: std::collections::HashSet::new(),
        }
    }

    /// Check if a command matches any dangerous patterns.
    pub fn check(&self, command: &str) -> Option<DangerDetection> {
        if !self.enabled {
            return None;
        }

        for pattern in DANGER_PATTERNS.iter() {
            if pattern.regex.is_match(command) {
                return Some(DangerDetection {
                    pattern_name: pattern.name.to_string(),
                    category: pattern.category.to_string(),
                    command: command.to_string(),
                });
            }
        }

        None
    }

    /// Confirm a dangerous command for execution.
    /// Returns `Err` if the command is not actually dangerous (anti-bypass).
    pub fn confirm(&mut self, command: &str, justification: &str) -> Result<(), String> {
        if justification.len() < 10 {
            return Err("Justification must be at least 10 characters".to_string());
        }

        // Anti-bypass: confirming a non-dangerous command is an error
        if self.check(command).is_none() {
            return Err("Command is not flagged as dangerous — no confirmation needed".to_string());
        }

        let key = format!("{:x}", md5_hash(command));
        self.confirmed.insert(key);
        Ok(())
    }

    /// Check if a command has been confirmed.
    pub fn is_confirmed(&self, command: &str) -> bool {
        let key = format!("{:x}", md5_hash(command));
        self.confirmed.contains(&key)
    }
}

/// Simple hash for command deduplication (not cryptographic).
fn md5_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
