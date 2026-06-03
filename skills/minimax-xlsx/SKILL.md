---
name: minimax-xlsx
description: Open, create, read, analyze, edit, and validate Excel/spreadsheet files (.xlsx, .xlsm, .csv, .tsv) with zero format loss. Use when the user asks to create, build, modify, analyze, read, validate, or format any Excel spreadsheet, financial model, pivot table, or tabular data file. Complements the `excel` skill (live COM automation) with headless programmatic generation, formula validation, and bulk row/column operations. Triggers: spreadsheet, Excel, .xlsx, .csv, pivot table, financial model, formula, tabular data, workbook.
implementation-status: implemented
uses-tool: app
tool-actions: [xlsx_create, xlsx_read, xlsx_edit, xlsx_validate, xlsx_recalc, xlsx_format, xlsx_add_column, xlsx_insert_row]
triggers:
  - "create an excel file"
  - "build a spreadsheet"
  - "make a financial model"
  - "add column to spreadsheet"
  - "insert row in excel"
  - "validate excel formulas"
  - "analyze spreadsheet data"
  - "open xlsx"
  - "read excel"
  - "format cells"
  - "create pivot table"
allowed-tools: read,write,edit,bash
platforms: windows,linux,macos
license: MIT
metadata:
  version: "1.0"
  category: productivity
  source: https://github.com/MiniMax-AI/skills/tree/main/skills/minimax-xlsx
---

# MiniMax XLSX Skill

Programmatic Excel generation, editing, and validation. Does NOT require Excel installed.

## When to Use This vs `excel`

| Use this skill | Use the `excel` skill |
|---|---|
| Headless generation, no Office install | Live COM automation on Windows |
| Bulk row/column operations on existing files | Read/write via the running Excel app |
| Formula validation, recalc, style audits | AI-driven analysis of open workbook |
| Cross-platform (Windows/Linux/macOS) | Windows-only |

## Capabilities

- **xlsx_create** — Generate a workbook from scratch. See `references/create.md`.
- **xlsx_read** — Read structure, cell values, formulas, named ranges. `scripts/xlsx_reader.py`.
- **xlsx_edit** — Unpack -> edit XML -> repack. Zero format loss. `scripts/xlsx_unpack.py` + `scripts/xlsx_pack.py`.
- **xlsx_validate** — Check formulas parse and reference valid ranges. `scripts/formula_check.py --report`.
- **xlsx_recalc** — Force recalculation via LibreOffice headless. `scripts/libreoffice_recalc.py`.
- **xlsx_format** — Apply professional financial formatting. `references/format.md`.
- **xlsx_add_column** — Add column with auto-copied formulas, numfmt, styles. `scripts/xlsx_add_column.py`.
- **xlsx_insert_row** — Insert row(s) with data and styles. `scripts/xlsx_insert_row.py`.

## Quick Start

### Read & discover

```bash
python3 scripts/xlsx_reader.py input.xlsx
python3 scripts/formula_check.py file.xlsx --report
```

### Edit (unpack/edit/repack)

```bash
python3 scripts/xlsx_unpack.py in.xlsx /tmp/work/
# edit XML in /tmp/work/xl/worksheets/sheet1.xml
python3 scripts/xlsx_pack.py /tmp/work/ out.xlsx
```

### Add a column with formulas

```bash
python3 scripts/xlsx_unpack.py input.xlsx /tmp/xlsx_work/
python3 scripts/xlsx_add_column.py /tmp/xlsx_work/ --col G \
    --sheet "Sheet1" --header "% of Total" \
    --formula '=F{row}/$F$10' --formula-rows 2:9 \
    --total-row 10 --total-formula '=SUM(G2:G9)' --numfmt '0.0%' \
    --border-row 10 --border-style medium
python3 scripts/xlsx_pack.py /tmp/xlsx_work/ output.xlsx
```

### Validate & audit

```bash
python3 scripts/formula_check.py file.xlsx --json
python3 scripts/style_audit.py file.xlsx
```

## Pitfalls

See `references/ooxml-cheatsheet.md` — shared strings, cell types, formula recalc flags.

## Templates

- `templates/minimal_xlsx/` — minimal valid xlsx as a base for new workbooks

## Dependencies

- `pip install openpyxl` (read/write)
- LibreOffice (for recalc) — optional, only if you need cached values

## License

MIT. Vendored from https://github.com/MiniMax-AI/skills.
