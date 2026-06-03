---
name: minimax-docx
description: Professional DOCX document creation, editing, and formatting. Use when the user wants to create a Word document (resume, report, proposal, academic paper), edit an existing .docx, or apply a specific style (corporate, academic, CJK typography). Three pipelines: create from scratch, fill/edit content in existing documents, format with style templates. Complements the `word` skill (live COM automation) with headless programmatic generation. Triggers: docx, Word document, resume, report, .docx, formatted document, academic paper, official document, memo.
implementation-status: implemented
uses-tool: app
tool-actions: [docx_create, docx_edit, docx_format, docx_apply_style, docx_fill]
triggers:
  - "create a word document"
  - "make a resume"
  - "write a report"
  - "edit docx file"
  - "format a document"
  - "academic paper"
  - "official document"
  - "apply corporate style"
  - "fill docx template"
allowed-tools: read,write,edit,bash
platforms: windows,linux,macos
license: MIT
metadata:
  version: "1.0.0"
  category: document-processing
  source: https://github.com/MiniMax-AI/skills/tree/main/skills/minimax-docx
  references:
    - "ECMA-376 Office Open XML File Formats"
    - "GB/T 9704-2012 Layout Standard for Official Documents"
    - "IEEE / ACM / APA / MLA / Chicago / Turabian Style Guides"
    - "Springer LNCS / Nature / HBR Document Templates"
---

# MiniMax DOCX Skill

Professional Word document creation and editing. Does NOT require Microsoft Word installed.

## When to Use This vs `word`

| Use this skill | Use the `word` skill |
|---|---|
| Generate from scratch (resume, report, academic) | Edit a doc the user has open in Word |
| Apply design templates (corporate, academic, CJK) | Add comments, revisions, feedback |
| Headless / no Office install | COM automation on Windows |
| OOXML-level precision | Live interactive editing |

## Capabilities

- **docx_create** — Generate from scratch. See `references/design_principles.md`.
- **docx_edit** — Open existing .docx, modify text, preserve formatting.
- **docx_format** — Apply style assets (corporate, academic, default). See `assets/styles/`.
- **docx_apply_style** — Re-style an existing document.
- **docx_fill** — Fill a template by replacing placeholders.

## Style Assets

- `assets/styles/default_styles.xml` — Default Word styles
- `assets/styles/corporate_styles.xml` — Corporate/business
- `assets/styles/academic_styles.xml` — Academic (APA, IEEE, Chicago, MLA, Turabian)

## Design References

- `references/design_principles.md` — When to use what style
- `references/design_good_bad_examples.md` — Visual do/don't
- `references/cjk_typography.md` — CJK (Chinese/Japanese/Korean) typography rules
- `references/cjk_university_template_guide.md` — Chinese university templates
- `references/comments_guide.md` — Comment annotation patterns
- `references/openxml_encyclopedia_part{1,2,3}.md` — Full OOXML element reference
- `references/openxml_element_order.md` — Required element order in `document.xml`
- `references/openxml_namespaces.md` — Namespace declarations
- `references/openxml_units.md` — Units (twips, EMU, etc.)

## Dependencies

- `pip install python-docx`
- `pip install lxml` (for OOXML manipulation)
- `pip install openpyxl` (sometimes needed for embedded tables)

## License

MIT. Vendored from https://github.com/MiniMax-AI/skills.
