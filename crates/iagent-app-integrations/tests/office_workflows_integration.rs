use iagent_app_integrations::office_workflows::{WorkflowKind, WorkflowRequest, run_workflow};

fn fixture(path: &str) -> String {
    format!(
        "{}/tests/fixtures/office/{}",
        env!("CARGO_MANIFEST_DIR"),
        path
    )
}

#[test]
fn summarize_word_workflow_preview_uses_doc_fixture() {
    let request = WorkflowRequest {
        workflow: WorkflowKind::SummarizeWordDoc,
        input_path: fixture("meeting-notes.docx"),
        output_path: None,
        csv_path: None,
        notes: None,
        dry_run: Some(true),
        max_items: None,
    };

    let result = run_workflow(&request).expect("word summary preview should succeed");
    assert!(result.success);
    assert!(!result.executed);
    assert!(
        result
            .preview
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
}

#[test]
fn extract_action_items_returns_normalized_objects() {
    let request = WorkflowRequest {
        workflow: WorkflowKind::ExtractActionItems,
        input_path: fixture("meeting-notes.txt"),
        output_path: None,
        csv_path: None,
        notes: None,
        dry_run: Some(true),
        max_items: Some(10),
    };

    let result = run_workflow(&request).expect("action extraction should succeed");
    assert!(result.success);

    let items = result
        .preview
        .get("action_items")
        .and_then(serde_json::Value::as_array)
        .expect("action_items should be an array");
    assert!(!items.is_empty());
    let first = &items[0];
    assert!(
        first
            .get("title")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert_eq!(
        first.get("owner").and_then(serde_json::Value::as_str),
        Some("unassigned")
    );
    assert_eq!(
        first.get("due_date").and_then(serde_json::Value::as_str),
        Some("TBD")
    );
    assert_eq!(
        first.get("status").and_then(serde_json::Value::as_str),
        Some("pending")
    );
}

#[test]
fn update_excel_from_csv_preview_reports_plan() {
    let request = WorkflowRequest {
        workflow: WorkflowKind::UpdateExcelFromCsv,
        input_path: fixture("template.xlsx"),
        output_path: None,
        csv_path: Some(fixture("updates.csv")),
        notes: None,
        dry_run: Some(true),
        max_items: None,
    };

    let result = run_workflow(&request).expect("excel preview should succeed");
    assert!(result.success);
    assert!(!result.executed);
    assert_eq!(
        result
            .preview
            .get("planned_cells")
            .and_then(serde_json::Value::as_u64),
        Some(6)
    );
}

#[test]
fn generate_powerpoint_preview_from_fixture_notes() {
    let request = WorkflowRequest {
        workflow: WorkflowKind::GeneratePowerpointFromNotes,
        input_path: fixture("slides-notes.txt"),
        output_path: Some("/tmp/iagent-office-workflow-test.pptx".to_string()),
        csv_path: None,
        notes: None,
        dry_run: Some(true),
        max_items: Some(5),
    };

    let result = run_workflow(&request).expect("ppt preview should succeed");
    assert!(result.success);
    assert!(!result.executed);
    assert!(
        result
            .preview
            .get("bullet_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn update_excel_requires_csv_path() {
    let request = WorkflowRequest {
        workflow: WorkflowKind::UpdateExcelFromCsv,
        input_path: fixture("template.xlsx"),
        output_path: None,
        csv_path: None,
        notes: None,
        dry_run: Some(true),
        max_items: None,
    };

    let err = run_workflow(&request).expect_err("missing csv_path should fail");
    assert!(err.to_string().contains("csv_path is required"));
}
