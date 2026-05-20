---
name: word
description: Open, read, revise, and add comments to Word documents (.docx) via COM automation on Windows.
triggers:
  - "open word document"
  - "read word document"
  - "add comment to document"
  - "suggest revision in word"
  - "export word to pdf"
  - "review document"
  - "give feedback on document"
---

# Word Document Integration

## Overview

Control Microsoft Word via COM automation on Windows. Read documents, add comments, suggest tracked revisions, generate AI feedback, and export to PDF. Available when the `word` toolset is enabled and running on Windows.

## Actions

### word_open
Open a Word document and get its ID.

**Input fields:**
- `path` (string, required): Full path to the .docx file

**Returns:** Document ID for subsequent operations.

### word_read
Read the text content of an open document.

**Input fields:**
- `document_id` (string, required): Document ID from word_open

**Returns:** Full text content of the document.

### word_comment
Add a comment at a specific position.

**Input fields:**
- `document_id` (string, required): Document ID
- `position` (string, required): Position hint (e.g., "paragraph 3", "after 'introduction'")
- `text` (string, required): Comment text
- `author` (string, optional, default: "iAgent"): Comment author name

**Returns:** Comment ID.

### word_revision
Suggest a tracked revision (text replacement).

**Input fields:**
- `document_id` (string, required): Document ID
- `position` (string, required): Position of the text to revise
- `original` (string, required): Original text
- `suggested` (string, required): Suggested replacement text

**Returns:** Revision confirmation.

### word_feedback
Generate AI-powered feedback on a document.

**Input fields:**
- `document_id` (string, required): Document ID
- `instructions` (string, required): Focus area for feedback (e.g., "clarity", "tone", "structure")

**Returns:** List of specific improvement suggestions.

### word_export_pdf
Export the document to PDF.

**Input fields:**
- `document_id` (string, required): Document ID
- `output_path` (string, required): Output PDF file path

**Returns:** PDF path confirmation.

## Prerequisites

- Microsoft Word must be installed on Windows.
- Requires COM automation support (available on all modern Windows versions).

## Example Usage

```
"open the contract at C:\Docs\contract.docx and review it"
→ word_open(path="C:\\Docs\\contract.docx")
→ word_feedback(document_id="...", instructions="legal clarity")

"add a comment on the introduction section of the proposal"
→ word_open(path="C:\\Docs\\proposal.docx")
→ word_comment(document_id="...", position="introduction", text="Consider adding a summary sentence here.")

"suggest shortening the conclusion"
→ word_revision(document_id="...", position="conclusion", original="In conclusion, it can be stated that...", suggested="In summary,...")
```

## Notes

- Positions are text-based hints (paragraph numbers, keyword matches) not precise character offsets.
- Comments are inserted as Word comments (visible in the Review pane).
- Revisions are added as tracked changes, not direct overwrites.
- The agent reads document content before adding feedback to ensure relevance.