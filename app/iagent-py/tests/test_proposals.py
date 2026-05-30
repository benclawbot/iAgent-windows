from iagent.proposals import ActionProposal, proposals_from_actions
from iagent.response_actions import ResponseActions


def test_proposals_from_actions_builds_command_and_iagent_cards() -> None:
    actions = ResponseActions(
        spoken_text="I can do that.",
        point_tag=None,
        type_text=None,
        press_enter=False,
        cli_command="python -m pytest -q",
        iagent_goal="update the README and run tests",
    )

    proposals = proposals_from_actions(actions)

    assert proposals == [
        ActionProposal(
            proposal_id="command:2d39d4d028d2",
            kind="command",
            title="Run Command",
            body="python -m pytest -q",
            payload="python -m pytest -q",
            press_enter=False,
        ),
        ActionProposal(
            proposal_id="iagent:ba1afe1187f2",
            kind="iagent",
            title="Delegate To iAgent",
            body="update the README and run tests",
            payload="update the README and run tests",
            press_enter=False,
        ),
    ]


def test_proposals_from_actions_includes_typing_draft_with_enter_hint() -> None:
    actions = ResponseActions(
        spoken_text="",
        point_tag=None,
        type_text="hello world",
        press_enter=True,
        cli_command=None,
        iagent_goal=None,
    )

    proposals = proposals_from_actions(actions)

    assert proposals == [
        ActionProposal(
            proposal_id="type:bcb07e9f1383",
            kind="type",
            title="Type Into Active App",
            body="hello world\n\nThen press Enter.",
            payload="hello world",
            press_enter=True,
        )
    ]
