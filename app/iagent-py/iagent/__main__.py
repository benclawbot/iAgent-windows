"""iAgent entry point: `python -m iagent`."""

from __future__ import annotations

from iagent.app import run


def main() -> int:
    return run()


if __name__ == "__main__":
    raise SystemExit(main())
