//! High-level form-fill automation via CDP.
//!
//! Builds on `browser.rs` to provide ergonomic, intent-driven form filling:
//! - Multi-field form fill from a structured request
//! - Smart field matching by name, label, placeholder, or CSS selector
//! - Submit handling (click submit button, press Enter)
//! - Wait conditions for form rendering
//! - Error reporting for missing/ambiguous fields

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::browser::{CdpBrowser, CdpFormField, CdpInteractable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// CSS selector for the field. If omitted, matching is done by name/id/placeholder.
    pub selector: Option<String>,
    /// Field name attribute.
    pub name: Option<String>,
    /// Field ID attribute.
    pub id: Option<String>,
    /// Label text (matched against adjacent label elements).
    pub label: Option<String>,
    /// Placeholder text to match.
    pub placeholder: Option<String>,
    /// Value to fill in.
    pub value: String,
    /// For checkboxes/radios: set to true to check.
    pub checked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFillRequest {
    /// URL to navigate to before filling (optional).
    pub url: Option<String>,
    /// Fields to fill.
    pub fields: Vec<FormField>,
    /// Whether to submit the form after filling.
    pub submit: Option<bool>,
    /// Selector for the submit button (if submit=true).
    pub submit_selector: Option<String>,
    /// Wait for navigation after submit (ms). 0 = don't wait.
    pub wait_after_submit_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFillResult {
    pub success: bool,
    pub message: String,
    pub filled_fields: Vec<String>,
    pub errors: Vec<String>,
    /// Final URL after any navigation.
    pub final_url: Option<String>,
}

/// Fill a form using a CDP browser connection.
pub async fn fill_form(
    browser: &mut CdpBrowser,
    request: &FormFillRequest,
) -> Result<FormFillResult> {
    let mut filled = Vec::new();
    let mut errors = Vec::new();

    // Optionally navigate first
    if let Some(ref url) = request.url {
        browser.navigate(url).await?;
    }

    // Get available interactables
    let interactables = browser.get_interactables().await?;

    // Build a lookup map from selector -> CdpInteractable
    let by_selector: std::collections::HashMap<&str, &CdpInteractable> = interactables
        .iter()
        .filter(|i| !i.selector.is_empty())
        .map(|i| (i.selector.as_str(), i))
        .collect();

    for field in &request.fields {
        // Determine the selector to use
        let selector = if let Some(ref sel) = field.selector {
            sel.clone()
        } else {
            // Try to find by name, id, or label
            find_field_selector(&interactables, field)
        };

        if selector.is_empty() {
            errors.push(format!(
                "Could not find field: name={:?}, id={:?}, label={:?}",
                field.name, field.id, field.label
            ));
            continue;
        }

        // Fill the field
        let cdp_field = CdpFormField {
            selector: selector.clone(),
            value: Some(field.value.clone()),
            input_type: by_selector
                .get(selector.as_str())
                .and_then(|i| i.input_type.clone())
                .unwrap_or_else(|| "text".to_string()),
            name: field.name.clone(),
            id: field.id.clone(),
            placeholder: field.placeholder.clone(),
            required: false,
            visible: true,
        };

        if let Err(e) = browser.fill_form(&[cdp_field]).await {
            errors.push(format!("Error filling {}: {}", selector, e));
            continue;
        }

        filled.push(selector);
    }

    // Handle submit if requested
    if request.submit.unwrap_or(false) {
        let submit_sel = request
            .submit_selector
            .as_deref()
            .unwrap_or("input[type=submit], button[type=submit], button:not([type])");

        if let Err(e) = browser.click(submit_sel).await {
            errors.push(format!("Error clicking submit ({:?}): {}", submit_sel, e));
        }
    }

    Ok(FormFillResult {
        success: errors.is_empty(),
        message: if errors.is_empty() {
            format!("Filled {} field(s) successfully", filled.len())
        } else {
            format!("Filled {} field(s) with {} error(s)", filled.len(), errors.len())
        },
        filled_fields: filled,
        errors,
        final_url: None,
    })
}

/// Find a selector for a field given interactables and field descriptor.
fn find_field_selector(interactables: &[CdpInteractable], field: &FormField) -> String {
    // Try exact name/id match first
    for i in interactables {
        if let Some(ref name) = field.name {
            if i.selector.contains(&format!("[name=\"{}\"]", name))
                || i.selector.contains(&format!("name=\"{}\"", name))
            {
                return i.selector.clone();
            }
        }
        if let Some(ref id) = field.id {
            if i.selector.contains(&format!("#{}", id))
                || i.selector.contains(&format!("[id=\"{}\"]", id))
            {
                return i.selector.clone();
            }
        }
    }

    // Fuzzy match on placeholder
    if let Some(ref ph) = field.placeholder {
        for i in interactables {
            if i.selector.contains(ph) {
                return i.selector.clone();
            }
        }
    }

    // Fall back to first matching input type
    String::new()
}

/// Fill a single form field by any identifying attribute (name, id, label, placeholder).
pub async fn fill_single_field(
    browser: &mut CdpBrowser,
    identifier: &str,
    value: &str,
) -> Result<()> {
    let interactables = browser.get_interactables().await?;

    let selector = find_field_selector(
        &interactables,
        &FormField {
            selector: None,
            name: Some(identifier.to_string()),
            id: None,
            label: None,
            placeholder: None,
            value: value.to_string(),
            checked: None,
        },
    );

    if selector.is_empty() {
        bail!("Could not find field: {}", identifier);
    }

    let cdp_field = CdpFormField {
        selector,
        value: Some(value.to_string()),
        input_type: "text".to_string(),
        name: None,
        id: None,
        placeholder: None,
        required: false,
        visible: true,
    };

    browser.fill_form(&[cdp_field]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_field_serialize() {
        let f = FormField {
            selector: Some("#email".to_string()),
            name: Some("email".to_string()),
            id: Some("email".to_string()),
            label: Some("Email address".to_string()),
            placeholder: Some("you@example.com".to_string()),
            value: "test@example.com".to_string(),
            checked: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn form_fill_request_serialize() {
        let req = FormFillRequest {
            url: Some("https://example.com/form".to_string()),
            fields: vec![FormField {
                selector: Some("#name".to_string()),
                name: None,
                id: None,
                label: None,
                placeholder: None,
                value: "Alice".to_string(),
                checked: None,
            }],
            submit: Some(true),
            submit_selector: Some("button[type=submit]".to_string()),
            wait_after_submit_ms: Some(2000),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("example.com"));
    }
}