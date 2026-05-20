---
name: gmail
description: Compose, send, search, and manage Gmail messages via the Gmail REST API.
triggers:
  - "send email"
  - "draft email"
  - "search emails"
  - "read email thread"
  - "reply to email"
---

# Gmail Integration

## Overview

Interact with Gmail via the REST API (v1). Requires OAuth2 authentication via the user's Google account. Tools are available when the `gmail` toolset is enabled.

## Actions

### gmail_compose
Compose a new email draft.

**Input fields:**
- `to` (string, required): Recipient email address
- `subject` (string, required): Email subject line
- `body` (string, required): Plain text body
- `cc` (string, optional): CC recipients, comma-separated
- `bcc` (string, optional): BCC recipients, comma-separated

**Returns:** Draft ID and preview of the email.

### gmail_send
Send a composed draft or direct email.

**Input fields:**
- `draft_id` (string, optional): Draft ID to send
- `to` (string, required if no draft_id): Recipient
- `subject` (string, required if no draft_id): Subject
- `body` (string, required if no draft_id): Body
- `cc` (string, optional): CC
- `attachments` (array of strings, optional): Local file paths to attach

**Returns:** Sent message ID.

### gmail_search
Search messages using Gmail search syntax.

**Input fields:**
- `query` (string, required): Gmail search query (e.g., "from:sarah subject:project after:2025/01/01")
- `max_results` (integer, optional, default: 10): Number of results to return

**Returns:** List of matching messages with ID, subject, from, date.

### gmail_read
Fetch a specific message or thread by ID.

**Input fields:**
- `message_id` (string, required): The Gmail message ID
- `format` (string, optional, default: "full"): "full", "metadata", or "raw"

**Returns:** Message content, headers, and body.

### gmail_reply
Reply to a thread with the same subject reference.

**Input fields:**
- `message_id` (string, required): Message ID to reply to
- `body` (string, required): Reply body text
- `cc` (string, optional): CC on the reply

**Returns:** Sent message ID.

## Prerequisites

1. User must authorize via Google OAuth2. The OAuth flow is handled automatically on first use.
2. Requires `google` in the connected platforms or a configured Gmail API key.

## Example Usage

```
"email Sarah about the Henderson project and attach Q3_report.pdf"
→ gmail_compose(to="sarah@example.com", subject="Henderson Project Update", body="...", attachments=["Q3_report.pdf"])
→ gmail_send(draft_id="...")

"find emails from Marcus about the API"
→ gmail_search(query="from:marcus subject:API")

"what did I say to Sarah about the Henderson project?"
→ gmail_search(query="to:sarah subject:Henderson")
→ gmail_read(message_id="...")
```

## Notes

- Search syntax follows standard Gmail operators: `from:`, `to:`, `subject:`, `after:`, `before:`, `is:unread`, `has:attachment`, etc.
- Attachments must be local file paths accessible to the agent.
- Drafts are stored server-side in Gmail and can be edited before sending.