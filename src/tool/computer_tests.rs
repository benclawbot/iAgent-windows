use super::*;
use crate::tool::{Tool, ToolExecutionMode};
use serde_json::json;

fn with_temp_home<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let temp = tempfile::TempDir::new().expect("create temp dir");
    crate::env::set_var("JCODE_HOME", temp.path());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    match prev_home {
        Some(value) => crate::env::set_var("JCODE_HOME", value),
        None => crate::env::remove_var("JCODE_HOME"),
    }

    result.unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn test_context(execution_mode: ToolExecutionMode) -> ToolContext {
    ToolContext {
        session_id: "session-test".to_string(),
        message_id: "message-test".to_string(),
        tool_call_id: "tool-call-test".to_string(),
        working_dir: None,
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode,
    }
}

#[test]
fn schema_exposes_constrained_computer_actions() {
    let schema = ComputerTool::new().parameters_schema();
    let actions = schema["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum");

    for action in [
        "screenshot",
        "click",
        "type",
        "hotkey",
        "scroll",
        "wait",
        "active_window",
        "context",
        "open_app",
        "list_apps",
    ] {
        assert!(actions.iter().any(|value| value == action), "{action}");
    }
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["app"]["type"], "string");
}

#[test]
fn input_rejects_arbitrary_action_names() {
    let input = json!({ "action": "python", "code": "print(1)" });
    let parsed = serde_json::from_value::<ComputerInput>(input);
    assert!(parsed.is_err());
}

#[test]
fn parses_hotkey_input() {
    let input = json!({ "action": "hotkey", "keys": ["ctrl", "l"] });
    let parsed: ComputerInput = serde_json::from_value(input).unwrap();
    assert_eq!(parsed.action, ComputerAction::Hotkey);
    assert_eq!(
        parsed.keys.unwrap(),
        vec!["ctrl".to_string(), "l".to_string()]
    );
}

#[test]
fn parses_open_app_input() {
    let input = json!({ "action": "open_app", "app": "Hermes" });
    let parsed: ComputerInput = serde_json::from_value(input).unwrap();
    assert_eq!(parsed.action, ComputerAction::OpenApp);
    assert_eq!(parsed.app.unwrap(), "Hermes");
}

#[test]
fn app_matching_prefers_exact_and_desktop_sources() {
    let candidates = vec![
        AppCandidate {
            name: "Hermes Admin".to_string(),
            path: "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Hermes Admin.lnk"
                .into(),
            source: "Common Start Menu".to_string(),
        },
        AppCandidate {
            name: "Hermes".to_string(),
            path: "C:\\Users\\test\\Desktop\\Hermes.lnk".into(),
            source: "Desktop".to_string(),
        },
    ];

    let matches = find_app_matches("Hermes", &candidates, 5);
    assert_eq!(matches[0].candidate.name, "Hermes");
    assert_eq!(matches[0].candidate.source, "Desktop");
    assert_eq!(score_app_match("Maker Gantt", "GanttMaker"), Some(70));
}

#[test]
fn agent_turn_mutating_action_queues_permission_without_execution() {
    with_temp_home(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let output = ComputerTool::new()
                .execute(
                    json!({ "action": "click", "x": 10, "y": 20 }),
                    test_context(ToolExecutionMode::AgentTurn),
                )
                .await
                .unwrap();

            assert!(output.output.contains("Permission request queued"));
            assert!(output.output.contains("was not executed"));
            assert_eq!(output.metadata.as_ref().unwrap()["executed"], false);

            let requests = crate::safety::SafetySystem::new().pending_requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].action, "computer.click");
            assert!(requests[0].description.contains("(10, 20)"));
        })
    });
}
