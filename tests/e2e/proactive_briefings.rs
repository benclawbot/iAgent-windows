use crate::test_support::setup_test_env;
use iagent::proactive_briefings::{
    BriefingCalendarItem, BriefingProjectInput, EndTaskRecapRequest, MeetingPrepRequest,
    MorningBriefingRequest, NeverSuggestRequest, ProactiveBriefingStore, RecommendationRequest,
};

#[test]
fn morning_briefing_builds_meeting_and_project_next_actions() {
    let _env = setup_test_env().expect("test env");
    let store = ProactiveBriefingStore::load().expect("load proactive store");

    let briefing = store
        .morning_briefing(MorningBriefingRequest {
            as_of: "2026-05-25T08:00:00Z".into(),
            focus: Some("iAgent roadmap".into()),
            calendar: vec![BriefingCalendarItem {
                title: "Roadmap review".into(),
                starts_at: "2026-05-25T10:00:00Z".into(),
                participants: vec!["Thomas".into(), "Codex".into()],
                source_ref: Some("calendar:roadmap-review".into()),
            }],
            due_reminders: vec!["Send connector-pack update".into()],
            projects: vec![BriefingProjectInput {
                name: "iAgent Windows".into(),
                recent_activity: vec!["Delivered connector permission packs".into()],
                blockers: vec!["Need low-noise proactive recommendations".into()],
                next_actions: vec!["Implement briefing store".into()],
            }],
        })
        .expect("morning briefing");

    assert_eq!(briefing.kind, "morning_briefing");
    assert!(
        briefing
            .sections
            .iter()
            .any(|section| section.title == "Meetings")
    );
    assert!(
        briefing
            .actions
            .iter()
            .any(|action| action.kind == "meeting_prep" && action.title.contains("Roadmap"))
    );
    assert!(
        briefing
            .actions
            .iter()
            .any(|action| action.kind == "project_resume")
    );
}

#[test]
fn task_recaps_are_saved_with_evidence_and_followups() {
    let _env = setup_test_env().expect("test env");
    let store = ProactiveBriefingStore::load().expect("load proactive store");

    let recap = store
        .end_task_recap(EndTaskRecapRequest {
            task_title: "Ship connector packs".into(),
            completed_steps: vec![
                "Added connector catalog".into(),
                "Recorded write evidence".into(),
            ],
            evidence_refs: vec!["commit:5311cc8".into(), "test:e2e connector_packs".into()],
            next_actions: vec!["Start proactive briefings".into()],
            open_questions: vec![],
        })
        .expect("task recap");

    assert_eq!(recap.kind, "end_task_recap");
    assert_eq!(recap.evidence_refs.len(), 2);
    assert!(
        recap
            .actions
            .iter()
            .any(|action| action.kind == "follow_up" && action.title.contains("proactive"))
    );

    let saved = store.list_recaps(10).expect("list recaps");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id, recap.id);
}

#[test]
fn never_suggest_feedback_suppresses_matching_recommendations() {
    let _env = setup_test_env().expect("test env");
    let store = ProactiveBriefingStore::load().expect("load proactive store");

    let first = store
        .recommend(RecommendationRequest {
            active_app: Some("Teams".into()),
            window_title: Some("Roadmap review with Thomas".into()),
            activity: Some("Preparing for a meeting".into()),
            signals: vec!["meeting".into(), "roadmap".into()],
            limit: 5,
        })
        .expect("recommend before feedback");
    assert!(first.iter().any(|action| action.kind == "meeting_prep"));

    store
        .never_suggest(NeverSuggestRequest {
            kind: Some("meeting_prep".into()),
            pattern: Some("roadmap".into()),
            reason: Some("Handled in another workflow".into()),
        })
        .expect("never suggest");

    let second = store
        .recommend(RecommendationRequest {
            active_app: Some("Teams".into()),
            window_title: Some("Roadmap review with Thomas".into()),
            activity: Some("Preparing for a meeting".into()),
            signals: vec!["meeting".into(), "roadmap".into()],
            limit: 5,
        })
        .expect("recommend after feedback");
    assert!(!second.iter().any(|action| action.kind == "meeting_prep"));
}

#[test]
fn meeting_prep_card_links_agenda_sources_and_actions() {
    let _env = setup_test_env().expect("test env");
    let store = ProactiveBriefingStore::load().expect("load proactive store");

    let card = store
        .meeting_prep(MeetingPrepRequest {
            title: "Launch review".into(),
            starts_at: Some("2026-05-25T15:00:00Z".into()),
            participants: vec!["Thomas".into(), "Ops".into()],
            agenda_hints: vec![
                "Review release notes".into(),
                "Confirm owner for rollout".into(),
            ],
            source_refs: vec!["meeting:last-launch-review".into()],
        })
        .expect("meeting prep");

    assert_eq!(card.kind, "meeting_prep");
    assert!(card.summary.contains("Launch review"));
    assert!(
        card.source_refs
            .contains(&"meeting:last-launch-review".into())
    );
    assert!(
        card.actions
            .iter()
            .any(|action| action.kind == "agenda_review")
    );
}
