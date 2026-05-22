---
name: browser
description: Navigate web pages, fill forms, click elements, extract content, and take screenshots using headless browser automation.
implementation-status: implemented
uses-tool: app
tool-actions: [oc_browser_navigate, oc_browser_snapshot, oc_browser_click, oc_browser_type, oc_browser_press, oc_browser_scroll, oc_browser_screenshot]
triggers:
  - "open a webpage"
  - "fill out a form"
  - "click a button"
  - "extract content from page"
  - "take a screenshot"
  - "navigate to URL"
---

# Browser Automation

## Overview

Control a headless Chromium browser to interact with any web page. Uses Playwright or CDP (Chrome DevTools Protocol) via an MCP server. Available when the `browser` toolset is enabled.

## Actions

### browser_navigate
Navigate to a URL and load the page.

**Input fields:**
- `url` (string, required): Full URL to navigate to

**Returns:** Page snapshot with interactive elements.

### browser_snapshot
Get a text-based snapshot of the current page's interactive elements.

**Input fields:**
- `full` (boolean, optional, default: false): If true, return full page content

**Returns:** List of elements with ref IDs for clicking/typing.

### browser_click
Click an element identified by its ref ID.

**Input fields:**
- `ref` (string, required): Element ref (e.g., "@e5")

### browser_type
Type text into an input field.

**Input fields:**
- `ref` (string, required): Input element ref
- `text` (string, required): Text to type

### browser_press
Press a keyboard key.

**Input fields:**
- `key` (string, required): Key name (e.g., "Enter", "Tab", "Escape")

### browser_scroll
Scroll the page.

**Input fields:**
- `direction` (string, required): "up" or "down"

### browser_screenshot
Capture a screenshot of the current page.

**Input fields:**
- `annotate` (boolean, optional): Overlay numbered labels on interactive elements

**Returns:** Screenshot path for delivery.

## Prerequisites

- Playwright or Chromium binary must be installed on the host machine.
- For MCP integration: `@playwright/mcp-server` added to the MCP pool.

## Example Usage

```
"go to github.com and log me in"
→ browser_navigate(url="https://github.com/login")
→ browser_snapshot()
→ browser_type(ref="@e5", text="username")
→ browser_type(ref="@e6", text="password")
→ browser_click(ref="@e7")

"find the pricing page on stripe.com"
→ browser_navigate(url="https://stripe.com")
→ browser_snapshot()
→ browser_click(ref="@e12")

"what's on this page?" (after navigation)
→ browser_snapshot(full=true)
```

## Notes

- Ref IDs (e.g., `@e5`) are shown in square brackets in snapshots.
- Screenshot annotations overlay [N] labels matching ref @eN.
- Forms can be filled by finding input refs and using browser_type.
- For complex multi-step flows, chain actions together.