use crate::test_support::setup_test_env;
use iagent::attention_budget::{
    AttentionBudgetSettingsInput, AttentionEventInput, AttentionPreflightRequest,
    AttentionSnoozeRequest, AttentionStore,
};

#[test]
fn attention_budget_blocks_low_priority_during_quiet_hours() {
    let _env = setup_test_env().expect("test env");
    let store = AttentionStore::load().expect("load attention store");
    store
        .update_settings(AttentionBudgetSettingsInput {
            quiet_hours_start: Some(Some("22:00".into())),
            quiet_hours_end: Some(Some("07:00".into())),
            max_interruptions_per_hour: Some(3),
            max_interruptions_per_day: Some(12),
            ..Default::default()
        })
        .expect("update settings");

    let decision = store
        .preflight(AttentionPreflightRequest {
            kind: "proactive_suggestion".into(),
            title: "Save this as a snippet".into(),
            priority: "low".into(),
            source: "personal_daemon".into(),
            at: "2026-05-25T23:15:00Z".into(),
        })
        .expect("preflight");

    assert!(!decision.allowed);
    assert_eq!(decision.reason, "quiet_hours");
    assert_eq!(decision.delivery, "digest");
}

#[test]
fn attention_budget_allows_critical_approval_during_quiet_hours() {
    let _env = setup_test_env().expect("test env");
    let store = AttentionStore::load().expect("load attention store");
    store
        .update_settings(AttentionBudgetSettingsInput {
            quiet_hours_start: Some(Some("22:00".into())),
            quiet_hours_end: Some(Some("07:00".into())),
            ..Default::default()
        })
        .expect("update settings");

    let decision = store
        .preflight(AttentionPreflightRequest {
            kind: "approval_needed".into(),
            title: "Approve remote dispatch".into(),
            priority: "critical".into(),
            source: "dispatch".into(),
            at: "2026-05-25T23:15:00Z".into(),
        })
        .expect("preflight");

    assert!(decision.allowed);
    assert_eq!(decision.delivery, "immediate");
    assert!(decision.reason.contains("critical"));
}

#[test]
fn attention_budget_enforces_hourly_caps_and_digest() {
    let _env = setup_test_env().expect("test env");
    let store = AttentionStore::load().expect("load attention store");
    store
        .update_settings(AttentionBudgetSettingsInput {
            max_interruptions_per_hour: Some(2),
            max_interruptions_per_day: Some(10),
            ..Default::default()
        })
        .expect("update settings");

    for index in 0..2 {
        store
            .record_event(AttentionEventInput {
                kind: "proactive_suggestion".into(),
                title: format!("Suggestion {}", index + 1),
                priority: "normal".into(),
                source: "briefing".into(),
                delivered: true,
                delivery: "immediate".into(),
                occurred_at: "2026-05-25T10:10:00Z".into(),
            })
            .expect("record event");
    }

    let decision = store
        .preflight(AttentionPreflightRequest {
            kind: "proactive_suggestion".into(),
            title: "Suggestion 3".into(),
            priority: "normal".into(),
            source: "briefing".into(),
            at: "2026-05-25T10:45:00Z".into(),
        })
        .expect("preflight cap");
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "hourly_budget_exhausted");

    let digest = store
        .digest("2026-05-25T00:00:00Z", "2026-05-26T00:00:00Z")
        .expect("digest");
    assert_eq!(digest.delivered_count, 2);
    assert!(digest.items.iter().any(|item| item.title == "Suggestion 1"));
}

#[test]
fn attention_snooze_suppresses_until_expiry() {
    let _env = setup_test_env().expect("test env");
    let store = AttentionStore::load().expect("load attention store");
    let snooze = store
        .snooze(AttentionSnoozeRequest {
            until: "2026-05-25T12:00:00Z".into(),
            reason: Some("Focus block".into()),
        })
        .expect("snooze");
    assert_eq!(snooze.reason.as_deref(), Some("Focus block"));

    let suppressed = store
        .preflight(AttentionPreflightRequest {
            kind: "meeting_prep".into(),
            title: "Prep launch review".into(),
            priority: "normal".into(),
            source: "briefing".into(),
            at: "2026-05-25T11:00:00Z".into(),
        })
        .expect("preflight snoozed");
    assert!(!suppressed.allowed);
    assert_eq!(suppressed.reason, "snoozed");

    let allowed = store
        .preflight(AttentionPreflightRequest {
            kind: "meeting_prep".into(),
            title: "Prep launch review".into(),
            priority: "normal".into(),
            source: "briefing".into(),
            at: "2026-05-25T12:30:00Z".into(),
        })
        .expect("preflight after snooze");
    assert!(allowed.allowed);
}
