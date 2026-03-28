use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;
use shellwright::session::state::SessionState;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-concurrent-test");
    config.max_sessions = 10;
    config
}

#[tokio::test]
async fn test_multiple_sessions_isolation() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Start multiple sessions with different output
    for i in 0..3 {
        let marker = format!("UNIQUE_MARKER_{}", i);
        let cmd = if cfg!(windows) {
            vec![
                "cmd.exe".to_string(),
                "/c".to_string(),
                format!("echo {}", marker),
            ]
        } else {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo '{}'", marker),
            ]
        };

        mgr.start(Some(format!("session-{}", i)), cmd, None, None)
            .unwrap();
    }

    // Process output multiple times to ensure all sessions are captured
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    // Verify each session has its own output
    for i in 0..3 {
        let (text, _) = mgr
            .read_output(&format!("session-{}", i), None, None)
            .unwrap();
        let marker = format!("UNIQUE_MARKER_{}", i);
        assert!(
            text.contains(&marker),
            "Session {} should contain '{}', got: '{}'",
            i,
            marker,
            text
        );
    }
}

#[tokio::test]
async fn test_session_list_accuracy() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo x".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "x".to_string()]
    };

    for i in 0..3 {
        mgr.start(Some(format!("list-{}", i)), cmd.clone(), None, None)
            .unwrap();
    }

    let list = mgr.list();
    assert_eq!(list.len(), 3);

    mgr.terminate("list-1").unwrap();

    let list = mgr.list();
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|s| s.name != "list-1"));
}

#[tokio::test]
async fn test_independent_lifecycle() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Session A: short-lived
    let cmd_a = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo fast".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "fast".to_string()]
    };

    // Session B: long-lived shell
    let cmd_b = if cfg!(windows) {
        vec!["cmd.exe".to_string()]
    } else {
        vec!["sh".to_string()]
    };

    mgr.start(Some("short".to_string()), cmd_a, None, None)
        .unwrap();
    mgr.start(Some("long".to_string()), cmd_b, None, None)
        .unwrap();

    // Wait for short to exit
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    // Both sessions should still be in the manager
    assert_eq!(mgr.session_count(), 2);

    // Short should have exited
    let short_status = mgr.status("short").unwrap();
    assert_eq!(short_status.state, SessionState::Exited);

    // Long might or might not have exited (cmd.exe behavior in PTY varies)
    // Just verify we can query it
    let _long_status = mgr.status("long").unwrap();

    // Clean up
    let _ = mgr.terminate("long");
}
