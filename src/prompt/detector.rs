use crate::prompt::calibrator::PromptCalibrator;
use crate::prompt::patterns;
use crate::prompt::settle::SettleDetector;

/// Result of prompt detection with confidence scoring.
#[derive(Debug, Clone)]
pub struct PromptDetection {
    /// Overall confidence that the process is waiting for input (0.0 - 1.0).
    pub confidence: f64,
    /// The detected prompt text.
    pub prompt_text: Option<String>,
    /// Which detection method produced the result.
    pub method: DetectionMethod,
    /// Name of the matched known pattern, if any.
    pub pattern_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectionMethod {
    /// Matched a known prompt pattern.
    KnownPattern,
    /// Matched the calibrated startup prompt.
    CalibratedPrompt,
    /// Output has settled (double-settle heuristic).
    OutputSettled,
    /// Multiple signals combined.
    Combined,
}

/// Main prompt detector combining all heuristics.
///
/// Detection pipeline:
/// 1. Check known prompt patterns (highest confidence)
/// 2. Check calibrated prompt match
/// 3. Check output settle state
/// 4. Combine signals with cursor position heuristic
pub struct PromptDetector {
    calibrator: PromptCalibrator,
    settle: SettleDetector,
    /// Minimum confidence to report a detection.
    threshold: f64,
}

impl PromptDetector {
    pub fn new(settle_time_ms: u64, confirm_time_ms: u64) -> Self {
        Self {
            calibrator: PromptCalibrator::new(),
            settle: SettleDetector::new(settle_time_ms, confirm_time_ms),
            threshold: 0.5,
        }
    }

    /// Set the minimum confidence threshold for detection.
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold;
    }

    /// Get a reference to the settle detector (for signaling activity).
    pub fn settle(&self) -> &SettleDetector {
        &self.settle
    }

    /// Get a mutable reference to the calibrator.
    pub fn calibrator_mut(&mut self) -> &mut PromptCalibrator {
        &mut self.calibrator
    }

    pub fn calibrator(&self) -> &PromptCalibrator {
        &self.calibrator
    }

    /// Signal that new output was received.
    pub fn on_output(&self) {
        self.settle.on_activity();
    }

    /// Detect whether the process appears to be waiting for input.
    ///
    /// `last_lines` should be the last few lines of terminal output.
    /// `cursor_at_end` is true if the VT parser shows cursor at end of line.
    pub fn detect(&self, last_lines: &[String], cursor_at_end: bool) -> Option<PromptDetection> {
        let mut best_confidence = 0.0_f64;
        let mut best_method = DetectionMethod::OutputSettled;
        let mut prompt_text = None;
        let mut pattern_name = None;

        // Check the last non-empty line
        let last_line = last_lines
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .map(|s| s.as_str())
            .unwrap_or("");

        // 1. Check known patterns
        if let Some((name, confidence)) = patterns::match_known_patterns(last_line) {
            if confidence > best_confidence {
                best_confidence = confidence;
                best_method = DetectionMethod::KnownPattern;
                prompt_text = Some(last_line.to_string());
                pattern_name = Some(name.to_string());
            }
        }

        // 2. Check calibrated prompt
        if self.calibrator.matches(last_line) {
            let cal_confidence = 0.8;
            if cal_confidence > best_confidence {
                best_confidence = cal_confidence;
                best_method = DetectionMethod::CalibratedPrompt;
                prompt_text = Some(last_line.to_string());
                pattern_name = None;
            }
        }

        // 3. Check settle state — adds confidence boost
        if self.settle.is_confirmed() {
            let settle_boost = 0.3;
            best_confidence = (best_confidence + settle_boost).min(1.0);

            if best_confidence < self.threshold && settle_boost >= self.threshold {
                // Settle alone meets threshold
                best_confidence = settle_boost;
                best_method = DetectionMethod::OutputSettled;
                prompt_text = Some(last_line.to_string());
            } else if best_confidence >= self.threshold {
                best_method = DetectionMethod::Combined;
            }
        }

        // 4. Cursor position heuristic — small boost if cursor at end of line
        if cursor_at_end && !last_line.is_empty() {
            best_confidence = (best_confidence + 0.1).min(1.0);
        }

        if best_confidence >= self.threshold {
            Some(PromptDetection {
                confidence: best_confidence,
                prompt_text,
                method: best_method,
                pattern_name,
            })
        } else {
            None
        }
    }

    /// Reset the detector state (e.g., after sending input).
    pub fn reset(&mut self) {
        self.settle.reset();
    }
}

impl std::fmt::Debug for PromptDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptDetector")
            .field("calibrator", &self.calibrator)
            .field("threshold", &self.threshold)
            .finish()
    }
}
