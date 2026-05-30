---
name: example
description: Reference skill demonstrating structure, triggers, and script usage.
allowed-tools: read,write,edit,bash
platforms: windows,linux,macos
scripts: bootstrap.ps1
---

# Example Skill

Use this skill when you want a minimal, documented template for authoring new skills.

## What It Demonstrates

- Frontmatter metadata consumed by the skill loader.
- Optional `skill.toml` manifest for runtime validation.
- Optional `scripts/` helpers referenced by name.

## Expected Behavior

1. The runtime discovers this folder at startup.
2. `SKILL.md` frontmatter and `skill.toml` are parsed and validated.
3. If either file is malformed, startup logs a clear warning and skips the skill.
