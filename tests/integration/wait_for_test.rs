use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-wait-test");
    config
}

#[tokio::test]
async fn test_wait_for_pattern_in_output() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo STEP1 && echo STEP2 && echo DONE".to_string(),
        ]
    } else {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo STEP1; echo STEP2; echo DONE".to_string(),
        ]
    };

    mgr.start(Some("wait-test".to_string()), cmd, None, None)
        .unwrap();

    let result = mgr
        .wait_for("wait-test", "DONE", Duration::from_secs(10))
        .await
        .unwrap();

    assert!(
        result.is_some(),
        "Expected to find 'DONE' in output. Buffer: '{}'",
        mgr.read_output("wait-test", None, None)
            .unwrap_or_default()
            .0
    );
    assert!(result.unwrap().contains("DONE"));
}

#[tokio::test]
async fn test_wait_for_timeout() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo hello".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "hello".to_string()]
    };

    mgr.start(Some("wait-timeout".to_string()), cmd, None, None)
        .unwrap();

    let result = mgr
        .wait_for("wait-timeout", "NEVER_APPEARS", Duration::from_millis(500))
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_wait_for_regex_pattern() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo Tests: 42 passed, 0 failed".to_string(),
        ]
    } else {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo 'Tests: 42 passed, 0 failed'".to_string(),
        ]
    };

    mgr.start(Some("wait-regex".to_string()), cmd, None, None)
        .unwrap();

    let result = mgr
        .wait_for("wait-regex", r"\d+ passed", Duration::from_secs(10))
        .await
        .unwrap();

    assert!(
        result.is_some(),
        "Expected regex match. Buffer: '{}'",
        mgr.read_output("wait-regex", None, None)
            .unwrap_or_default()
            .0
    );
}

#[tokio::test]
async fn test_wait_for_nonexistent_session() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let result = mgr
        .wait_for("ghost", "pattern", Duration::from_secs(1))
        .await;

    assert!(result.is_err());
}
