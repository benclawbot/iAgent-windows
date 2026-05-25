// =============================================================================
// Ambient Mode Integration Tests
// =============================================================================

/// Test safety system: action classification
#[test]
fn test_safety_classification() {
    use iagent::safety::SafetySystem;

    let safety = SafetySystem::new();

    // Tier 1: auto-allowed
    assert!(safety.classify("read") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("glob") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("grep") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("memory") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("todoread") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("todowrite") == iagent::safety::ActionTier::AutoAllowed);

    // Tier 2: requires permission
    assert!(safety.classify("bash") == iagent::safety::ActionTier::RequiresPermission);
    assert!(safety.classify("edit") == iagent::safety::ActionTier::RequiresPermission);
    assert!(safety.classify("write") == iagent::safety::ActionTier::RequiresPermission);
    assert!(
        safety.classify("create_pull_request") == iagent::safety::ActionTier::RequiresPermission
    );
    assert!(safety.classify("send_email") == iagent::safety::ActionTier::RequiresPermission);

    // Case insensitive
    assert!(safety.classify("READ") == iagent::safety::ActionTier::AutoAllowed);
    assert!(safety.classify("Bash") == iagent::safety::ActionTier::RequiresPermission);
}

/// Test safety system: permission request queue + decision flow
#[test]
fn test_safety_permission_flow() {
    use iagent::safety::{PermissionRequest, PermissionResult, SafetySystem, Urgency};

    let safety = SafetySystem::new();

    // Count existing pending requests (may have leftover state from other tests)
    let baseline = safety.pending_requests().len();

    // Queue a permission request
    let req = PermissionRequest {
        id: "test_perm_flow_001".to_string(),
        action: "create_pull_request".to_string(),
        description: "Create PR for auth fixes".to_string(),
        rationale: "Found 3 failing auth tests".to_string(),
        urgency: Urgency::High,
        wait: false,
        created_at: chrono::Utc::now(),
        context: None,
    };

    let result = safety.request_permission(req);
    assert!(matches!(result, PermissionResult::Queued { .. }));

    // Verify our request was added
    let pending = safety.pending_requests();
    assert_eq!(pending.len(), baseline + 1);
    assert!(
        pending
            .iter()
            .any(|p| p.action == "create_pull_request" && p.id == "test_perm_flow_001")
    );

    // Record an approval decision
    let _ = safety.record_decision(
        "test_perm_flow_001",
        true,
        "test",
        Some("looks good".to_string()),
    );

    // Verify our request was removed
    assert_eq!(safety.pending_requests().len(), baseline);
}

/// Test safety system: transcript saving
#[test]
fn test_safety_transcript() {
    use iagent::safety::{AmbientTranscript, SafetySystem, TranscriptStatus};

    let safety = SafetySystem::new();

    let transcript = AmbientTranscript {
        session_id: "test_ambient_001".to_string(),
        started_at: chrono::Utc::now(),
        ended_at: Some(chrono::Utc::now()),
        status: TranscriptStatus::Complete,
        provider: "mock".to_string(),
        model: "mock-model".to_string(),
        actions: vec![],
        pending_permissions: 0,
        summary: Some("Test cycle completed".to_string()),
        compactions: 0,
        memories_modified: 3,
        conversation: None,
    };

    // Should not panic
    let result = safety.save_transcript(&transcript);
    assert!(result.is_ok());
}

/// Test safety system: summary generation
#[test]
fn test_safety_summary_generation() {
    use iagent::safety::{ActionLog, ActionTier, PolicyDisposition, RiskLevel, SafetySystem};

    let safety = SafetySystem::new();

    // Log some actions
    safety.log_action(ActionLog {
        action_type: "memory_consolidation".to_string(),
        description: "Merged 2 duplicate memories".to_string(),
        tier: ActionTier::AutoAllowed,
        risk_level: Some(RiskLevel::ReadOnly),
        disposition: Some(PolicyDisposition::AutoAllow),
        undo_token: None,
        screenshot_before: None,
        screenshot_after: None,
        screenshot_diff: None,
        details: None,
        timestamp: chrono::Utc::now(),
    });

    safety.log_action(ActionLog {
        action_type: "memory_prune".to_string(),
        description: "Pruned 1 stale memory".to_string(),
        tier: ActionTier::AutoAllowed,
        risk_level: Some(RiskLevel::ReadOnly),
        disposition: Some(PolicyDisposition::AutoAllow),
        undo_token: None,
        screenshot_before: None,
        screenshot_after: None,
        screenshot_diff: None,
        details: None,
        timestamp: chrono::Utc::now(),
    });

    let summary = safety.generate_summary();
    assert!(summary.contains("Merged 2 duplicate memories"));
    assert!(summary.contains("Pruned 1 stale memory"));
}

