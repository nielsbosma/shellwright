use shellwright::prompt::calibrator::PromptCalibrator;

#[test]
fn test_calibrate_basic() {
    let mut cal = PromptCalibrator::new();
    assert!(!cal.is_calibrated());

    let lines = vec!["Welcome to bash".to_string(), "user@host:~$ ".to_string()];
    cal.calibrate(&lines);

    assert!(cal.is_calibrated());
    assert_eq!(cal.prompt_text(), Some("user@host:~$"));
}

#[test]
fn test_match_calibrated_prompt() {
    let mut cal = PromptCalibrator::new();
    let lines = vec![">>> ".to_string()];
    cal.calibrate(&lines);

    assert!(cal.matches(">>> "));
    assert!(cal.matches(">>>"));
    assert!(!cal.matches("... "));
}

#[test]
fn test_calibrate_skips_empty_lines() {
    let mut cal = PromptCalibrator::new();
    let lines = vec!["".to_string(), "prompt> ".to_string(), "".to_string()];
    cal.calibrate(&lines);

    assert!(cal.is_calibrated());
    assert!(cal.matches("prompt>"));
}

#[test]
fn test_calibrate_last_non_empty_line() {
    let mut cal = PromptCalibrator::new();
    let lines = vec![
        "first line".to_string(),
        "second line".to_string(),
        "mysql> ".to_string(),
    ];
    cal.calibrate(&lines);

    assert!(cal.matches("mysql>"));
    assert!(!cal.matches("first line"));
}

#[test]
fn test_special_regex_chars_escaped() {
    let mut cal = PromptCalibrator::new();
    // Prompt with special regex chars: $, (, )
    let lines = vec!["(env) user@host:~/project$ ".to_string()];
    cal.calibrate(&lines);

    assert!(cal.matches("(env) user@host:~/project$"));
}

#[test]
fn test_reset() {
    let mut cal = PromptCalibrator::new();
    let lines = vec![">>> ".to_string()];
    cal.calibrate(&lines);
    assert!(cal.is_calibrated());

    cal.reset();
    assert!(!cal.is_calibrated());
    assert!(!cal.matches(">>>"));
}

#[test]
fn test_uncalibrated_never_matches() {
    let cal = PromptCalibrator::new();
    assert!(!cal.matches("anything"));
    assert!(!cal.matches(">>> "));
    assert!(!cal.matches("$ "));
}

#[test]
fn test_empty_lines_no_calibration() {
    let mut cal = PromptCalibrator::new();
    let lines: Vec<String> = vec!["".to_string(), "  ".to_string()];
    cal.calibrate(&lines);
    // No non-empty lines — should not calibrate
    assert!(!cal.is_calibrated());
}
