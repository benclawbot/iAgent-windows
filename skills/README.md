# iAgent Skills

Skills extend iAgent with domain-specific capabilities. Each skill is a folder containing a `SKILL.md` (frontmatter + body) and optionally a `skill.toml` for runtime validation and permissions.

## How skills are discovered

At startup, the runtime scans every folder under `skills/` and parses its `SKILL.md` frontmatter. If the frontmatter or `skill.toml` is malformed, startup logs a warning and skips the skill. Otherwise the skill is registered with its `name`, `description`, `triggers`, and `tool-actions`. Triggers decide when the LLM is shown the skill; `tool-actions` declare which runtime actions the skill uses.

## Available skills

### Built-in (COM automation, requires Office on Windows)

| Skill | Description |
|---|---|
| `browser/` | Navigate, fill forms, click, extract content, take screenshots |
| `excel/` | Read/write/analyze Excel via COM automation |
| `powerpoint/` | Read/edit PowerPoint via COM automation |
| `word/` | Read/revise/comment on Word docs via COM automation |
| `form-fill/` | Autofill web forms using browser automation |

### Vendored from MiniMax-AI/skills (headless, no Office required)

| Skill | Description |
|---|---|
| `pptx-generator/` | Programmatic PowerPoint generation (PptxGenJS, markitdown) |
| `minimax-xlsx/` | Programmatic Excel creation, editing, formula validation |
| `minimax-docx/` | Professional Word document creation with style assets |
| `minimax-pdf/` | Design-forward PDF generation with token-based design system |
| `fullstack-dev/` | Full-stack architecture and integration patterns (knowledge-only) |

### Reference

| Skill | Description |
|---|---|
| `example/` | Minimal template showing skill structure |

## Built-in vs vendored

The built-in skills (excel, powerpoint, word) use live COM automation, so they require Office installed on Windows and operate on a file the user already has open. The vendored MiniMax skills work headlessly — no Office required, fully cross-platform — and are best for greenfield generation and bulk operations.

For example, to edit a deck the user has open in PowerPoint, use `powerpoint/`. To generate a fresh pitch deck from an outline, use `pptx-generator/`.

## Adding a new skill

1. Create `skills/<name>/`
2. Add `SKILL.md` with frontmatter: `name`, `description`, `implementation-status`, `uses-tool`, `tool-actions`, `triggers`. Optionally `allowed-tools` and `platforms`.
3. Optionally add `skill.toml` with `permissions` and `triggers` for runtime validation.
4. Optionally add `scripts/` and reference them from the body.

See `example/SKILL.md` for the minimum viable structure.

## Sources

Vendored skills are MIT-licensed from https://github.com/MiniMax-AI/skills. Each vendored skill retains the original LICENSE where provided.
