from iagent.response_actions import parse_response_actions


def test_parse_response_actions_extracts_cmd_and_type_tags() -> None:
    actions = parse_response_actions(
        "running now. [POINT:none][TYPE:hello world][ENTER][CMD:git status]"
    )

    assert actions.spoken_text == "running now."
    assert actions.point_tag is None
    assert actions.type_text == "hello world"
    assert actions.press_enter is True
    assert actions.cli_command == "git status"
    assert actions.jcode_goal is None


def test_parse_response_actions_handles_empty_cmd_as_none() -> None:
    actions = parse_response_actions("nothing to run [CMD:   ]")

    assert actions.spoken_text == "nothing to run"
    assert actions.cli_command is None
    assert actions.jcode_goal is None


def test_parse_response_actions_extracts_iagent_goal() -> None:
    actions = parse_response_actions(
        "delegating this now. [POINT:none][IAGENT:build a site with tests and a cron workflow]"
    )

    assert actions.spoken_text == "delegating this now."
    assert actions.iagent_goal == "build a site with tests and a cron workflow"
