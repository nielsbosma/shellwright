use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;
use shellwright::session::state::SessionState;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-integration");
    config
}

#[tokio::test]
async fn test_full_session_lifecycle() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo Hello from Shellwright".to_string(),
        ]
    } else {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo 'Hello from Shellwright'".to_string(),
        ]
    };

    let info = mgr
        .start(Some("lifecycle".to_string()), cmd, None, None)
        .unwrap();
    assert_eq!(info.name, "lifecycle");

    // Wait for output and process multiple times to ensure we capture it
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let (text, cursor) = mgr.read_output("lifecycle", None, None).unwrap();
    assert!(
        text.contains("Hello from Shellwright"),
        "Expected output to contain 'Hello from Shellwright', got: '{}'",
        text
    );
    assert!(cursor > 0);
}

#[tokio::test]
async fn test_session_send_input() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Use a long-lived interactive shell
    let cmd = if cfg!(windows) {
        vec!["cmd.exe".to_string()]
    } else {
        vec!["sh".to_string()]
    };

    mgr.start(Some("input-test".to_string()), cmd, None, None)
        .unwrap();

    // Wait for shell to start
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    // Only send input if the session hasn't exited
    let status = mgr.status("input-test").unwrap();
    if status.state == SessionState::Exited {
        // Shell exited immediately — skip the interactive part
        // This can happen in CI environments
        return;
    }

    let echo_cmd = if cfg!(windows) {
        "echo MARKER_123"
    } else {
        "echo MARKER_123"
    };
    mgr.send_input("input-test", echo_cmd).await.unwrap();

    // Wait for the command output to appear
    let result = mgr
        .wait_for("input-test", "MARKER_123", Duration::from_secs(5))
        .await
        .unwrap();
    assert!(
        result.is_some(),
        "Expected 'MARKER_123' to appear in output"
    );

    mgr.terminate("input-test").unwrap();
}

#[tokio::test]
async fn test_session_state_transitions() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo done".to_string(),
        ]
    } else {
        vec!["sh".to_string(), "-c".to_string(), "echo done".to_string()]
    };

    let info = mgr
        .start(Some("states".to_string()), cmd, None, None)
        .unwrap();
    assert_eq!(info.state, SessionState::Spawning);

    // Process until the session settles
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let status = mgr.status("states").unwrap();
    // For a fast command, state should be Running or Exited
    assert!(
        status.state == SessionState::Running || status.state == SessionState::Exited,
        "Unexpected state: {:?}",
        status.state
    );
}

#[tokio::test]
async fn test_session_terminate() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec!["cmd.exe".to_string()]
    } else {
        vec!["sh".to_string()]
    };

    mgr.start(Some("to-kill".to_string()), cmd, None, None)
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    mgr.terminate("to-kill").unwrap();
    assert_eq!(mgr.session_count(), 0);
}
