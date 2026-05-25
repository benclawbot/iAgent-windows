use crate::test_support::setup_test_env;
use iagent::personal_layer::{
    ClipboardInput, JobInput, PersonalStore, ReminderInput, SnippetInput, WindowBounds,
};
use std::fs;

#[test]
fn personal_store_covers_recovery_reminders_snippets_apps_jobs_and_layouts() {
    let _env = setup_test_env().expect("test env");
    let store = PersonalStore::load().expect("load personal store");

    let snippet = store
        .create_snippet(SnippetInput {
            trigger: "/sig".into(),
            body: "Thomas".into(),
            description: Some("email signature".into()),
            app_scope: vec!["Mail".into()],
        })
        .expect("create snippet");
    assert_eq!(store.expand_snippet("/sig").expect("expand"), "Thomas");
    assert_eq!(snippet.trigger, "/sig");

    let reminder = store
        .create_reminder(ReminderInput {
            title: "Follow up".into(),
            note: Some("about this spreadsheet".into()),
            due_at: "2026-05-25T09:00:00Z".into(),
            source_app: Some("Excel".into()),
            source_title: Some("Budget.xlsx".into()),
        })
        .expect("create reminder");
    assert_eq!(reminder.status, "pending");
    assert_eq!(store.list_pending_reminders().expect("pending").len(), 1);

    store
        .record_clipboard(ClipboardInput {
            content: "copied text".into(),
            source_app: Some("Editor".into()),
        })
        .expect("record clipboard");
    store
        .record_clipboard(ClipboardInput {
            content: "copied text".into(),
            source_app: Some("Editor".into()),
        })
        .expect("dedupe clipboard");
    assert_eq!(store.recent_clipboard(10).expect("clipboard").len(), 1);

    store
        .record_app_window("Code.exe", "C:/Tools/Code.exe", "iAgent roadmap")
        .expect("record app");
    let app_match = store
        .resolve_app_description("roadmap")
        .expect("resolve app")
        .expect("app result");
    assert_eq!(app_match.window_title, "iAgent roadmap");

    let summary_folder = folder();
    let job = store
        .create_job(JobInput {
            kind: "folder_summary".into(),
            description: "Summarize a folder".into(),
            input_json: serde_json::json!({"folder": summary_folder.path()}),
        })
        .expect("create job");
    assert_eq!(job.status, "pending");
    let finished = store
        .run_job(&job.id)
        .expect("run job")
        .expect("finished job");
    assert_eq!(finished.status, "succeeded");
    assert_eq!(finished.output_json.as_ref().unwrap()["files"], 1);

    let cancellable = store
        .create_job(JobInput {
            kind: "batch_rename_preview".into(),
            description: "Preview screenshot rename".into(),
            input_json: serde_json::json!({"folder":"C:/Temp"}),
        })
        .expect("create cancellable job");
    store.cancel_job(&job.id).expect("cancel job");
    store.cancel_job(&cancellable.id).expect("cancel job");
    assert_eq!(store.list_jobs().expect("jobs")[0].status, "cancelled");

    let snapped = iagent::personal_layer::plan_snap_window(
        WindowBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        "left",
    )
    .expect("snap left");
    assert_eq!(snapped.width, 960);
    assert_eq!(snapped.height, 1080);
}

fn folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().expect("temp folder");
    fs::write(folder.path().join("note.txt"), "hello").expect("write file");
    fs::create_dir(folder.path().join("nested")).expect("write dir");
    folder
}
