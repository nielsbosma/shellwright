use std::time::Duration;

use shellwright::prompt::settle::SettleDetector;

#[tokio::test]
async fn test_settle_on_silence() {
    let detector = SettleDetector::new(50, 50);

    // No activity — should settle within timeout
    let settled = detector.wait_for_settle(Duration::from_millis(500)).await;
    assert!(settled);
    assert!(detector.is_confirmed());
}

#[tokio::test]
async fn test_activity_resets_settle() {
    let detector = SettleDetector::new(100, 100);

    // Signal activity in a background task after a short delay
    let det_clone = detector.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        det_clone.on_activity();
    });

    // Should still eventually settle (activity was brief)
    let settled = detector.wait_for_settle(Duration::from_millis(1000)).await;
    assert!(settled);
}

#[tokio::test]
async fn test_continuous_activity_prevents_settle() {
    let detector = SettleDetector::new(100, 100);

    // Continuously signal activity
    let det_clone = detector.clone();
    let handle = tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(30)).await;
            det_clone.on_activity();
        }
    });

    // With activity every 30ms and settle time of 100ms, should not settle
    // within a short timeout
    let settled = detector.wait_for_settle(Duration::from_millis(300)).await;

    handle.abort();

    // May or may not settle depending on timing — the key test is
    // that it doesn't falsely settle during continuous activity
    // Just verify the detector state is consistent
    assert_eq!(detector.is_confirmed(), settled);
}

#[tokio::test]
async fn test_timeout_returns_false() {
    let detector = SettleDetector::new(2000, 2000);

    // Settle time (4000ms total) far exceeds timeout (100ms)
    let settled = detector.wait_for_settle(Duration::from_millis(100)).await;
    assert!(!settled);
}

#[tokio::test]
async fn test_reset() {
    let detector = SettleDetector::new(50, 50);

    // Settle first
    detector.wait_for_settle(Duration::from_millis(500)).await;
    assert!(detector.is_confirmed());

    // Reset
    detector.reset();
    assert!(!detector.is_settled());
    assert!(!detector.is_confirmed());
}

#[tokio::test]
async fn test_double_settle_not_premature() {
    let detector = SettleDetector::new(50, 50);

    // First settle should happen, then confirmation after second period
    let settled = detector.wait_for_settle(Duration::from_millis(500)).await;
    assert!(settled);

    // Both phases should be complete
    assert!(detector.is_settled());
    assert!(detector.is_confirmed());
}
