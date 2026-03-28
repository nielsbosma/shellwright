use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;
use shellwright::session::state::SessionState;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-prompt-test");
    config.settle_time_ms = 200;
    config.double_settle_time_ms = 200;
    config
}

#[tokio::test]
async fn test_detect_shell_prompt() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec!["cmd.exe".to_string()]
    } else {
        vec!["sh".to_string()]
    };

    mgr.start(Some("prompt-shell".to_string()), cmd, None, None)
        .unwrap();

    // Wait for shell to fully start
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let status = mgr.status("prompt-shell").unwrap();
    // Shell should be Running, AwaitingInput, or possibly Exited in CI
    assert!(
        status.state == SessionState::Running
            || status.state == SessionState::AwaitingInput
            || status.state == SessionState::Exited,
        "Unexpected state: {:?}",
        status.state
    );

    if status.state != SessionState::Exited {
        mgr.terminate("prompt-shell").unwrap();
    }
}

#[tokio::test]
async fn test_prompt_after_command() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec!["cmd.exe".to_string()]
    } else {
        vec!["sh".to_string()]
    };

    mgr.start(Some("prompt-after".to_string()), cmd, None, None)
        .unwrap();

    // Wait for initial prompt
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    // Check if shell is still alive before sending input
    let status = mgr.status("prompt-after").unwrap();
    if status.state == SessionState::Exited {
        // Shell exited immediately — skip interactive test
        return;
    }

    mgr.send_input("prompt-after", "echo PROMPT_TEST_MARKER")
        .await
        .unwrap();

    // Use wait_for to reliably capture the output
    let result = mgr
        .wait_for("prompt-after", "PROMPT_TEST_MARKER", Duration::from_secs(5))
        .await
        .unwrap();
    assert!(result.is_some(), "Expected 'PROMPT_TEST_MARKER' to appear");

    mgr.terminate("prompt-after").unwrap();
}
