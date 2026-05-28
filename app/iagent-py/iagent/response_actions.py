from __future__ import annotations

import re
from dataclasses import dataclass

from iagent.point_parser import PointTag, parse_point_tag

_ENTER_RE = re.compile(r"\[ENTER\]\s*$", re.IGNORECASE)
_TYPE_RE = re.compile(r"\[TYPE:([^\]]*)\]\s*$", re.IGNORECASE)
_CMD_RE = re.compile(r"\[CMD:([^\]]+)\]\s*$", re.IGNORECASE)
_IAGENT_RE = re.compile(r"\[IAGENT:([^\]]+)\]\s*$", re.IGNORECASE)
_THINK_BLOCK_RE = re.compile(r"<think>[\s\S]*?</think>", re.IGNORECASE)
_THINK_UNCLOSED_RE = re.compile(r"<think>[\s\S]*$", re.IGNORECASE)


@dataclass(frozen=True, slots=True)
class ResponseActions:
    spoken_text: str
    point_tag: PointTag | None
    type_text: str | None
    press_enter: bool
    cli_command: str | None
    iagent_goal: str | None


def strip_reasoning_text(text: str) -> str:
    """Remove model reasoning markup from assistant-visible text."""
    cleaned = _THINK_BLOCK_RE.sub("", text)
    cleaned = _THINK_UNCLOSED_RE.sub("", cleaned)
    return cleaned.replace("</think>", "")


def parse_response_actions(response: str) -> ResponseActions:
    """Parse assistant action tags from the end of a response string.

    Supported tags:
    - [POINT:x,y:label] or [POINT:none]
    - [TYPE:some text]
    - [ENTER]
    - [CMD:some shell command]
    - [IAGENT:high level goal for iagent run]
    """
    working = strip_reasoning_text(response).strip()
    point_tag: PointTag | None = None
    type_text: str | None = None
    press_enter = False
    cli_command: str | None = None
    iagent_goal: str | None = None

    while True:
        prev = working

        spoken, parsed_point = parse_point_tag(working)
        if spoken != working:
            point_tag = parsed_point
            working = spoken.rstrip()
            continue

        m_enter = _ENTER_RE.search(working)
        if m_enter:
            press_enter = True
            working = working[: m_enter.start()].rstrip()
            continue

        m_type = _TYPE_RE.search(working)
        if m_type:
            type_text = m_type.group(1).replace("\\n", "\n").replace("\\t", "\t")
            working = working[: m_type.start()].rstrip()
            continue

        m_cmd = _CMD_RE.search(working)
        if m_cmd:
            parsed_cmd = m_cmd.group(1).strip()
            cli_command = parsed_cmd or None
            working = working[: m_cmd.start()].rstrip()
            continue

        m_iagent = _IAGENT_RE.search(working)
        if m_iagent:
            parsed_goal = m_iagent.group(1).strip()
            iagent_goal = parsed_goal or None
            working = working[: m_iagent.start()].rstrip()
            continue

        if working == prev:
            break

    return ResponseActions(
        spoken_text=working.strip(),
        point_tag=point_tag,
        type_text=type_text,
        press_enter=press_enter,
        cli_command=cli_command,
        iagent_goal=iagent_goal,
    )
