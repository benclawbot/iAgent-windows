from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import Literal

from iagent.response_actions import ResponseActions

ProposalKind = Literal["command", "jcode", "type"]


@dataclass(frozen=True, slots=True)
class ActionProposal:
    proposal_id: str
    kind: ProposalKind
    title: str
    body: str
    payload: str
    press_enter: bool = False


def proposals_from_actions(actions: ResponseActions) -> list[ActionProposal]:
    """Build user-facing approval cards for mutating assistant actions."""
    proposals: list[ActionProposal] = []
    if actions.type_text:
        body = actions.type_text
        if actions.press_enter:
            body = f"{body}\n\nThen press Enter."
        proposals.append(
            _proposal(
                kind="type",
                title="Type Into Active App",
                body=body,
                payload=actions.type_text,
                press_enter=actions.press_enter,
            )
        )
    if actions.cli_command:
        proposals.append(
            _proposal(
                kind="command",
                title="Run Command",
                body=actions.cli_command,
                payload=actions.cli_command,
            )
        )
    if actions.jcode_goal:
        proposals.append(
            _proposal(
                kind="jcode",
                title="Delegate To JCode",
                body=actions.jcode_goal,
                payload=actions.jcode_goal,
            )
        )
    return proposals


def _proposal(
    *,
    kind: ProposalKind,
    title: str,
    body: str,
    payload: str,
    press_enter: bool = False,
) -> ActionProposal:
    return ActionProposal(
        proposal_id=f"{kind}:{_stable_id(kind, payload, press_enter)}",
        kind=kind,
        title=title,
        body=body,
        payload=payload,
        press_enter=press_enter,
    )


def _stable_id(kind: ProposalKind, payload: str, press_enter: bool) -> str:
    key = f"{kind}\0{payload}\0{'1' if press_enter else '0'}"
    return hashlib.sha256(key.encode("utf-8")).hexdigest()[:12]
