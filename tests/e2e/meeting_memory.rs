use crate::test_support::setup_test_env;
use iagent::meeting_memory::{
    MeetingMemoryStore, MeetingSegmentInput, MeetingSessionInput, MeetingTaskConversionRequest,
};

#[test]
fn meeting_memory_finishes_with_source_linked_notes() {
    let _env = setup_test_env().expect("test env");
    let store = MeetingMemoryStore::load().expect("load meeting store");

    let session = store
        .start_session(MeetingSessionInput {
            title: "Roadmap planning".into(),
            participants: vec!["Thomas".into(), "Codex".into()],
        })
        .expect("start meeting");
    store
        .append_segment(MeetingSegmentInput {
            session_id: session.id.clone(),
            speaker: Some("Thomas".into()),
            start_ms: 0,
            end_ms: 3_200,
            text: "Decision: ship the meeting memory mode next.".into(),
        })
        .expect("decision segment");
    store
        .append_segment(MeetingSegmentInput {
            session_id: session.id.clone(),
            speaker: Some("Codex".into()),
            start_ms: 3_300,
            end_ms: 7_000,
            text: "Action: create a follow-up task for transcript citations.".into(),
        })
        .expect("action segment");
    store
        .append_segment(MeetingSegmentInput {
            session_id: session.id.clone(),
            speaker: Some("Thomas".into()),
            start_ms: 7_100,
            end_ms: 9_000,
            text: "Question: should this create reminders automatically?".into(),
        })
        .expect("question segment");

    let finished = store.finish_session(&session.id).expect("finish meeting");
    let summary = finished.summary.expect("summary");

    assert_eq!(finished.status, "complete");
    assert_eq!(finished.segments.len(), 3);
    assert_eq!(
        summary.decisions[0].text,
        "ship the meeting memory mode next."
    );
    assert_eq!(
        summary.action_items[0].text,
        "create a follow-up task for transcript citations."
    );
    assert_eq!(
        summary.questions[0].text,
        "should this create reminders automatically?"
    );
    assert_eq!(
        summary.decisions[0].source_segment_id,
        finished.segments[0].id
    );
    assert!(summary.source_notes.iter().any(|note| {
        note.source_segment_id == finished.segments[1].id
            && note.text.contains("follow-up task")
            && note.quote.contains("Action:")
    }));
}

#[test]
fn meeting_memory_converts_action_items_to_personal_work() {
    let _env = setup_test_env().expect("test env");
    let store = MeetingMemoryStore::load().expect("load meeting store");

    let session = store
        .start_session(MeetingSessionInput {
            title: "Launch review".into(),
            participants: vec!["Thomas".into()],
        })
        .expect("start meeting");
    store
        .append_segment(MeetingSegmentInput {
            session_id: session.id.clone(),
            speaker: Some("Thomas".into()),
            start_ms: 0,
            end_ms: 2_000,
            text: "Action: send release notes to the team.".into(),
        })
        .expect("action segment");
    let finished = store.finish_session(&session.id).expect("finish meeting");
    let action_id = finished.summary.as_ref().unwrap().action_items[0]
        .id
        .clone();

    let conversion = store
        .convert_action_items(MeetingTaskConversionRequest {
            session_id: session.id.clone(),
            action_item_ids: vec![action_id.clone()],
            create_reminders: true,
            create_jobs: true,
            create_delegation_drafts: true,
        })
        .expect("convert action items");

    assert_eq!(conversion.reminders.len(), 1);
    assert_eq!(conversion.jobs.len(), 1);
    assert_eq!(conversion.delegation_drafts.len(), 1);
    assert_eq!(conversion.delegation_drafts[0].tool, "communicate");

    let updated = store
        .get_session(&session.id)
        .expect("get meeting")
        .unwrap();
    let item = &updated.summary.as_ref().unwrap().action_items[0];
    assert!(
        item.converted_to
            .iter()
            .any(|target| target.kind == "reminder")
    );
    assert!(item.converted_to.iter().any(|target| target.kind == "job"));
    assert!(
        item.converted_to
            .iter()
            .any(|target| target.kind == "delegation_draft")
    );
}
