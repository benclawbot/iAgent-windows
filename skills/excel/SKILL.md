---
name: excel
description: Open, read, write, and analyze Excel spreadsheets (.xlsx) via COM automation on Windows.
implementation-status: implemented
uses-tool: app
tool-actions: [oc_excel_open, oc_excel_read, oc_excel_write, oc_excel_formula, oc_excel_summarize, oc_excel_sheets]
triggers:
  - "open spreadsheet"
  - "read excel range"
  - "write to spreadsheet"
  - "analyze spreadsheet"
  - "add formula to excel"
  - "create chart in excel"
  - "summarize spreadsheet data"
---

# Excel Spreadsheet Integration

## Overview

Control Microsoft Excel via COM automation on Windows. Read and write cell ranges, evaluate formulas, generate AI summaries, and create charts. Available when the `excel` toolset is enabled and running on Windows.

## Actions

### excel_open
Open a workbook and get its ID.

**Input fields:**
- `path` (string, required): Full path to the .xlsx file

**Returns:** Workbook ID for subsequent operations.

### excel_read
Read a cell range and return values as JSON.

**Input fields:**
- `workbook_id` (string, required): Workbook ID
- `sheet` (string, required): Sheet name (e.g., "Sheet1")
- `range` (string, required): Range like "A1:B10"

**Returns:** JSON with sheet, range, and values array.

### excel_write
Write values to a cell range.

**Input fields:**
- `workbook_id` (string, required): Workbook ID
- `sheet` (string, required): Sheet name
- `range` (string, required): Range to write to
- `values` (string, required): JSON array of rows

**Returns:** Write confirmation.

### excel_formula
Evaluate or insert a formula.

**Input fields:**
- `workbook_id` (string, required): Workbook ID
- `sheet` (string, required): Sheet name
- `formula` (string, required): Formula text (e.g., "=SUM(A1:A10)")

**Returns:** Formula result.

### excel_summarize
Generate AI summary of spreadsheet data.

**Input fields:**
- `workbook_id` (string, required): Workbook ID
- `sheet` (string, required): Sheet name
- `range` (string, required): Data range to analyze

**Returns:** Natural language summary with key metrics.

### excel_sheets
Get list of sheet names in a workbook.

**Input fields:**
- `workbook_id` (string, required): Workbook ID

**Returns:** Array of sheet names.

## Prerequisites

- Microsoft Excel must be installed on Windows.
- Requires COM automation support.

## Example Usage

```
"open the sales report and summarize the data"
→ excel_open(path="C:\\Reports\\sales.xlsx")
→ excel_summarize(workbook_id="...", sheet="Sheet1", range="A1:D50")

"add a totals column to column E"
→ excel_write(workbook_id="...", sheet="Sheet1", range="E1:E10", values="[[\"Total\"],[\"100\"],[\"200\"]]")

"what are the Q3 totals in the budget spreadsheet?"
→ excel_open(path="C:\\Budget\\budget.xlsx")
→ excel_read(workbook_id="...", sheet="Q3", range="A1:G50")
```

## Notes

- Values are JSON arrays: `[["A1","B1"],["A2","B2"]]` for a 2x2 range.
- Sheet names are case-sensitive.
- Formulas return computed values, not the formula text.
- Summaries extract key metrics, trends, and anomalies from data ranges.