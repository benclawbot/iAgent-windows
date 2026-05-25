use iagent::recipe_catalog::{
    RecipeCatalog, RecipeDispatchRequest, RecipeInputValue, RecipeSearch,
};

#[test]
fn recipe_catalog_searches_hotkey_ready_workflows() {
    let catalog = RecipeCatalog::built_in();

    let results = catalog.search(RecipeSearch {
        query: Some("weekly report".into()),
        tags: vec!["reporting".into()],
        limit: 10,
    });

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "weekly_report");
    assert_eq!(results[0].title, "Create weekly report");
    assert_eq!(results[0].hotkey.as_deref(), Some("Ctrl+Alt+W"));
    assert_eq!(results[0].approval_policy, "confirm_before_write");
    assert!(
        results[0]
            .required_tools
            .iter()
            .any(|tool| tool == "flight_recorder")
    );
}

#[test]
fn recipe_catalog_builds_typed_dispatch_plan_without_executing() {
    let catalog = RecipeCatalog::built_in();

    let plan = catalog
        .dispatch_plan(RecipeDispatchRequest {
            recipe_id: "office_document".into(),
            inputs: vec![
                RecipeInputValue {
                    name: "goal".into(),
                    value: "Draft a client recap document".into(),
                },
                RecipeInputValue {
                    name: "document_type".into(),
                    value: "word".into(),
                },
            ],
        })
        .expect("dispatch plan");

    assert_eq!(plan.recipe_id, "office_document");
    assert_eq!(plan.approval_policy, "confirm_before_write");
    assert_eq!(plan.steps.len(), 4);
    assert_eq!(plan.steps[0].tool, "personal");
    assert_eq!(plan.steps[0].action, "preview_redaction");
    assert_eq!(plan.steps[1].tool, "word");
    assert_eq!(plan.steps[2].tool, "open");
    assert!(plan.summary.contains("Draft a client recap document"));
}

#[test]
fn recipe_catalog_validates_required_inputs() {
    let catalog = RecipeCatalog::built_in();

    let err = catalog
        .dispatch_plan(RecipeDispatchRequest {
            recipe_id: "web_form_fill".into(),
            inputs: vec![RecipeInputValue {
                name: "url".into(),
                value: "https://example.test/form".into(),
            }],
        })
        .expect_err("missing required input should fail");

    assert!(
        err.to_string()
            .contains("missing required input data_source")
    );
}
