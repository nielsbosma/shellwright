use shellwright::prompt::detector::{DetectionMethod, PromptDetector};

fn make_detector() -> PromptDetector {
    PromptDetector::new(300, 300)
}

#[test]
fn test_detect_yes_no_prompt() {
    let det = make_detector();
    let lines = vec!["Do you want to continue? [Y/n]".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
    let d = result.unwrap();
    assert!(d.confidence >= 0.9);
    assert_eq!(d.method, DetectionMethod::KnownPattern);
    assert_eq!(d.pattern_name.as_deref(), Some("yes_no"));
}

#[test]
fn test_detect_password_prompt() {
    let det = make_detector();
    let lines = vec!["Password: ".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
    let d = result.unwrap();
    assert!(d.confidence >= 0.9);
    assert_eq!(d.pattern_name.as_deref(), Some("password"));
}

#[test]
fn test_detect_python_repl() {
    let det = make_detector();
    let lines = vec![">>> ".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
    let d = result.unwrap();
    assert!(d.confidence >= 0.8);
    assert_eq!(d.pattern_name.as_deref(), Some("python_repl"));
}

#[test]
fn test_detect_shell_prompt() {
    let det = make_detector();
    let lines = vec!["user@host:~$ ".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
}

#[test]
fn test_no_detection_on_regular_output() {
    let det = make_detector();
    let lines = vec!["Compiling shellwright v0.1.0".to_string()];
    let result = det.detect(&lines, false);
    // Regular build output shouldn't trigger prompt detection
    assert!(result.is_none() || result.unwrap().confidence < 0.5);
}

#[test]
fn test_detect_ssh_confirm() {
    let det = make_detector();
    let lines =
        vec!["Are you sure you want to continue connecting (yes/no/[fingerprint])?".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
    assert!(result.unwrap().confidence >= 0.9);
}

#[test]
fn test_detect_enter_value() {
    let det = make_detector();
    let lines = vec!["Enter a value: ".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
    assert!(result.unwrap().confidence >= 0.8);
}

#[test]
fn test_detect_npm_prompt() {
    let det = make_detector();
    let lines = vec!["? Project name: ".to_string()];
    let result = det.detect(&lines, true);
    assert!(result.is_some());
}

#[test]
fn test_threshold_filtering() {
    let mut det = make_detector();
    det.set_threshold(0.99);
    // Even strong patterns shouldn't pass a 0.99 threshold without settle
    let lines = vec!["user@host:~$ ".to_string()];
    let result = det.detect(&lines, false);
    // Shell prompt has 0.6 base confidence — should be filtered
    assert!(result.is_none());
}

#[test]
fn test_empty_lines() {
    let det = make_detector();
    let lines: Vec<String> = vec![];
    let result = det.detect(&lines, false);
    assert!(result.is_none());
}

#[test]
fn test_confidence_scoring_range() {
    let det = make_detector();
    let lines = vec!["Password: ".to_string()];
    let result = det.detect(&lines, true).unwrap();
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
}
