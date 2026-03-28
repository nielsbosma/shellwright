use shellwright::config::Config;
use shellwright::session::manager::SessionManager;

fn test_config() -> Config {
    let mut config = Config::default();
    config.max_sessions = 5;
    config.data_dir = std::env::temp_dir().join("shellwright-test");
    config
}

#[test]
fn test_start_session() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Use a simple command that exists on all platforms
    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo hello".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "hello".to_string()]
    };

    let info = mgr.start(Some("test-1".to_string()), cmd, None, None);
    assert!(info.is_ok());
    let info = info.unwrap();
    assert_eq!(info.name, "test-1");
    assert_eq!(mgr.session_count(), 1);
}

#[test]
fn test_auto_name_generation() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo hi".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "hi".to_string()]
    };

    let info = mgr.start(None, cmd, None, None).unwrap();
    assert!(info.name.starts_with("session-"));
}

#[test]
fn test_duplicate_name_rejected() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo a".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "a".to_string()]
    };

    mgr.start(Some("dup".to_string()), cmd.clone(), None, None)
        .unwrap();

    let result = mgr.start(Some("dup".to_string()), cmd, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_session_cap() {
    let config = test_config(); // max_sessions = 5
    let mut mgr = SessionManager::new(config);

    for i in 0..5 {
        let cmd = if cfg!(windows) {
            vec![
                "cmd.exe".to_string(),
                "/c".to_string(),
                format!("echo {}", i),
            ]
        } else {
            vec!["echo".to_string(), format!("{}", i)]
        };
        mgr.start(Some(format!("s-{}", i)), cmd, None, None)
            .unwrap();
    }

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo overflow".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "overflow".to_string()]
    };

    let result = mgr.start(Some("overflow".to_string()), cmd, None, None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Maximum session limit"));
}

#[test]
fn test_list_sessions() {
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

    mgr.start(Some("a".to_string()), cmd.clone(), None, None)
        .unwrap();
    mgr.start(Some("b".to_string()), cmd, None, None).unwrap();

    let list = mgr.list();
    assert_eq!(list.len(), 2);
}

#[test]
fn test_get_nonexistent() {
    let config = test_config();
    let mgr = SessionManager::new(config);
    assert!(mgr.get("nonexistent").is_err());
}

#[test]
fn test_terminate_session() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    let cmd = if cfg!(windows) {
        vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            "echo bye".to_string(),
        ]
    } else {
        vec!["echo".to_string(), "bye".to_string()]
    };

    mgr.start(Some("term".to_string()), cmd, None, None)
        .unwrap();
    assert_eq!(mgr.session_count(), 1);

    mgr.terminate("term").unwrap();
    assert_eq!(mgr.session_count(), 0);
}

#[test]
fn test_terminate_nonexistent() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);
    assert!(mgr.terminate("ghost").is_err());
}
