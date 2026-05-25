use crate::test_support::setup_test_env;
use iagent::remote_dispatch::{
    DispatchCompletionRequest, DispatchFailureRequest, DispatchSubmitRequest, RemoteDispatchStore,
};
use serde_json::json;

#[test]
fn remote_dispatch_requires_auth_and_surfaces_approval_status() {
    let _env = setup_test_env().expect("test env");
    let store = RemoteDispatchStore::load().expect("load dispatch store");
    let client = store
        .create_client("phone")
        .expect("create dispatch client");

    let rejected = store
        .submit_task(DispatchSubmitRequest {
            client_token: "bad-token".into(),
            title: "Review launch checklist".into(),
            instruction: "Check launch blockers".into(),
            origin: "mobile".into(),
            target: "local".into(),
            scheduled_for: None,
            approval_level: Some("confirm_before_execute".into()),
            context: json!({"source": "test"}),
        })
        .expect_err("bad token should fail");
    assert!(rejected.to_string().contains("invalid dispatch token"));

    let task = store
        .submit_task(DispatchSubmitRequest {
            client_token: client.token.clone(),
            title: "Review launch checklist".into(),
            instruction: "Check launch blockers".into(),
            origin: "mobile".into(),
            target: "local".into(),
            scheduled_for: None,
            approval_level: Some("confirm_before_execute".into()),
            context: json!({"source": "test"}),
        })
        .expect("submit task");

    assert_eq!(task.status, "approval_needed");
    assert!(task.approval_required);

    let status = store
        .status(&task.id)
        .expect("status")
        .expect("task status");
    assert_eq!(status.status, "approval_needed");
    assert!(status.approval_required);
    assert_eq!(status.last_event.as_deref(), Some("approval_needed"));

    let events = store
        .watch_events(Some(&task.id), 10)
        .expect("watch events");
    assert!(events.iter().any(|event| {
        event.kind == "approval_needed" && event.notify_user && event.message.contains("approval")
    }));
}

#[test]
fn remote_dispatch_completes_with_evidence_for_watch_mode() {
    let _env = setup_test_env().expect("test env");
    let store = RemoteDispatchStore::load().expect("load dispatch store");
    let client = store
        .create_client("tablet")
        .expect("create dispatch client");
    let task = store
        .submit_task(DispatchSubmitRequest {
            client_token: client.token,
            title: "Summarize repo".into(),
            instruction: "Create a concise repo status".into(),
            origin: "tablet".into(),
            target: "local".into(),
            scheduled_for: None,
            approval_level: Some("auto_read_only".into()),
            context: json!({}),
        })
        .expect("submit task");

    store.approve_task(&task.id).expect("approve task");
    let completed = store
        .complete_task(DispatchCompletionRequest {
            task_id: task.id.clone(),
            summary: "Repo status sent to device".into(),
            evidence_refs: vec!["flight_recorder:run-1".into(), "commit:abc123".into()],
        })
        .expect("complete task");

    assert_eq!(completed.status, "completed");
    assert_eq!(completed.evidence_refs.len(), 2);

    let status = store.status(&task.id).expect("status").unwrap();
    assert_eq!(status.status, "completed");
    assert_eq!(status.evidence_refs.len(), 2);
    assert!(status.failure_packet.is_none());
}

#[test]
fn remote_dispatch_scheduled_tasks_and_failures_have_packets() {
    let _env = setup_test_env().expect("test env");
    let store = RemoteDispatchStore::load().expect("load dispatch store");
    let client = store.create_client("web").expect("create dispatch client");
    let task = store
        .submit_task(DispatchSubmitRequest {
            client_token: client.token,
            title: "Run nightly check".into(),
            instruction: "Check background jobs".into(),
            origin: "web".into(),
            target: "local".into(),
            scheduled_for: Some("2026-05-25T09:00:00Z".into()),
            approval_level: Some("auto_read_only".into()),
            context: json!({"job": "nightly"}),
        })
        .expect("submit task");

    assert_eq!(task.status, "scheduled");
    let due = store.due_tasks("2026-05-25T09:05:00Z").expect("due tasks");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, task.id);

    let failed = store
        .fail_task(DispatchFailureRequest {
            task_id: task.id.clone(),
            error: "Provider unavailable".into(),
            retry_hint: Some("Retry after provider auth refresh".into()),
            log_refs: vec!["log:dispatch-1".into()],
        })
        .expect("fail task");

    assert_eq!(failed.status, "failed");
    let packet = failed.failure_packet.expect("failure packet");
    assert_eq!(packet.error, "Provider unavailable");
    assert!(packet.retry_hint.unwrap().contains("Retry"));
}
