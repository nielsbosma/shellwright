use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-xplat-test");
    config
}

#[tokio::test]
async fn test_platform_shell_spawn() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo platform_test".to_string(),
        ]
    } else {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo platform_test".to_string(),
        ]
    };

    let info = mgr
        .start(Some("xplat".to_string()), cmd, None, None)
        .unwrap();
    assert_eq!(info.name, "xplat");
    assert!(info.pid.is_some());

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let (text, _) = mgr.read_output("xplat", None, None).unwrap();
    assert!(
        text.contains("platform_test"),
        "Expected 'platform_test' in output, got: '{}'",
        text
    );
}

#[tokio::test]
async fn test_platform_pty_size() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo sized".to_string(),
        ]
    } else {
        vec!["sh".to_string(), "-c".to_string(), "echo sized".to_string()]
    };

    let info = mgr
        .start(Some("sized".to_string()), cmd, Some(40), Some(120))
        .unwrap();
    assert_eq!(info.name, "sized");

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let (text, _) = mgr.read_output("sized", None, None).unwrap();
    assert!(
        text.contains("sized"),
        "Expected 'sized' in output, got: '{}'",
        text
    );
}

#[cfg(windows)]
#[tokio::test]
async fn test_windows_cmd_specific() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Use a simple echo — env var expansion in cmd.exe /c is reliable
    let cmd = vec![
        "cmd.exe".to_string(),
        "/c".to_string(),
        "echo WIN_MARKER_OK".to_string(),
    ];

    mgr.start(Some("win-cmd".to_string()), cmd, None, None)
        .unwrap();

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let (text, _) = mgr.read_output("win-cmd", None, None).unwrap();
    assert!(
        text.contains("WIN_MARKER_OK"),
        "Expected 'WIN_MARKER_OK' in output, got: '{}'",
        text
    );
}

#[cfg(unix)]
#[tokio::test]
async fn test_unix_bash_specific() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = vec![
        "bash".to_string(),
        "-c".to_string(),
        "echo UNIX_MARKER_OK".to_string(),
    ];

    mgr.start(Some("unix-bash".to_string()), cmd, None, None)
        .unwrap();

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        mgr.process_all().await;
    }

    let (text, _) = mgr.read_output("unix-bash", None, None).unwrap();
    assert!(text.contains("UNIX_MARKER_OK"));
}

#[tokio::test]
async fn test_ipc_path_is_platform_appropriate() {
    let config = Config::default();
    let path = config.ipc_path();

    if cfg!(windows) {
        assert!(
            path.to_string_lossy().contains(r"\\.\pipe\"),
            "Windows IPC should use named pipe: {:?}",
            path
        );
    } else {
        assert!(
            path.to_string_lossy().ends_with(".sock"),
            "Unix IPC should use socket: {:?}",
            path
        );
    }
}
