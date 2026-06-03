---
name: pptx-generator
description: Generate, edit, and read PowerPoint presentations programmatically. Use when the user wants to create a deck from an outline/brief without opening PowerPoint, produce a polished pitch deck, edit XML inside an existing .pptx with zero format loss, or extract text from slides. Complements the `powerpoint` skill (live COM automation) — this skill works headlessly and is best for greenfield creation and bulk generation. Triggers: PPT, PPTX, PowerPoint, presentation, slide, deck, slides, pitch deck.
implementation-status: implemented
uses-tool: app
tool-actions: [pptx_generate, pptx_edit, pptx_extract_text, pptx_render]
triggers:
  - "generate a powerpoint"
  - "create a presentation"
  - "make a slide deck"
  - "build a pitch deck"
  - "edit pptx file"
  - "extract text from presentation"
  - "pptx from outline"
  - "create slides from brief"
allowed-tools: read,write,edit,bash
platforms: windows,linux,macos
license: MIT
metadata:
  version: "1.0"
  category: productivity
  source: https://github.com/MiniMax-AI/skills/tree/main/skills/pptx-generator
---

# PPTX Generator & Editor

Headless PowerPoint generation. Does NOT require PowerPoint installed.

## When to Use This vs `powerpoint`

| Use this skill | Use the `powerpoint` skill |
|---|---|
| Greenfield deck from outline/brief | Edits to a deck the user already has open |
| Bulk generation (many decks) | Adding a comment or single slide live |
| Headless / no Office install | COM automation on Windows with Office |
| XML edit with format preservation | Real-time interactive revisions |

## Capabilities

- **pptx_generate** — Create deck from scratch (cover, TOC, content, section divider, summary). Use PptxGenJS via Node, or the bundled `scripts/generate.js` template.
- **pptx_edit** — Unpack the .pptx, edit XML directly, repack with zero format loss. See `references/editing.md`.
- **pptx_extract_text** — `python -m markitdown <file.pptx>` for fast text extraction.
- **pptx_render** — Convert PPTX to PDF/images via LibreOffice headless.

## Quick Start

### Generate from outline

```bash
npm install -g pptxgenjs markitdown
node scripts/generate.js --outline "Intro;Problem;Solution;Market;Ask" --output deck.pptx
```

### Edit existing PPTX

```bash
python3 scripts/edit.py unpack input.pptx /tmp/work/
# edit XML in /tmp/work/ppt/slides/slide*.xml
python3 scripts/edit.py pack /tmp/work/ output.pptx
```

### Extract text

```bash
python -m markitdown presentation.pptx
```

## Slide Types

Five built-in templates (`references/slide-types.md`): `cover`, `toc`, `content`, `section`, `summary`.

## Design System

Color, typography, spacing via `references/design-system.md`. Override per-deck with a `theme.json` in the working directory.

## Pitfalls

See `references/pitfalls.md` — font embedding, image sizing, chart compatibility.

## Dependencies

- `npm install -g pptxgenjs` (generation)
- `pip install "markitdown[pptx]"` (extraction)
- `pip install python-pptx` (editing)

## License

MIT. Vendored from https://github.com/MiniMax-AI/skills.
