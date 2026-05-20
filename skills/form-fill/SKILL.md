---
name: form-fill
description: Automatically fill web forms using browser automation and data from memory/documents.
triggers:
  - "fill out a form"
  - "fill form with data"
  - "complete application"
  - "autofill online form"
---

# Form Auto-Fill

## Overview

Fill web forms by combining browser automation with data retrieved from the user's memory graph or provided documents. The agent first retrieves relevant information (name, address, passport data, etc.) from memory, then navigates to the form URL and fills it programmatically. Available when both `browser` and `memory` toolsets are enabled.

## Modes

### Structured Mode
Provide a JSON schema of the form fields. The agent maps memory data to form fields and fills them.

**Input fields:**
- `form_url` (string, required): URL of the form to fill
- `field_schema` (object, required): Field mapping (see example below)

### Natural Language Mode
Describe what form to fill and provide the data source.

**Input fields:**
- `form_url` (string, required): URL of the form
- `instructions` (string, required): What to fill and where (e.g., "Use my passport info to fill the visa application form")

## Field Schema Format

```json
{
  "field_name": "selector_or_label",
  "firstName": "#firstName",
  "lastName": "#lastName",
  "email": "input[name='email']",
  "passport": "#passport-number",
  "address": "#address-line1"
}
```

## Workflow

1. **Retrieve data**: Query memory graph for user data (or accept from instructions)
2. **Navigate**: Open form URL in browser
3. **Map fields**: Match schema selectors to form fields
4. **Fill**: Type data into each field
5. **Submit**: Click submit button or download confirmation

## Prerequisites

- `browser` toolset enabled (headless browser available)
- User data accessible from memory graph

## Example Usage

```
"fill out the DS-160 visa form with my passport info"
→ form_fill(form_url="https://ceac.state.gov/DS160.aspx", instructions="Use passport info from memory to fill all required fields")

"complete the job application at linkedin using my resume data"
→ form_fill(form_url="https://www.linkedin.com/jobs/apply/...", field_schema={
    "firstName": "#input-firstName",
    "lastName": "#input-lastName",
    "email": "#input-email",
    "resume": "#input-resume"
  })

"fill the health insurance enrollment form"
→ form_fill(form_url="https://benefits.company.com/enroll", instructions="Use my personal info and dependents data from memory")
```

## Notes

- The agent infers field selectors from labels and placeholders when not explicitly provided.
- For multi-page forms, the agent walks through each step.
- After submission, the agent captures and returns the confirmation number or receipt.
- Sensitive data (SSN, passwords) is never stored in memory without explicit user consent.