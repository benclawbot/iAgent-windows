---
name: minimax-pdf
description: Generate beautiful, design-forward PDF documents from scratch or by reformatting existing content. Use when the user wants a PDF with a strong visual identity — proposals, resumes, polished reports, cover-page-first client deliverables. Complements `nano-pdf` (fast text/typo editor) by providing token-based design system: color, typography, and spacing derived from the document type. Triggers: make a PDF, generate a report, write a proposal, create a resume, beautiful PDF, professional document, polished PDF, client-ready document, reformat this document, convert markdown to PDF.
implementation-status: implemented
uses-tool: app
tool-actions: [pdf_create, pdf_fill, pdf_reformat, pdf_merge, pdf_cover]
triggers:
  - "make a pdf"
  - "generate a report"
  - "write a proposal"
  - "create a resume as pdf"
  - "beautiful pdf"
  - "professional document"
  - "client-ready pdf"
  - "reformat this document"
  - "convert markdown to pdf"
  - "fill in pdf form"
  - "apply design to pdf"
allowed-tools: read,write,edit,bash
platforms: windows,linux,macos
license: MIT
metadata:
  version: "1.0"
  category: productivity
  source: https://github.com/MiniMax-AI/skills/tree/main/skills/minimax-pdf
---

# MiniMax PDF Skill

Design-forward PDF generation. Output is print-ready and looks like a real publication, not a printed webpage.

## When to Use This vs `nano-pdf`

| Use this skill | Use the `nano-pdf` skill |
|---|---|
| Create polished PDF from scratch with design | Edit text/typos in an existing PDF |
| Apply a design system to markdown/text | Surgical text replacements |
| Fill in PDF form fields | Read field names |
| Reformat an existing PDF with new visual style | Minimal layout change |

## Capabilities

- **pdf_create** — Generate from scratch via cover + body renderers. `scripts/cover.py` + `scripts/render_body.py`.
- **pdf_fill** — Inspect and write PDF form fields. `scripts/fill_inspect.py` + `scripts/fill_write.py`.
- **pdf_reformat** — Apply a design system to existing markdown/text. `scripts/reformat_parse.py` + `scripts/render_*.{py,js}`.
- **pdf_merge** — Combine multiple PDFs. `scripts/merge.py`.
- **pdf_cover** — Generate a cover page. `scripts/cover.py` (Python) or `scripts/render_cover.js` (Node).

## Design System

Token-based: `scripts/palette.py` derives a color/type/spacing palette from the document type (proposal, resume, report, etc.) and applies it across every page. See `design/design.md` for the design rationale.

## Quick Start

### Generate a proposal PDF

```bash
bash scripts/make.sh --template proposal --input brief.md --output proposal.pdf
```

### Fill a PDF form

```bash
python3 scripts/fill_inspect.py form.pdf           # show fields
python3 scripts/fill_write.py form.pdf out.pdf field_values.json
```

### Reformat an existing document

```bash
python3 scripts/reformat_parse.py input.md /tmp/content.json
python3 scripts/render_body.py /tmp/content.json --theme corporate --output report.pdf
```

## Dependencies

- `pip install reportlab` (Python rendering)
- `npm install -g pdf-lib` (cover rendering, optional)
- `pip install pypdf` (form field inspection)
- LibreOffice or WeasyPrint (for some templates)

## License

MIT. Vendored from https://github.com/MiniMax-AI/skills.
