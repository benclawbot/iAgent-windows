use super::*;
use serde_json::json;

#[test]
fn schema_exposes_word_actions() {
    let schema = WordTool::new().parameters_schema();
    let actions = schema["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum");

    for expected in ["create_document", "extract_text", "review_document"] {
        assert!(
            actions.iter().any(|value| value == expected),
            "missing action {expected}"
        );
    }
    assert_eq!(schema["properties"]["suggestions"]["type"], "array");
}

#[test]
fn parses_review_document_input() {
    let input = json!({
        "action": "review_document",
        "path": "C:\\Users\\test\\draft.docx",
        "suggestions": [{
            "target": "old wording",
            "comment": "This can be clearer.",
            "replacement": "clearer wording"
        }]
    });

    let parsed: WordInput = serde_json::from_value(input).expect("parse input");
    assert_eq!(parsed.action, WordAction::ReviewDocument);
    assert_eq!(parsed.suggestions.len(), 1);
    assert_eq!(
        parsed.suggestions[0].replacement.as_deref(),
        Some("clearer wording")
    );
}

#[test]
fn rejects_empty_create_document() {
    let input = WordInput {
        action: WordAction::CreateDocument,
        title: None,
        content: None,
        path: None,
        visible: None,
        save: None,
        suggestions: Vec::new(),
    };

    let err = validate_input(&input).expect_err("should reject empty draft");
    assert!(err.to_string().contains("title or content"));
}

#[test]
fn rejects_review_without_suggestions() {
    let input = WordInput {
        action: WordAction::ReviewDocument,
        title: None,
        content: None,
        path: None,
        visible: None,
        save: None,
        suggestions: Vec::new(),
    };

    let err = validate_input(&input).expect_err("should reject missing suggestions");
    assert!(err.to_string().contains("suggestions"));
}
