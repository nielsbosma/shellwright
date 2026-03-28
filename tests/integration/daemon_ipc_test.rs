use shellwright::daemon::protocol::*;

// Note: Full IPC tests require starting the daemon server, which binds
// to a socket/pipe. These tests verify the protocol serialization
// and message handling at the type level.

#[test]
fn test_request_serialization() {
    let request = Request {
        id: "test-123".to_string(),
        kind: RequestKind::Start(StartParams {
            name: Some("build".to_string()),
            command: vec!["npm".to_string(), "run".to_string(), "build".to_string()],
            rows: None,
            cols: None,
        }),
    };

    let json = serde_json::to_string(&request).unwrap();
    let back: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "test-123");
}

#[test]
fn test_response_success_serialization() {
    let response = Response::success(
        "req-1".to_string(),
        ResponseData::Ok(OkData {
            session: "build".to_string(),
            message: "Session started".to_string(),
        }),
    );

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(!json.contains("\"error\""));
}

#[test]
fn test_response_error_serialization() {
    let response = Response::error("req-2".to_string(), "Session not found");

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("Session not found"));
}

#[test]
fn test_all_request_kinds_serialize() {
    let kinds = vec![
        RequestKind::Start(StartParams {
            name: Some("s1".to_string()),
            command: vec!["echo".to_string()],
            rows: Some(24),
            cols: Some(80),
        }),
        RequestKind::Read(ReadParams {
            session: "s1".to_string(),
            format: Some("clean".to_string()),
            since: Some(0),
            tail: None,
        }),
        RequestKind::Send(SendParams {
            session: "s1".to_string(),
            input: "y".to_string(),
            wait_for: Some("done".to_string()),
            timeout: Some(30.0),
        }),
        RequestKind::Wait(WaitParams {
            session: "s1".to_string(),
            pattern: "PASS|FAIL".to_string(),
            timeout: 60.0,
        }),
        RequestKind::List,
        RequestKind::Status(StatusParams {
            session: "s1".to_string(),
        }),
        RequestKind::Interrupt(InterruptParams {
            session: "s1".to_string(),
        }),
        RequestKind::Terminate(TerminateParams {
            session: "s1".to_string(),
        }),
        RequestKind::ConfirmDanger(ConfirmDangerParams {
            command: "rm -rf /tmp".to_string(),
            justification: "Cleaning up test data from CI run".to_string(),
        }),
        RequestKind::Shutdown,
    ];

    for kind in kinds {
        let request = Request {
            id: "test".to_string(),
            kind,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.is_empty());
        // Verify it round-trips
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "test");
    }
}

#[test]
fn test_output_data_serialization() {
    let data = OutputData {
        session: "build".to_string(),
        text: "Build output...".to_string(),
        cursor: 42,
        lines: 100,
        output_file: "/tmp/shellwright/build/output.txt".to_string(),
        output_tail: "Build succeeded".to_string(),
    };

    let response = Response::success("r1".to_string(), ResponseData::Output(data));
    let json = serde_json::to_string_pretty(&response).unwrap();
    assert!(json.contains("\"cursor\": 42"));
    assert!(json.contains("Build succeeded"));
}

#[test]
fn test_wait_result_serialization() {
    let result = WaitResult {
        session: "s1".to_string(),
        matched: true,
        pattern: "PASS".to_string(),
        match_text: Some("PASS".to_string()),
        timed_out: false,
    };

    let response = Response::success("r2".to_string(), ResponseData::WaitResult(result));
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"matched\":true"));
    assert!(json.contains("\"timed_out\":false"));
}
