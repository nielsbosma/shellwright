use regex::Regex;

/// Startup prompt calibrator.
///
/// Captures the actual prompt displayed by a shell or REPL at startup,
/// then uses that exact text as a literal pattern for future prompt
/// detection. More reliable than generic pattern matching since it
/// anchors to the real prompt.
#[derive(Debug)]
pub struct PromptCalibrator {
    /// The captured startup prompt text.
    calibrated_prompt: Option<String>,
    /// Compiled regex for matching the calibrated prompt.
    prompt_regex: Option<Regex>,
}

impl PromptCalibrator {
    pub fn new() -> Self {
        Self {
            calibrated_prompt: None,
            prompt_regex: None,
        }
    }

    /// Calibrate from terminal output lines.
    ///
    /// Call this after the initial process spawn has settled.
    /// Checks the last 3 non-empty lines and picks the last one
    /// as the prompt pattern.
    pub fn calibrate(&mut self, lines: &[String]) {
        // Filter to non-empty lines
        let non_empty: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();

        if let Some(last) = non_empty.last() {
            let prompt = last.trim().to_string();
            // Escape the prompt text for use as a literal regex
            let escaped = regex::escape(&prompt);
            if let Ok(re) = Regex::new(&escaped) {
                self.calibrated_prompt = Some(prompt);
                self.prompt_regex = Some(re);
            }
        }
    }

    /// Check if a line matches the calibrated prompt.
    pub fn matches(&self, line: &str) -> bool {
        if let Some(re) = &self.prompt_regex {
            re.is_match(line.trim())
        } else {
            false
        }
    }

    /// Get the calibrated prompt text, if any.
    pub fn prompt_text(&self) -> Option<&str> {
        self.calibrated_prompt.as_deref()
    }

    /// Whether calibration has occurred.
    pub fn is_calibrated(&self) -> bool {
        self.calibrated_prompt.is_some()
    }

    /// Reset calibration (e.g. when the shell changes).
    pub fn reset(&mut self) {
        self.calibrated_prompt = None;
        self.prompt_regex = None;
    }
}

impl Default for PromptCalibrator {
    fn default() -> Self {
        Self::new()
    }
}
