use crate::test_support::setup_test_env;
use iagent::connector_packs::{
    ConnectorEvidenceInput, ConnectorGrantRequest, ConnectorPackStore, ConnectorWritePreflight,
};

#[test]
fn connector_catalog_covers_core_ambient_services() {
    let catalog = ConnectorPackStore::built_in_catalog();
    let ids: Vec<&str> = catalog
        .iter()
        .map(|connector| connector.id.as_str())
        .collect();

    for id in [
        "outlook_mail",
        "outlook_calendar",
        "gmail",
        "google_calendar",
        "slack",
        "teams",
        "github",
        "linear",
        "jira",
        "notion",
        "obsidian",
        "file_share",
    ] {
        assert!(ids.contains(&id), "missing connector {id}");
    }

    let github = catalog
        .iter()
        .find(|connector| connector.id == "github")
        .expect("github connector");
    assert!(github.scopes.iter().any(|scope| scope.id == "issues.write"));
    assert!(
        github
            .write_operations
            .iter()
            .any(|operation| operation.id == "create_issue")
    );
}

#[test]
fn connector_write_preflight_requires_explicit_write_scope() {
    let _env = setup_test_env().expect("test env");
    let store = ConnectorPackStore::load().expect("load connector store");

    let denied = store
        .preflight_write(ConnectorWritePreflight {
            connector_id: "github".into(),
            operation: "create_issue".into(),
            target: "benclawbot/iAgent-windows".into(),
            run_id: Some("run-42".into()),
        })
        .expect("preflight without grant");
    assert!(!denied.allowed);
    assert_eq!(denied.required_scopes, vec!["issues.write"]);
    assert_eq!(denied.missing_scopes, vec!["issues.write"]);

    let grant = store
        .grant_scopes(ConnectorGrantRequest {
            connector_id: "github".into(),
            scopes: vec!["issues.write".into()],
            actor: "Thomas".into(),
            reason: "Let iAgent draft GitHub issue follow-ups".into(),
            expires_at: None,
        })
        .expect("grant write scope");
    assert_eq!(grant.connector_id, "github");
    assert_eq!(grant.scopes, vec!["issues.write"]);

    let allowed = store
        .preflight_write(ConnectorWritePreflight {
            connector_id: "github".into(),
            operation: "create_issue".into(),
            target: "benclawbot/iAgent-windows".into(),
            run_id: Some("run-42".into()),
        })
        .expect("preflight with grant");
    assert!(allowed.allowed);
    assert_eq!(allowed.grant_ids, vec![grant.id]);
}

#[test]
fn connector_write_evidence_is_recorded_for_allowed_writes() {
    let _env = setup_test_env().expect("test env");
    let store = ConnectorPackStore::load().expect("load connector store");
    store
        .grant_scopes(ConnectorGrantRequest {
            connector_id: "slack".into(),
            scopes: vec!["messages.write".into()],
            actor: "Thomas".into(),
            reason: "Post approved agent updates".into(),
            expires_at: None,
        })
        .expect("grant slack writes");

    let evidence = store
        .record_write_evidence(ConnectorEvidenceInput {
            connector_id: "slack".into(),
            operation: "post_message".into(),
            target: "#launch".into(),
            run_id: "run-100".into(),
            tool_call_id: Some("tool-9".into()),
            summary: "Posted approved launch status".into(),
            evidence_refs: vec!["flight_recorder:entry-1".into(), "screenshot:after".into()],
        })
        .expect("record evidence");

    assert_eq!(evidence.connector_id, "slack");
    assert_eq!(evidence.required_scopes, vec!["messages.write"]);
    assert_eq!(evidence.run_id, "run-100");
    assert_eq!(evidence.evidence_refs.len(), 2);

    let audit = store
        .audit_writes(Some("slack"), None, 10)
        .expect("audit writes");
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].id, evidence.id);
    assert_eq!(audit[0].target, "#launch");
}
