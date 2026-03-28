//! End-to-end test: Shellwright driving DotnetSpectreTest.
//!
//! This is the definition of done — the test passes only when an agent
//! can successfully navigate the Spectre.Console interactive prompts
//! through Shellwright.
//!
//! The DotnetSpectreTest app is a 14-step Server Deployment Wizard
//! that exercises every problematic interaction pattern for AI agents:
//! - Confirmation prompts (Y/n)
//! - Arrow-key navigation (SelectionPrompt)
//! - Spacebar toggles (MultiSelectionPrompt)
//! - Text input with validation
//! - Secret/hidden input
//! - Custom confirm characters (p/a instead of y/n)
//! - Progress bars and live displays

use std::time::Duration;

use shellwright::config::Config;
use shellwright::session::manager::SessionManager;

fn test_config() -> Config {
    let mut config = Config::default();
    config.data_dir = std::env::temp_dir().join("shellwright-e2e");
    config.settle_time_ms = 500;
    config.double_settle_time_ms = 500;
    config
}

/// Helper: wait for a pattern, panicking with context on failure.
async fn wait_for(mgr: &mut SessionManager, session: &str, pattern: &str, timeout_secs: u64) {
    let result = mgr
        .wait_for(session, pattern, Duration::from_secs(timeout_secs))
        .await
        .expect("wait_for failed");

    if result.is_none() {
        let (text, _) = mgr.read_output(session, None, Some(20)).unwrap();
        panic!(
            "Timed out waiting for pattern '{}'. Last output:\n{}",
            pattern, text
        );
    }
}

/// Helper: send input and wait briefly for processing.
async fn send(mgr: &mut SessionManager, session: &str, input: &str) {
    mgr.send_input(session, input).await.expect("send failed");
    tokio::time::sleep(Duration::from_millis(300)).await;
    mgr.process_all().await;
}

/// Helper: send raw bytes (for arrow keys, etc.)
async fn send_key(mgr: &mut SessionManager, session: &str, key: &[u8]) {
    let session_obj = mgr.get_mut(session).unwrap();
    session_obj
        .send_input(&String::from_utf8_lossy(key))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    mgr.process_all().await;
}

#[tokio::test]
#[ignore] // Requires DotnetSpectreTest to be built: dotnet build test/DotnetSpectreTest
async fn test_spectre_console_wizard() {
    let config = test_config();
    let mut mgr = SessionManager::new(config);

    // Path to the DotnetSpectreTest executable
    let exe_path = if cfg!(windows) {
        r"D:\Repos\_Personal\Shellwright\test\DotnetSpectreTest\bin\Debug\net10.0\DotnetSpectreTest.exe"
    } else {
        "D:/Repos/_Personal/Shellwright/test/DotnetSpectreTest/bin/Debug/net10.0/DotnetSpectreTest"
    };

    let cmd = vec![exe_path.to_string()];
    mgr.start(Some("spectre".to_string()), cmd, Some(30), Some(120))
        .unwrap();

    // Step 1: Confirmation — "Do you want to proceed with deployment? [y/n]"
    wait_for(&mut mgr, "spectre", "(?i)proceed|deployment", 10).await;
    send(&mut mgr, "spectre", "y").await;

    // Step 2: Environment Selection — Arrow key navigation
    wait_for(&mut mgr, "spectre", "(?i)environment|target", 5).await;
    // Navigate to "staging-1" (down arrow to reach it, then Enter)
    // The exact number of presses depends on the list order
    send_key(&mut mgr, "spectre", b"\x1b[B").await; // Down
    send_key(&mut mgr, "spectre", b"\x1b[B").await; // Down
    send_key(&mut mgr, "spectre", b"\x1b[B").await; // Down
    send_key(&mut mgr, "spectre", b"\r").await; // Enter

    // Step 3: Service Selection — Multi-select with spacebar
    wait_for(&mut mgr, "spectre", "(?i)service|deploy", 5).await;
    // Toggle some services and confirm
    send_key(&mut mgr, "spectre", b" ").await; // Space to toggle
    send_key(&mut mgr, "spectre", b"\r").await; // Enter to confirm

    // Step 4: Server Name — Text input with validation
    wait_for(&mut mgr, "spectre", "(?i)server name", 5).await;
    send(&mut mgr, "spectre", "test-server-01").await;

    // Step 5: Port Configuration — Numeric input with default
    wait_for(&mut mgr, "spectre", "(?i)port", 5).await;
    send(&mut mgr, "spectre", "8080").await;

    // Step 6: Replica Count — With choices
    wait_for(&mut mgr, "spectre", "(?i)replica", 5).await;
    send(&mut mgr, "spectre", "3").await;

    // Step 7: Database Password — Secret input
    wait_for(&mut mgr, "spectre", "(?i)password", 5).await;
    send(&mut mgr, "spectre", "S3cure!Pass1").await;

    // Step 8: API Key — Invisible secret
    wait_for(&mut mgr, "spectre", "(?i)api.?key", 5).await;
    send(&mut mgr, "spectre", "test-api-key-12345").await;

    // Step 9: Log Level — Selection with search
    wait_for(&mut mgr, "spectre", "(?i)log.?level", 5).await;
    send_key(&mut mgr, "spectre", b"\x1b[B").await; // Down to select
    send_key(&mut mgr, "spectre", b"\r").await; // Enter

    // Step 10: Feature Flags — Optional multi-select
    wait_for(&mut mgr, "spectre", "(?i)feature", 5).await;
    send_key(&mut mgr, "spectre", b"\r").await; // Enter (skip optional)

    // Step 11: Final Confirmation — Custom p/a characters
    wait_for(&mut mgr, "spectre", "(?i)continue|deployment", 5).await;
    send(&mut mgr, "spectre", "p").await; // 'p' for proceed

    // Steps 12-14: Progress bars and live displays — just wait
    wait_for(&mut mgr, "spectre", "(?i)complete|success|operational", 30).await;

    // Verify final output
    let (text, _) = mgr.read_output("spectre", None, None).unwrap();
    assert!(
        text.contains("Operational") || text.contains("success") || text.contains("completed"),
        "Final output should indicate success. Got:\n{}",
        text
    );

    // Cleanup
    let _ = mgr.terminate("spectre");
}
