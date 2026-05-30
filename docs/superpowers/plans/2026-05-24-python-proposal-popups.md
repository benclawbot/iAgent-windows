# Python Proposal Popups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add system-wide floating proposal popups to the Python iAgent frontend so AI-suggested actions require a user Validate or Refuse decision before execution.

**Architecture:** Keep the proposal decision model separate from PySide widgets so action routing is unit-testable. The CompanionManager emits proposal requests for mutating response actions, the app-level popup controller displays topmost floating cards, and accepted proposals route back through the existing command, iagent, and typing handlers.

**Tech Stack:** Python 3.12, PySide6, pytest, uv, PyInstaller.

---

### Task 1: Proposal Model

**Files:**
- Create: `app/iagent-py/iagent/proposals.py`
- Test: `app/iagent-py/tests/test_proposals.py`

- [ ] Write failing tests for converting `ResponseActions` into proposal records.
- [ ] Implement `ActionProposal` and `proposals_from_actions`.
- [ ] Verify `uv run pytest tests/test_proposals.py -q` passes.

### Task 2: Manager Gating

**Files:**
- Modify: `app/iagent-py/iagent/companion_manager.py`
- Modify: `app/iagent-py/tests/test_submit_text_prompt.py`

- [ ] Write failing tests proving `[CMD:...]` and `[IAGENT:...]` produce proposals instead of immediate execution.
- [ ] Add `proposal_requested` and `proposal_decided` signals.
- [ ] Route accepted proposals through the existing command, iagent, and typing execution paths.
- [ ] Verify focused manager tests pass.

### Task 3: Floating Popup UI

**Files:**
- Create: `app/iagent-py/iagent/ui/proposal_popup.py`
- Modify: `app/iagent-py/iagent/app.py`

- [ ] Implement a topmost, frameless `ProposalPopup` with Validate and Refuse buttons.
- [ ] Implement a `ProposalPopupController` that stacks active popups near the lower-right screen edge.
- [ ] Wire controller decisions to `CompanionManager.accept_proposal` and `CompanionManager.reject_proposal`.

### Task 4: Documentation and Validation

**Files:**
- Modify: `app/iagent-py/README.md`

- [ ] Document proposal popups in the runtime architecture and operating modes.
- [ ] Run Python tests, ruff, and PyInstaller build.
- [ ] Commit, push, and test a clean install from the GitHub branch.
