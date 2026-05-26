use crate::test_support::setup_test_env;
use iagent::intent_manifest::{IntentActionPlanRequest, IntentManifestStore};
use serde_json::json;

#[test]
fn intent_manifest_imports_actions_for_planning() {
    let _env = setup_test_env().expect("test env");
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("iagent.intent.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "app_id": "local.notes",
            "name": "Local Notes",
            "description": "A test notes app",
            "entrypoint": "notes.exe",
            "actions": [{
                "id": "create_note",
                "title": "Create note",
                "description": "Create a note with a title and body.",
                "parameters": [
                    {"name": "title", "kind": "string", "required": true, "description": "Note title"},
                    {"name": "body", "kind": "string", "required": false, "description": "Note body"}
                ],
                "examples": [{
                    "summary": "Daily note",
                    "input": {"title": "Daily plan", "body": "Ship the feature"}
                }],
                "approval_level": "confirm_before_write",
                "rollback_hint": "Delete the created note from the Local Notes archive."
            }]
        }))
        .unwrap(),
    )
    .expect("write manifest");

    let store = IntentManifestStore::load().expect("load store");
    let imported = store
        .import_manifest(&manifest_path)
        .expect("import manifest");
    assert_eq!(imported.app_id, "local.notes");
    assert_eq!(imported.actions.len(), 1);

    let actions = store.list_actions(Some("note"), 10).expect("list actions");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_id, "create_note");

    let plan = store
        .plan_action(IntentActionPlanRequest {
            app_id: "local.notes".into(),
            action_id: "create_note".into(),
            parameters: json!({"title": "Daily plan"}),
        })
        .expect("plan action");
    assert_eq!(plan.approval_level, "confirm_before_write");
    assert!(plan.rollback_hint.contains("Delete"));
    assert_eq!(plan.required_parameters, vec!["title"]);
    assert_eq!(plan.tool, "intent");
}

#[test]
fn intent_manifest_discovery_finds_nested_iagent_files() {
    let _env = setup_test_env().expect("test env");
    let temp = tempfile::tempdir().expect("tempdir");
    let nested = temp.path().join("app").join("scripts");
    std::fs::create_dir_all(&nested).expect("nested dir");
    let manifest_path = nested.join("iagent.intent.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "app_id": "local.runner",
            "name": "Local Runner",
            "actions": [{
                "id": "run_report",
                "title": "Run report",
                "description": "Run a local reporting script.",
                "parameters": [],
                "examples": [],
                "approval_level": "confirm_before_execute",
                "rollback_hint": "Stop the generated report job."
            }]
        }))
        .unwrap(),
    )
    .expect("write manifest");

    let found = IntentManifestStore::discover(temp.path(), 5).expect("discover manifests");
    assert_eq!(found, vec![manifest_path]);
}

#[test]
fn intent_action_plan_rejects_missing_required_parameters() {
    let _env = setup_test_env().expect("test env");
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = temp.path().join("iagent.intent.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "app_id": "local.deploy",
            "name": "Deploy Helper",
            "actions": [{
                "id": "deploy",
                "title": "Deploy",
                "description": "Deploy a local build.",
                "parameters": [
                    {"name": "environment", "kind": "choice", "required": true, "description": "Target environment"}
                ],
                "examples": [],
                "approval_level": "confirm_before_external_write",
                "rollback_hint": "Run the rollback command printed by the deploy helper."
            }]
        }))
        .unwrap(),
    )
    .expect("write manifest");

    let store = IntentManifestStore::load().expect("load store");
    store
        .import_manifest(&manifest_path)
        .expect("import manifest");

    let err = store
        .plan_action(IntentActionPlanRequest {
            app_id: "local.deploy".into(),
            action_id: "deploy".into(),
            parameters: json!({}),
        })
        .expect_err("missing parameter should fail");

    assert!(
        err.to_string()
            .contains("missing required parameter environment")
    );
}
