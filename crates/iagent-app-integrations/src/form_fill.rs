//! High-level form-fill automation via CDP.
//!
//! Builds on `browser.rs` to provide ergonomic, intent-driven form filling:
//! - Multi-field form fill from a structured request
//! - Smart field matching by name, label, placeholder, or CSS selector
//! - Planning + ambiguity handling before mutation
//! - Dry-run preview mode
//! - Optional before/after screenshots
//! - Human approval gate before submit

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::browser::{CdpBrowser, CdpFormField, CdpInteractable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// CSS selector for the field. If omitted, matching is done by name/id/placeholder/label.
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
    /// Plan only. Do not mutate page state.
    pub dry_run: Option<bool>,
    /// Capture screenshots around execution.
    pub capture_screenshots: Option<bool>,
    /// Require explicit approval before submit click.
    pub submit_requires_approval: Option<bool>,
    /// Caller-provided submit approval token (true => approved).
    pub submit_approved: Option<bool>,
    /// Confidence threshold for automatic field choice (default: 0.8).
    pub ambiguity_threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCandidate {
    pub selector: String,
    pub score: f32,
    pub reasons: Vec<String>,
    pub input_type: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFieldResolution {
    pub request_index: usize,
    pub requested: FormField,
    pub selected_selector: Option<String>,
    pub selected_score: Option<f32>,
    pub selected_reason: Option<String>,
    pub ambiguous: bool,
    pub candidates: Vec<FieldCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFillResult {
    pub success: bool,
    pub message: String,
    pub filled_fields: Vec<String>,
    pub errors: Vec<String>,
    /// Final URL after any navigation.
    pub final_url: Option<String>,
    /// Planned field resolution details, always returned.
    pub plan: Vec<PlannedFieldResolution>,
    /// True if at least one field needs disambiguation or could not be resolved.
    pub requires_user_choice: bool,
    /// True when submit was requested but blocked by approval gate.
    pub approval_required: bool,
    /// Whether submit was attempted and performed.
    pub submit_performed: bool,
    /// Human-readable workflow transcript.
    pub transcript: Vec<String>,
    /// Optional before/after page screenshots as base64 PNG.
    pub before_screenshot_b64: Option<String>,
    pub after_screenshot_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageFieldDescriptor {
    selector: String,
    name: Option<String>,
    id: Option<String>,
    label: Option<String>,
    placeholder: Option<String>,
    input_type: Option<String>,
    visible: bool,
}

/// Fill a form using a CDP browser connection.
pub async fn fill_form(
    browser: &mut CdpBrowser,
    request: &FormFillRequest,
) -> Result<FormFillResult> {
    let dry_run = request.dry_run.unwrap_or(false);
    let capture_screenshots = request.capture_screenshots.unwrap_or(false);
    let submit_requested = request.submit.unwrap_or(false);
    let submit_requires_approval = request.submit_requires_approval.unwrap_or(false);
    let submit_approved = request.submit_approved.unwrap_or(false);
    let ambiguity_threshold = request.ambiguity_threshold.unwrap_or(0.8).clamp(0.0, 1.0);

    let mut filled = Vec::new();
    let mut errors = Vec::new();
    let mut transcript = Vec::new();

    // Optionally navigate first
    if let Some(ref url) = request.url {
        browser.navigate(url).await?;
        transcript.push(format!("navigated to {}", url));
    }

    let before_screenshot_b64 = if capture_screenshots {
        match browser.screenshot().await {
            Ok(bytes) => Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
            Err(err) => {
                transcript.push(format!("before screenshot failed: {err}"));
                None
            }
        }
    } else {
        None
    };

    // Collect available inputs from both interactables and semantic field scan.
    let interactables = browser.get_interactables().await?;
    let page_fields = extract_page_fields(browser).await.unwrap_or_default();
    let by_selector: HashMap<&str, &CdpInteractable> = interactables
        .iter()
        .filter(|i| !i.selector.is_empty())
        .map(|i| (i.selector.as_str(), i))
        .collect();

    let plan = build_fill_plan(
        &request.fields,
        &interactables,
        &page_fields,
        ambiguity_threshold,
    );
    let requires_user_choice = plan
        .iter()
        .any(|p| p.ambiguous || p.selected_selector.is_none());

    if dry_run {
        transcript.push("dry_run=true: plan generated, no fields were mutated".to_string());
    } else if requires_user_choice {
        transcript.push(
            "execution paused: one or more fields require disambiguation or are unresolved"
                .to_string(),
        );
    } else {
        for item in &plan {
            let Some(selector) = item.selected_selector.clone() else {
                errors.push(format!(
                    "Could not resolve selector for request field {}",
                    item.request_index
                ));
                continue;
            };

            // Fill the field
            let cdp_field = CdpFormField {
                selector: selector.clone(),
                value: Some(item.requested.value.clone()),
                input_type: by_selector
                    .get(selector.as_str())
                    .and_then(|i| i.input_type.clone())
                    .or_else(|| {
                        item.candidates
                            .first()
                            .and_then(|candidate| candidate.input_type.clone())
                    })
                    .unwrap_or_else(|| "text".to_string()),
                name: item.requested.name.clone(),
                id: item.requested.id.clone(),
                placeholder: item.requested.placeholder.clone(),
                required: false,
                visible: true,
            };

            if let Err(e) = browser.fill_form(&[cdp_field]).await {
                errors.push(format!("Error filling {}: {}", selector, e));
                continue;
            }

            filled.push(selector);
        }
    }

    let approval_required = submit_requested && submit_requires_approval && !submit_approved;
    let mut submit_performed = false;

    // Handle submit if requested
    if submit_requested {
        let submit_sel = request
            .submit_selector
            .as_deref()
            .unwrap_or("input[type=submit], button[type=submit], button:not([type])");

        if dry_run {
            transcript.push(format!(
                "dry_run: submit planned for selector {}",
                submit_sel
            ));
        } else if requires_user_choice {
            transcript.push("submit skipped: unresolved/ambiguous fields remain".to_string());
        } else if approval_required {
            transcript
                .push("submit blocked: approval required but submit_approved=false".to_string());
        } else if let Err(e) = browser.click(submit_sel).await {
            errors.push(format!("Error clicking submit ({:?}): {}", submit_sel, e));
        } else {
            submit_performed = true;
            transcript.push(format!("submit clicked via {}", submit_sel));
        }
    }

    let after_screenshot_b64 = if capture_screenshots {
        match browser.screenshot().await {
            Ok(bytes) => Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
            Err(err) => {
                transcript.push(format!("after screenshot failed: {err}"));
                None
            }
        }
    } else {
        None
    };

    let final_url = browser.evaluate("window.location.href").await.ok();
    let success = errors.is_empty() && !requires_user_choice && !approval_required;
    let message = if dry_run {
        format!(
            "Dry run complete: {} field(s) planned, {} unresolved",
            plan.len(),
            plan.iter()
                .filter(|p| p.selected_selector.is_none() || p.ambiguous)
                .count()
        )
    } else if requires_user_choice {
        "Execution paused: user disambiguation required".to_string()
    } else if approval_required {
        "Execution paused: submit approval required".to_string()
    } else if errors.is_empty() {
        format!("Filled {} field(s) successfully", filled.len())
    } else {
        format!(
            "Filled {} field(s) with {} error(s)",
            filled.len(),
            errors.len()
        )
    };

    Ok(FormFillResult {
        success,
        message,
        filled_fields: filled,
        errors,
        final_url,
        plan,
        requires_user_choice,
        approval_required,
        submit_performed,
        transcript,
        before_screenshot_b64,
        after_screenshot_b64,
    })
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

fn score_candidate(field: &FormField, candidate: &PageFieldDescriptor) -> Option<FieldCandidate> {
    let mut score = 0.0_f32;
    let mut reasons = Vec::new();

    if let Some(selector) = &field.selector
        && selector == &candidate.selector
    {
        score += 1.0;
        reasons.push("selector_exact".to_string());
    }

    if let (Some(request_id), Some(candidate_id)) = (&field.id, &candidate.id)
        && normalize_text(request_id) == normalize_text(candidate_id)
    {
        score += 0.95;
        reasons.push("id_exact".to_string());
    }

    if let (Some(request_name), Some(candidate_name)) = (&field.name, &candidate.name)
        && normalize_text(request_name) == normalize_text(candidate_name)
    {
        score += 0.9;
        reasons.push("name_exact".to_string());
    }

    if let (Some(request_label), Some(candidate_label)) = (&field.label, &candidate.label) {
        let req = normalize_text(request_label);
        let cand = normalize_text(candidate_label);
        if req == cand {
            score += 0.85;
            reasons.push("label_exact".to_string());
        } else if !req.is_empty()
            && !cand.is_empty()
            && (cand.contains(&req) || req.contains(&cand))
        {
            score += 0.65;
            reasons.push("label_partial".to_string());
        }
    }

    if let (Some(request_placeholder), Some(candidate_placeholder)) =
        (&field.placeholder, &candidate.placeholder)
    {
        let req = normalize_text(request_placeholder);
        let cand = normalize_text(candidate_placeholder);
        if req == cand {
            score += 0.8;
            reasons.push("placeholder_exact".to_string());
        } else if !req.is_empty()
            && !cand.is_empty()
            && (cand.contains(&req) || req.contains(&cand))
        {
            score += 0.55;
            reasons.push("placeholder_partial".to_string());
        }
    }

    if let Some(selector_hint) = &field.selector
        && !selector_hint.is_empty()
        && selector_hint != &candidate.selector
        && candidate.selector.contains(selector_hint)
    {
        score += 0.4;
        reasons.push("selector_partial".to_string());
    }

    if !candidate.visible {
        score -= 0.15;
        reasons.push("visibility_penalty".to_string());
    }

    if score <= 0.0 {
        return None;
    }

    Some(FieldCandidate {
        selector: candidate.selector.clone(),
        score: score.min(1.0),
        reasons,
        input_type: candidate.input_type.clone(),
        name: candidate.name.clone(),
        id: candidate.id.clone(),
        label: candidate.label.clone(),
        placeholder: candidate.placeholder.clone(),
        visible: candidate.visible,
    })
}

fn build_fill_plan(
    fields: &[FormField],
    interactables: &[CdpInteractable],
    page_fields: &[PageFieldDescriptor],
    ambiguity_threshold: f32,
) -> Vec<PlannedFieldResolution> {
    let mut descriptor_by_selector: HashMap<String, PageFieldDescriptor> = HashMap::new();
    for descriptor in page_fields {
        descriptor_by_selector
            .entry(descriptor.selector.clone())
            .or_insert_with(|| descriptor.clone());
    }

    for interactable in interactables {
        if interactable.selector.is_empty() {
            continue;
        }
        descriptor_by_selector
            .entry(interactable.selector.clone())
            .or_insert_with(|| PageFieldDescriptor {
                selector: interactable.selector.clone(),
                name: None,
                id: None,
                label: interactable.text.clone(),
                placeholder: None,
                input_type: interactable.input_type.clone(),
                visible: interactable.visible,
            });
    }

    let descriptors: Vec<PageFieldDescriptor> = descriptor_by_selector.into_values().collect();
    let mut plan = Vec::with_capacity(fields.len());

    for (idx, field) in fields.iter().enumerate() {
        let mut candidates: Vec<FieldCandidate> = descriptors
            .iter()
            .filter_map(|descriptor| score_candidate(field, descriptor))
            .collect();

        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        let selected = candidates.first().cloned();
        let second = candidates.get(1);

        let mut selected_selector = None;
        let mut selected_score = None;
        let mut selected_reason = None;
        let mut ambiguous = false;

        if let Some(top) = selected {
            let close_second = second
                .map(|other| {
                    (top.score - other.score).abs() < 0.08 && other.score >= ambiguity_threshold
                })
                .unwrap_or(false);
            ambiguous = close_second || top.score < ambiguity_threshold;

            if !ambiguous {
                selected_selector = Some(top.selector.clone());
                selected_score = Some(top.score);
                selected_reason = Some(top.reasons.join(","));
            } else {
                selected_score = Some(top.score);
            }
        }

        if selected_selector.is_none()
            && !ambiguous
            && let Some(explicit_selector) = &field.selector
            && !explicit_selector.is_empty()
        {
            selected_selector = Some(explicit_selector.clone());
            selected_score = Some(0.5);
            selected_reason = Some("explicit_selector_fallback".to_string());
        }

        plan.push(PlannedFieldResolution {
            request_index: idx,
            requested: field.clone(),
            selected_selector,
            selected_score,
            selected_reason,
            ambiguous,
            candidates,
        });
    }

    plan
}

async fn extract_page_fields(browser: &CdpBrowser) -> Result<Vec<PageFieldDescriptor>> {
    let script = r#"
(() => {
  const cssEscape = (window.CSS && CSS.escape)
    ? CSS.escape
    : (v) => String(v).replace(/["\\]/g, "\\$&");

  const selectorFor = (el) => {
    if (!el || !(el instanceof Element)) return "";
    if (el.id) return `#${cssEscape(el.id)}`;
    if (el.getAttribute("name")) {
      return `${el.tagName.toLowerCase()}[name="${cssEscape(el.getAttribute("name"))}"]`;
    }
    const chain = [];
    let cur = el;
    while (cur && cur.nodeType === Node.ELEMENT_NODE && chain.length < 8) {
      let part = cur.tagName.toLowerCase();
      if (cur.parentElement) {
        const sibs = Array.from(cur.parentElement.children).filter(n => n.tagName === cur.tagName);
        if (sibs.length > 1) part += `:nth-of-type(${sibs.indexOf(cur) + 1})`;
      }
      chain.unshift(part);
      cur = cur.parentElement;
    }
    return chain.join(" > ");
  };

  const text = (node) => (node?.innerText || node?.textContent || "").trim() || null;
  const labelFor = (el) => {
    const aria = el.getAttribute("aria-label");
    if (aria) return aria.trim();

    const labelledBy = el.getAttribute("aria-labelledby");
    if (labelledBy) {
      const joined = labelledBy
        .split(/\s+/)
        .map(id => text(document.getElementById(id)))
        .filter(Boolean)
        .join(" ");
      if (joined) return joined;
    }

    if (el.labels && el.labels.length > 0) {
      const joined = Array.from(el.labels).map(l => text(l)).filter(Boolean).join(" ");
      if (joined) return joined;
    }

    const wrapped = el.closest("label");
    if (wrapped) {
      const clone = wrapped.cloneNode(true);
      clone.querySelectorAll("input,select,textarea,button").forEach(n => n.remove());
      const wrappedText = text(clone);
      if (wrappedText) return wrappedText;
    }

    if (el.id) {
      const byFor = document.querySelector(`label[for="${cssEscape(el.id)}"]`);
      const byForText = text(byFor);
      if (byForText) return byForText;
    }

    const prev = el.previousElementSibling;
    if (prev) {
      const prevText = text(prev);
      if (prevText) return prevText;
    }

    return null;
  };

  const isVisible = (el) => {
    const rect = el.getBoundingClientRect();
    const style = window.getComputedStyle(el);
    return rect.width > 0 &&
      rect.height > 0 &&
      style.visibility !== "hidden" &&
      style.display !== "none" &&
      style.opacity !== "0";
  };

  const fields = Array.from(document.querySelectorAll("input, textarea, select"))
    .map((el) => ({
      selector: selectorFor(el),
      name: el.getAttribute("name"),
      id: el.id || null,
      label: labelFor(el),
      placeholder: el.getAttribute("placeholder"),
      input_type: (el.getAttribute("type") || el.tagName || "").toLowerCase(),
      visible: isVisible(el),
    }))
    .filter((entry) => entry.selector);

  return JSON.stringify(fields);
})()
"#;

    let payload = browser.evaluate(script).await?;
    let fields: Vec<PageFieldDescriptor> = serde_json::from_str(&payload)
        .with_context(|| "Failed parsing semantic field extraction payload".to_string())?;
    Ok(fields)
}

/// Find a selector for a field given interactables and field descriptor.
fn find_field_selector(interactables: &[CdpInteractable], field: &FormField) -> String {
    // Try exact name/id match first
    for i in interactables {
        if let Some(ref name) = field.name
            && (i.selector.contains(&format!("[name=\"{}\"]", name))
                || i.selector.contains(&format!("name=\"{}\"", name)))
        {
            return i.selector.clone();
        }
        if let Some(ref id) = field.id
            && (i.selector.contains(&format!("#{}", id))
                || i.selector.contains(&format!("[id=\"{}\"]", id)))
        {
            return i.selector.clone();
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
            dry_run: Some(true),
            capture_screenshots: Some(true),
            submit_requires_approval: Some(true),
            submit_approved: Some(false),
            ambiguity_threshold: Some(0.8),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("dry_run"));
    }

    #[test]
    fn candidate_scoring_prefers_exact_id() {
        let request = FormField {
            selector: None,
            name: Some("email".to_string()),
            id: Some("email".to_string()),
            label: Some("Email".to_string()),
            placeholder: None,
            value: "a@b.c".to_string(),
            checked: None,
        };
        let descriptor = PageFieldDescriptor {
            selector: "#email".to_string(),
            name: Some("email".to_string()),
            id: Some("email".to_string()),
            label: Some("Email".to_string()),
            placeholder: None,
            input_type: Some("text".to_string()),
            visible: true,
        };
        let candidate = score_candidate(&request, &descriptor).expect("candidate");
        assert!(candidate.score >= 0.9);
    }
}
