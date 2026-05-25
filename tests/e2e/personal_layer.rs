use crate::test_support::setup_test_env;
use iagent::personal_layer::{
    ClearPersonalData, ClipboardInput, JobInput, PersonalSettings, PersonalSettingsInput,
    PersonalStore, ReminderInput, RuntimeTickInput, SavedWindowLayoutInput, SnippetInput,
    WindowBounds, WindowPlacement, plan_tile_two_windows,
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
        .create_reminder(ReminderInput {
            title: "Already due".into(),
            note: None,
            due_at: "2026-05-25T07:00:00Z".into(),
            source_app: Some("Editor".into()),
            source_title: Some("notes.md".into()),
        })
        .expect("create due reminder");
    let due = store
        .list_due_reminders("2026-05-25T08:00:00Z")
        .expect("due reminders");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].title, "Already due");

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

    let tiled = plan_tile_two_windows(WindowBounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    })
    .expect("tile two windows");
    assert_eq!(tiled[0].label, "left");
    assert_eq!(tiled[0].bounds.width, 960);
    assert_eq!(tiled[1].label, "right");
    assert_eq!(tiled[1].bounds.x, 960);
}

#[test]
fn personal_store_covers_full_product_runtime_controls() {
    let _env = setup_test_env().expect("test env");
    let store = PersonalStore::load().expect("load personal store");

    let settings = store.settings().expect("default settings");
    assert!(settings.clipboard_history_enabled);
    assert!(settings.reminder_notifications_enabled);
    assert!(settings.background_jobs_enabled);
    assert!(settings.proactive_suggestions_enabled);
    assert!(settings.snippet_expansion_enabled);

    let updated = store
        .update_settings(PersonalSettingsInput {
            clipboard_history_enabled: Some(false),
            max_clipboard_entries: Some(2),
            ..Default::default()
        })
        .expect("update settings");
    assert!(!updated.clipboard_history_enabled);
    assert_eq!(updated.max_clipboard_entries, 2);

    store
        .update_settings(PersonalSettingsInput {
            clipboard_history_enabled: Some(true),
            ..Default::default()
        })
        .expect("reenable clipboard");

    store
        .create_snippet(SnippetInput {
            trigger: "/sig".into(),
            body: "Thomas".into(),
            description: Some("email signature".into()),
            app_scope: vec!["Mail".into()],
        })
        .expect("create snippet");
    let expansion = store
        .expand_typed_snippet("Thanks,\n/sig", Some("Mail"))
        .expect("typed snippet")
        .expect("expansion");
    assert_eq!(expansion.output_text, "Thanks,\nThomas");
    assert_eq!(
        store
            .expand_typed_snippet("Thanks,\n/sig", Some("Code"))
            .expect("typed snippet wrong scope"),
        None
    );

    let clipboard = store
        .record_clipboard(ClipboardInput {
            content: "first copied value".into(),
            source_app: Some("Editor".into()),
        })
        .expect("record clipboard")
        .expect("clipboard");
    assert!(store.pin_clipboard(&clipboard.id, true).expect("pin"));
    assert!(store.delete_clipboard(&clipboard.id).expect("delete"));
    assert!(store.recent_clipboard(10).expect("clipboard").is_empty());

    store
        .create_reminder(ReminderInput {
            title: "Bring this back".into(),
            note: Some("contextual reminder".into()),
            due_at: "2026-05-25T07:00:00Z".into(),
            source_app: Some("Editor".into()),
            source_title: Some("notes.md".into()),
        })
        .expect("create reminder");

    let summary_folder = folder();
    store
        .create_job(JobInput {
            kind: "folder_summary".into(),
            description: "Summarize a folder".into(),
            input_json: serde_json::json!({ "folder": summary_folder.path() }),
        })
        .expect("queue job");

    let tick = store
        .run_runtime_tick(RuntimeTickInput {
            as_of: "2026-05-25T08:00:00Z".into(),
            clipboard_content: Some("second copied value".into()),
            active_app: Some("Editor".into()),
            active_window_title: Some("notes.md".into()),
            run_one_job: true,
        })
        .expect("runtime tick");
    assert_eq!(tick.due_reminders.len(), 1);
    assert!(tick.captured_clipboard.is_some());
    assert_eq!(tick.completed_job.as_ref().unwrap().status, "succeeded");
    assert!(
        tick.suggestions
            .iter()
            .any(|suggestion| suggestion.kind == "remember_context")
    );

    let jobs = store.list_jobs().expect("jobs");
    assert!(jobs[0].log_path.is_some());

    let layout = store
        .save_window_layout(SavedWindowLayoutInput {
            name: "writing".into(),
            placements: vec![
                WindowPlacement {
                    label: "notes".into(),
                    bounds: WindowBounds {
                        x: 0,
                        y: 0,
                        width: 960,
                        height: 1080,
                    },
                },
                WindowPlacement {
                    label: "browser".into(),
                    bounds: WindowBounds {
                        x: 960,
                        y: 0,
                        width: 960,
                        height: 1080,
                    },
                },
            ],
        })
        .expect("save layout");
    assert_eq!(layout.name, "writing");
    assert_eq!(
        store
            .saved_window_layout_plan("writing")
            .expect("layout plan")
            .expect("layout")
            .len(),
        2
    );

    let cleared = store
        .clear_personal_data(ClearPersonalData {
            clipboard: true,
            reminders: true,
            snippets: true,
            jobs: true,
            app_windows: true,
            layouts: true,
        })
        .expect("clear personal data");
    assert!(cleared.clipboard > 0);
    assert!(cleared.reminders > 0);
    assert!(cleared.snippets > 0);
    assert!(cleared.jobs > 0);
    assert!(cleared.layouts > 0);

    let reset_settings = store
        .update_settings(
            PersonalSettings {
                clipboard_history_enabled: true,
                ..PersonalSettings::default()
            }
            .into(),
        )
        .expect("reset settings");
    assert!(reset_settings.clipboard_history_enabled);
}

fn folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().expect("temp folder");
    fs::write(folder.path().join("note.txt"), "hello").expect("write file");
    fs::create_dir(folder.path().join("nested")).expect("write dir");
    folder
}
