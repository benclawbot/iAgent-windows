---
name: powerpoint
description: Open, read, modify, and export PowerPoint presentations (.pptx) via COM automation on Windows.
triggers:
  - "open presentation"
  - "read slide content"
  - "add slide to presentation"
  - "set slide text"
  - "add comment to slide"
  - "export presentation to pdf"
  - "generate slides"
  - "suggest design improvements"
---

# PowerPoint Presentation Integration

## Overview

Control Microsoft PowerPoint via COM automation on Windows. Read slides, add content, set text, add comments, generate slides from outline, and export to PDF. Available when the `powerpoint` toolset is enabled and running on Windows.

## Actions

### pptx_open
Open a presentation and get its ID.

**Input fields:**
- `path` (string, required): Full path to the .pptx file

**Returns:** Presentation ID for subsequent operations.

### pptx_read
Read content of a specific slide.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `slide_index` (integer, required): 0-based slide index

**Returns:** Text content of the slide.

### pptx_count
Get total number of slides.

**Input fields:**
- `presentation_id` (string, required): Presentation ID

**Returns:** Slide count.

### pptx_add_slide
Add a new slide with title and optional bullets.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `layout` (string, required): Layout name (e.g., "Title and Content", "Blank")
- `title` (string, required): Slide title
- `bullets` (string, optional): Bullet points, one per line

**Returns:** New slide index.

### pptx_set_text
Set text in a text box on a slide.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `slide_index` (integer, required): 0-based slide index
- `textbox_id` (string, required): Text box identifier (e.g., "title", "content")
- `content` (string, required): New text content

**Returns:** Confirmation.

### pptx_comment
Add a comment to a slide.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `slide_index` (integer, required): 0-based slide index
- `text` (string, required): Comment text
- `author` (string, optional, default: "iAgent"): Author name

**Returns:** Comment confirmation.

### pptx_generate
Generate slides from a text outline.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `outline` (string, required): Slide outline (title;title;title or structured text)

**Returns:** List of created slide indices.

### pptx_export_pdf
Export presentation to PDF.

**Input fields:**
- `presentation_id` (string, required): Presentation ID
- `output_path` (string, required): Output PDF path

**Returns:** PDF path confirmation.

### pptx_suggest
Get design improvement suggestions based on slide content.

**Input fields:**
- `content` (string, required): Slide text content to analyze

**Returns:** List of design suggestions (split slide, add bullets, add visual, etc.).

## Prerequisites

- Microsoft PowerPoint must be installed on Windows.
- Requires COM automation support.

## Example Usage

```
"open the quarterly review and add a summary slide at the end"
→ pptx_open(path="C:\\Presentations\\Q4_review.pptx")
→ pptx_count(presentation_id="...")
→ pptx_add_slide(presentation_id="...", layout="Title and Content", title="Q4 Summary", bullets="Revenue: +15%\nClients: 120\nNew markets: 3")

"generate 5 slides about AI trends for the board presentation"
→ pptx_open(path="C:\\Presentations\\board.pptx")
→ pptx_generate(presentation_id="...", outline="Introduction to AI;Current AI Capabilities;Business Applications;Risk Assessment;Next Steps")

"suggest improvements for slide 3"
→ pptx_read(presentation_id="...", slide_index=2)
→ pptx_suggest(content="...")
```

## Notes

- Slide indices are 0-based (slide 1 = index 0).
- Layouts vary by template; common ones include "Title and Content", "Blank", "Two Content".
- Text box IDs are shape names from the PowerPoint object model (e.g., "Title Placeholder 1", "Content Placeholder 2").
- Suggestions include: SplitSlide, AddBullets, AddVisual, ReduceText, ImproveContrast.