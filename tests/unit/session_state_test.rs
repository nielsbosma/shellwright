use shellwright::session::state::SessionState;

#[test]
fn test_valid_transitions() {
    use SessionState::*;

    // Spawning -> Running
    assert!(Spawning.can_transition_to(Running));

    // Running -> various
    assert!(Running.can_transition_to(AwaitingInput));
    assert!(Running.can_transition_to(AwaitingConfirmation));
    assert!(Running.can_transition_to(Exited));

    // AwaitingInput -> Running or Exited
    assert!(AwaitingInput.can_transition_to(Running));
    assert!(AwaitingInput.can_transition_to(Exited));

    // AwaitingConfirmation -> Running or Exited
    assert!(AwaitingConfirmation.can_transition_to(Running));
    assert!(AwaitingConfirmation.can_transition_to(Exited));
}

#[test]
fn test_invalid_transitions() {
    use SessionState::*;

    // Cannot go back to Spawning
    assert!(!Running.can_transition_to(Spawning));
    assert!(!Exited.can_transition_to(Spawning));

    // Cannot go from Spawning directly to AwaitingInput
    assert!(!Spawning.can_transition_to(AwaitingInput));
    assert!(!Spawning.can_transition_to(Exited));

    // Exited is terminal — cannot transition from it
    assert!(!Exited.can_transition_to(Running));
    assert!(!Exited.can_transition_to(AwaitingInput));
}

#[test]
fn test_display() {
    assert_eq!(SessionState::Spawning.to_string(), "spawning");
    assert_eq!(SessionState::Running.to_string(), "running");
    assert_eq!(SessionState::AwaitingInput.to_string(), "awaiting_input");
    assert_eq!(
        SessionState::AwaitingConfirmation.to_string(),
        "awaiting_confirmation"
    );
    assert_eq!(SessionState::Exited.to_string(), "exited");
}

#[test]
fn test_serialize_deserialize() {
    let state = SessionState::Running;
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(json, "\"running\"");

    let deserialized: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, state);
}

#[test]
fn test_all_states_serialize() {
    use SessionState::*;
    for state in [
        Spawning,
        Running,
        AwaitingInput,
        AwaitingConfirmation,
        Exited,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let back: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }
}

#[test]
fn test_self_transition_invalid() {
    use SessionState::*;
    // Self-transitions are not in the valid list
    assert!(!Spawning.can_transition_to(Spawning));
    assert!(!Exited.can_transition_to(Exited));
}