/// Test safety system: action flight recorder read model
#[test]
fn test_action_flight_recorder_filters_actions_and_preserves_evidence() {
    use iagent::safety::{
        ActionLog, ActionTier, FlightRecorderQuery, PolicyDisposition, RiskLevel, SafetySystem,
    };
    use serde_json::json;

    let safety = SafetySystem::new();

    safety.log_action(ActionLog {
        action_type: "write".to_string(),
        description: "Edited README roadmap".to_string(),
        tier: ActionTier::RequiresPermission,
        risk_level: Some(RiskLevel::EditLocal),
        disposition: Some(PolicyDisposition::Confirm),
        undo_token: Some("undo-readme-001".to_string()),
        screenshot_before: Some("before.png".to_string()),
        screenshot_after: Some("after.png".to_string()),
        screenshot_diff: Some("diff.png".to_string()),
        details: Some(json!({
            "file": "README.md",
            "success": true
        })),
        timestamp: chrono::Utc::now(),
    });

    let view = safety.flight_recorder(FlightRecorderQuery {
        action_query: Some("roadmap".to_string()),
        risk_level: Some(RiskLevel::EditLocal),
        limit: Some(10),
        include_context: true,
        ..Default::default()
    });

    assert_eq!(view.entries.len(), 1);
    let entry = &view.entries[0];
    assert_eq!(entry.kind, "action");
    assert_eq!(entry.action_type, "write");
    assert_eq!(entry.summary, "Edited README roadmap");
    assert_eq!(entry.risk_level, Some(RiskLevel::EditLocal));
    assert_eq!(entry.disposition, Some(PolicyDisposition::Confirm));
    assert_eq!(entry.undo_token.as_deref(), Some("undo-readme-001"));
    assert_eq!(entry.screenshot_before.as_deref(), Some("before.png"));
    assert_eq!(entry.screenshot_after.as_deref(), Some("after.png"));
    assert_eq!(entry.screenshot_diff.as_deref(), Some("diff.png"));
    assert!(entry.context.is_some());
    assert!(!entry.needs_follow_up);
    assert_eq!(view.totals.total_entries, 1);
    assert_eq!(view.totals.undo_available, 1);
    assert_eq!(view.totals.screenshots_captured, 1);
}

/// Test safety system: pending approvals are first-class flight recorder entries
#[test]
fn test_action_flight_recorder_includes_pending_permission_followups() {
    use iagent::safety::{FlightRecorderQuery, PermissionRequest, SafetySystem, Urgency};

    let safety = SafetySystem::new();
    let request_id = format!("flight_perm_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap());

    let _ = safety.request_permission(PermissionRequest {
        id: request_id.clone(),
        action: "communicate".to_string(),
        description: "Send the follow-up summary".to_string(),
        rationale: "User asked the agent to close the loop".to_string(),
        urgency: Urgency::High,
        wait: false,
        created_at: chrono::Utc::now(),
        context: None,
    });

    let view = safety.flight_recorder(FlightRecorderQuery {
        action_query: Some("follow-up".to_string()),
        include_context: false,
        ..Default::default()
    });

    let entry = view
        .entries
        .iter()
        .find(|entry| entry.id == request_id)
        .expect("pending permission should appear in the flight recorder");

    assert_eq!(entry.kind, "pending_permission");
    assert_eq!(entry.action_type, "communicate");
    assert_eq!(entry.summary, "Send the follow-up summary");
    assert!(entry.needs_follow_up);
    assert_eq!(view.totals.pending_permissions, 1);

    let _ = safety.record_decision(&request_id, false, "test", Some("cleanup".to_string()));
}
