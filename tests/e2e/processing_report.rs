use crate::test_support::setup_test_env;
use iagent::processing_report::{
    ProcessingDeletionRequest, ProcessingRecordInput, ProcessingReportQuery, ProcessingReportStore,
};

#[test]
fn processing_report_records_local_cloud_and_connector_activity() {
    let _env = setup_test_env().expect("test env");
    let store = ProcessingReportStore::load().expect("load processing report");

    let local = store
        .record(ProcessingRecordInput {
            purpose: "Summarize local meeting transcript".into(),
            processor: "iAgent local runtime".into(),
            environment: "local_device".into(),
            data_categories: vec!["meeting_transcript".into(), "speaker_labels".into()],
            source_refs: vec!["meeting:launch-review".into()],
            retention: "local_until_deleted".into(),
            user_visible: true,
        })
        .expect("record local processing");
    let cloud = store
        .record(ProcessingRecordInput {
            purpose: "Generate model response".into(),
            processor: "OpenAI".into(),
            environment: "external_model".into(),
            data_categories: vec!["prompt".into()],
            source_refs: vec!["session:abc".into()],
            retention: "provider_policy".into(),
            user_visible: true,
        })
        .expect("record cloud processing");

    assert_eq!(local.environment, "local_device");
    assert_eq!(cloud.environment, "external_model");

    let report = store
        .report(ProcessingReportQuery {
            environment: None,
            processor: None,
            data_category: None,
            include_deleted: false,
            limit: 20,
        })
        .expect("report");
    assert_eq!(report.total_records, 2);
    assert_eq!(report.by_environment["local_device"], 1);
    assert_eq!(report.by_environment["external_model"], 1);
    assert_eq!(report.by_data_category["prompt"], 1);
}

#[test]
fn processing_report_filters_and_exports_user_readable_summary() {
    let _env = setup_test_env().expect("test env");
    let store = ProcessingReportStore::load().expect("load processing report");
    store
        .record(ProcessingRecordInput {
            purpose: "Connector write preflight".into(),
            processor: "GitHub connector".into(),
            environment: "external_connector".into(),
            data_categories: vec!["repo_metadata".into()],
            source_refs: vec!["connector:github".into()],
            retention: "audit_log".into(),
            user_visible: true,
        })
        .expect("record connector");
    store
        .record(ProcessingRecordInput {
            purpose: "Local redaction preview".into(),
            processor: "Sensitive Context Firewall".into(),
            environment: "local_device".into(),
            data_categories: vec!["clipboard_text".into()],
            source_refs: vec!["personal:clipboard".into()],
            retention: "not_stored".into(),
            user_visible: true,
        })
        .expect("record local");

    let filtered = store
        .report(ProcessingReportQuery {
            environment: Some("external_connector".into()),
            processor: None,
            data_category: None,
            include_deleted: false,
            limit: 10,
        })
        .expect("filtered report");
    assert_eq!(filtered.total_records, 1);
    assert_eq!(filtered.records[0].processor, "GitHub connector");

    let export = store
        .export_markdown(ProcessingReportQuery {
            environment: None,
            processor: None,
            data_category: None,
            include_deleted: false,
            limit: 10,
        })
        .expect("export markdown");
    assert!(export.contains("# iAgent Processing Transparency Report"));
    assert!(export.contains("GitHub connector"));
    assert!(export.contains("Sensitive Context Firewall"));
}

#[test]
fn processing_report_marks_records_deleted_without_losing_audit() {
    let _env = setup_test_env().expect("test env");
    let store = ProcessingReportStore::load().expect("load processing report");
    let record = store
        .record(ProcessingRecordInput {
            purpose: "Temporary screen analysis".into(),
            processor: "iAgent local runtime".into(),
            environment: "local_device".into(),
            data_categories: vec!["screen_context".into()],
            source_refs: vec!["snapshot:123".into()],
            retention: "local_until_deleted".into(),
            user_visible: true,
        })
        .expect("record processing");

    let deleted = store
        .mark_deleted(ProcessingDeletionRequest {
            record_id: record.id.clone(),
            reason: "User requested snapshot deletion".into(),
        })
        .expect("mark deleted");

    assert_eq!(
        deleted.deleted_reason.as_deref(),
        Some("User requested snapshot deletion")
    );
    assert!(deleted.deleted_at.is_some());

    let hidden = store
        .report(ProcessingReportQuery {
            environment: None,
            processor: None,
            data_category: None,
            include_deleted: false,
            limit: 10,
        })
        .expect("hidden report");
    assert_eq!(hidden.total_records, 0);

    let audit = store
        .report(ProcessingReportQuery {
            environment: None,
            processor: None,
            data_category: None,
            include_deleted: true,
            limit: 10,
        })
        .expect("audit report");
    assert_eq!(audit.total_records, 1);
    assert_eq!(audit.records[0].id, record.id);
}
