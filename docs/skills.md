# Skills: Authoring and Discovery

Skills are reusable prompt+workflow bundles that guide the runtime toward reliable tool usage patterns.

## Skill Layout

Each skill lives in its own directory:

```text
skills/<skill-name>/
  SKILL.md
  skill.toml          # optional but recommended manifest
  scripts/            # optional helper scripts
```

## `SKILL.md` Requirements

`SKILL.md` must include YAML frontmatter:

- `name`: skill id
- `description`: short summary
- optional: `allowed-tools`, `platforms`, `scripts`

If frontmatter is malformed, the runtime logs a warning and skips loading that skill.

## `skill.toml` Manifest

`skill.toml` is optional but validated when present.

Supported keys:

- `permissions = ["..."]`
- `triggers = ["..."]`

Malformed manifests are treated as load errors and clearly reported in logs.

## Discovery at Startup

The runtime loads skills from:

1. `~/.iagent/skills`
2. `./.iagent/skills` (project-local)
3. `./.claude/skills` (compatibility fallback)

On first run, it can import compatible skills from existing local skill stores (Claude/Codex) into `~/.iagent/skills`.

## Trigger Model

- Explicit invocation: slash-style invocation by skill name.
- Implicit steering: skill content is included in the available skill context for agent selection.
- `triggers` in `skill.toml` provide searchable intent hints and documentation for maintainers.

## Example

See:

- `skills/example/SKILL.md`
- `skills/example/skill.toml`
- `skills/example/scripts/bootstrap.ps1`
