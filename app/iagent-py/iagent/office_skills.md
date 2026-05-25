# iAgent Office Skills

These deterministic builders are used instead of generic `jcode run` flows when the user asks to create Office files.

## `powerpoint_builder`
- Script: `iagent/create_powerpoint_from_goal.ps1`
- Trigger keywords: `powerpoint`, `ppt`, `pptx`, `slideshow`, `presentation`, `slides`
- Behavior:
  - plans a multi-slide deck from the goal
  - respects requested slide count when provided
  - applies a modern style when requested
  - saves `.pptx` then opens the finished file
  - emits `created_presentation=<path>`

## `word_builder`
- Script: `iagent/create_word_from_goal.ps1`
- Trigger keywords: `microsoft word`, `word doc`, `word document`, `docx`, `.doc`
- Behavior:
  - creates a structured document with title and sections
  - saves `.docx` then opens the finished file
  - emits `created_document=<path>`

## `excel_builder`
- Script: `iagent/create_excel_from_goal.ps1`
- Trigger keywords: `excel`, `xlsx`, `spreadsheet`, `workbook`, `worksheet`
- Behavior:
  - creates a data sheet with formulas
  - creates a summary sheet with a chart
  - saves `.xlsx` then opens the finished file
  - emits `created_workbook=<path>`
