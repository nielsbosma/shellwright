use once_cell::sync::Lazy;
use regex::Regex;

/// A known prompt pattern with its expected confidence contribution.
#[derive(Debug)]
pub struct PromptPattern {
    pub name: &'static str,
    pub regex: Regex,
    pub confidence: f64,
}

/// Library of known CLI prompt patterns.
pub static KNOWN_PATTERNS: Lazy<Vec<PromptPattern>> = Lazy::new(|| {
    vec![
        // Yes/No confirmation prompts
        PromptPattern {
            name: "yes_no",
            regex: Regex::new(r"\[Y/n\]|\[y/N\]|\[yes/no\]|\[Yes/No\]").unwrap(),
            confidence: 0.95,
        },
        // Generic confirmation
        PromptPattern {
            name: "confirm",
            regex: Regex::new(r"(?i)(continue|proceed|confirm)\??\s*$").unwrap(),
            confidence: 0.85,
        },
        // Password / secret prompts
        PromptPattern {
            name: "password",
            regex: Regex::new(r"(?i)(password|passphrase|secret|token)\s*:\s*$").unwrap(),
            confidence: 0.95,
        },
        // Enter a value
        PromptPattern {
            name: "enter_value",
            regex: Regex::new(r"(?i)enter\s+(a\s+)?value\s*:\s*$").unwrap(),
            confidence: 0.9,
        },
        // Shell prompts
        PromptPattern {
            name: "shell_dollar",
            regex: Regex::new(r"^.*[$#%>]\s*$").unwrap(),
            confidence: 0.6,
        },
        // Python REPL
        PromptPattern {
            name: "python_repl",
            regex: Regex::new(r"^>>>\s*$|^\.\.\.\s*$").unwrap(),
            confidence: 0.9,
        },
        // Node REPL
        PromptPattern {
            name: "node_repl",
            regex: Regex::new(r"^>\s*$|^\.\.\.\s*$").unwrap(),
            confidence: 0.5,
        },
        // SQL prompts (psql, mysql, etc.)
        PromptPattern {
            name: "sql_prompt",
            regex: Regex::new(r"^[\w-]+=?[#>]\s*$|^mysql>\s*$|^sqlite>\s*$").unwrap(),
            confidence: 0.85,
        },
        // npm init / yarn init / pnpm create prompts
        PromptPattern {
            name: "npm_init",
            regex: Regex::new(r"^\?\s+.+[:\?]\s*").unwrap(),
            confidence: 0.85,
        },
        // Terraform apply
        PromptPattern {
            name: "terraform",
            regex: Regex::new(r"(?i)enter\s+a\s+value\s*:").unwrap(),
            confidence: 0.9,
        },
        // SSH host key confirmation
        PromptPattern {
            name: "ssh_confirm",
            regex: Regex::new(r"(?i)are you sure you want to continue connecting").unwrap(),
            confidence: 0.95,
        },
        // Docker login
        PromptPattern {
            name: "docker_login",
            regex: Regex::new(r"(?i)username:|login:").unwrap(),
            confidence: 0.85,
        },
        // Generic colon-terminated prompts
        PromptPattern {
            name: "colon_prompt",
            regex: Regex::new(r"^[A-Z][\w\s]+:\s*$").unwrap(),
            confidence: 0.5,
        },
        // Spectre.Console / interactive selection indicators
        PromptPattern {
            name: "selection_prompt",
            regex: Regex::new(r"[>❯]\s+\S").unwrap(),
            confidence: 0.7,
        },
        // Bracket-terminated prompts (e.g., "[default: 8080]:")
        PromptPattern {
            name: "default_value",
            regex: Regex::new(r"\[default:?\s*[^\]]+\]\s*:?\s*$").unwrap(),
            confidence: 0.85,
        },
    ]
});

/// Check a line against all known patterns.
/// Returns the best match with its confidence score.
pub fn match_known_patterns(line: &str) -> Option<(&'static str, f64)> {
    let mut best: Option<(&str, f64)> = None;

    for pattern in KNOWN_PATTERNS.iter() {
        if pattern.regex.is_match(line) {
            match best {
                None => best = Some((pattern.name, pattern.confidence)),
                Some((_, c)) if pattern.confidence > c => {
                    best = Some((pattern.name, pattern.confidence));
                }
                _ => {}
            }
        }
    }

    best
}
