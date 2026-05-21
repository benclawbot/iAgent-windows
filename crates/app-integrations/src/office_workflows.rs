//! Productized Office workflows built on top of `officecli` primitives.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::officecli;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    SummarizeWordDoc,
    UpdateExcelFromCsv,
    GeneratePowerpointFromNotes,
    ExtractActionItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub workflow: WorkflowKind,
    pub input_path: String,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub csv_path: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub success: bool,
    pub workflow: WorkflowKind,
    pub executed: bool,
    pub message: String,
    pub steps: Vec<WorkflowStep>,
    pub preview: serde_json::Value,
    pub errors: Vec<String>,
}

pub fn run_workflow(request: &WorkflowRequest) -> Result<WorkflowResult> {
    match request.workflow {
        WorkflowKind::SummarizeWordDoc => summarize_word_doc(request),
        WorkflowKind::UpdateExcelFromCsv => update_excel_from_csv(request),
        WorkflowKind::GeneratePowerpointFromNotes => generate_powerpoint_from_notes(request),
        WorkflowKind::ExtractActionItems => extract_action_items(request),
    }
}

fn summarize_word_doc(request: &WorkflowRequest) -> Result<WorkflowResult> {
    let dry_run = request.dry_run.unwrap_or(true);
    let text = officecli::text(&request.input_path, None, None)?;
    let sentences = split_sentences(&text);
    let summary = sentences.into_iter().take(5).collect::<Vec<_>>().join(" ");
    let preview = serde_json::json!({
        "summary": summary,
        "source_chars": text.chars().count()
    });

    Ok(WorkflowResult {
        success: true,
        workflow: WorkflowKind::SummarizeWordDoc,
        executed: !dry_run,
        message: if dry_run {
            "Preview generated for Word summary workflow".to_string()
        } else {
            "Word summary workflow executed".to_string()
        },
        steps: vec![
            WorkflowStep {
                step: "load_text".to_string(),
                details: format!("Loaded {}", request.input_path),
            },
            WorkflowStep {
                step: "summarize".to_string(),
                details: "Generated extractive summary".to_string(),
            },
        ],
        preview,
        errors: Vec::new(),
    })
}

fn update_excel_from_csv(request: &WorkflowRequest) -> Result<WorkflowResult> {
    let dry_run = request.dry_run.unwrap_or(true);
    let csv_path = request
        .csv_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("csv_path is required for update_excel_from_csv"))?;
    let csv_text = std::fs::read_to_string(csv_path)
        .with_context(|| format!("failed reading CSV file {}", csv_path))?;
    let rows = parse_csv_rows(&csv_text);
    if rows.is_empty() {
        bail!("CSV appears empty: {}", csv_path);
    }

    let mut errors = Vec::new();
    let mut executed_cells = 0usize;
    if !dry_run {
        for (ridx, row) in rows.iter().enumerate() {
            for (cidx, value) in row.iter().enumerate() {
                let cell = excel_cell_ref(ridx + 1, cidx + 1);
                if let Err(err) = officecli::xlsx_set_cell(&request.input_path, &cell, value) {
                    errors.push(format!("{}: {}", cell, err));
                } else {
                    executed_cells += 1;
                }
            }
        }
    }

    let preview = serde_json::json!({
        "rows": rows.len(),
        "columns_first_row": rows.first().map(|r| r.len()).unwrap_or(0),
        "first_row": rows.first().cloned().unwrap_or_default(),
        "planned_cells": rows.iter().map(|r| r.len()).sum::<usize>()
    });

    Ok(WorkflowResult {
        success: errors.is_empty(),
        workflow: WorkflowKind::UpdateExcelFromCsv,
        executed: !dry_run,
        message: if dry_run {
            "Preview generated for Excel update workflow".to_string()
        } else {
            format!("Updated {} cells from CSV", executed_cells)
        },
        steps: vec![
            WorkflowStep {
                step: "parse_csv".to_string(),
                details: format!("Parsed {} rows from {}", rows.len(), csv_path),
            },
            WorkflowStep {
                step: "apply_cells".to_string(),
                details: if dry_run {
                    "Dry run mode: no writes performed".to_string()
                } else {
                    format!("Wrote {} cells", executed_cells)
                },
            },
        ],
        preview,
        errors,
    })
}

fn generate_powerpoint_from_notes(request: &WorkflowRequest) -> Result<WorkflowResult> {
    let dry_run = request.dry_run.unwrap_or(true);
    let output_path = request.output_path.as_deref().ok_or_else(|| {
        anyhow::anyhow!("output_path is required for generate_powerpoint_from_notes")
    })?;
    let notes = if let Some(notes) = request.notes.as_deref() {
        notes.to_string()
    } else if request.input_path.is_empty() {
        String::new()
    } else {
        std::fs::read_to_string(&request.input_path).unwrap_or_default()
    };
    let bullets = extract_tasks_like_items(&notes, request.max_items.unwrap_or(8));

    let mut errors = Vec::new();
    if !dry_run {
        if let Err(err) = officecli::create(output_path)
            .with_context(|| format!("failed creating presentation {}", output_path))
        {
            errors.push(err.to_string());
        }
        if let Err(err) = officecli::pptx_add_slide(output_path, None) {
            errors.push(format!("add slide failed: {}", err));
        }
        for (idx, bullet) in bullets.iter().enumerate() {
            let y = format!("{}", 120 + (idx as i32 * 40));
            if let Err(err) =
                officecli::pptx_add_textbox(output_path, "/slide[1]", bullet, Some("80"), Some(&y))
            {
                errors.push(format!("add textbox {} failed: {}", idx + 1, err));
            }
        }
    }

    let preview = serde_json::json!({
        "output_path": output_path,
        "bullet_count": bullets.len(),
        "bullets": bullets,
    });

    Ok(WorkflowResult {
        success: errors.is_empty(),
        workflow: WorkflowKind::GeneratePowerpointFromNotes,
        executed: !dry_run,
        message: if dry_run {
            "Preview generated for PowerPoint workflow".to_string()
        } else {
            format!("Generated presentation draft at {}", output_path)
        },
        steps: vec![
            WorkflowStep {
                step: "extract_bullets".to_string(),
                details: "Converted notes into slide bullets".to_string(),
            },
            WorkflowStep {
                step: "render_presentation".to_string(),
                details: if dry_run {
                    "Dry run mode: no presentation written".to_string()
                } else {
                    "Created slide and textboxes via OfficeCLI".to_string()
                },
            },
        ],
        preview,
        errors,
    })
}

fn extract_action_items(request: &WorkflowRequest) -> Result<WorkflowResult> {
    let text = officecli::text(&request.input_path, None, None)?;
    let max_items = request.max_items.unwrap_or(12);
    let action_items = extract_tasks_like_items(&text, max_items);
    let preview = serde_json::json!({
        "action_items": action_items,
        "count": action_items.len()
    });

    Ok(WorkflowResult {
        success: true,
        workflow: WorkflowKind::ExtractActionItems,
        executed: !request.dry_run.unwrap_or(true),
        message: format!("Extracted {} action items", action_items.len()),
        steps: vec![
            WorkflowStep {
                step: "load_text".to_string(),
                details: format!("Loaded {}", request.input_path),
            },
            WorkflowStep {
                step: "extract_actions".to_string(),
                details: "Identified task-like lines".to_string(),
            },
        ],
        preview,
        errors: Vec::new(),
    })
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("{}.", s))
        .collect()
}

fn parse_csv_rows(csv_text: &str) -> Vec<Vec<String>> {
    csv_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').map(|col| col.trim().to_string()).collect())
        .collect()
}

fn excel_cell_ref(row: usize, col: usize) -> String {
    let mut col_idx = col;
    let mut letters = String::new();
    while col_idx > 0 {
        let rem = (col_idx - 1) % 26;
        letters.insert(0, (b'A' + rem as u8) as char);
        col_idx = (col_idx - 1) / 26;
    }
    format!("/sheet[1]/{}{}", letters, row)
}

fn extract_tasks_like_items(text: &str, max_items: usize) -> Vec<String> {
    let task_markers = [
        "todo",
        "action item",
        "next step",
        "follow up",
        "please",
        "must",
        "should",
    ];
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        let bullet = trimmed.starts_with("- ") || trimmed.starts_with("* ");
        let marked = task_markers.iter().any(|marker| lower.contains(marker));
        if bullet || marked {
            out.push(
                trimmed
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .to_string(),
            );
        }
        if out.len() >= max_items {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_ref_handles_multi_letter_columns() {
        assert_eq!(excel_cell_ref(1, 1), "/sheet[1]/A1");
        assert_eq!(excel_cell_ref(3, 27), "/sheet[1]/AA3");
        assert_eq!(excel_cell_ref(10, 52), "/sheet[1]/AZ10");
    }

    #[test]
    fn task_extraction_finds_marked_lines() {
        let text = "Intro\n- Finish budget\nPlease schedule review\nFYI only";
        let tasks = extract_tasks_like_items(text, 10);
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].contains("Finish budget"));
    }
}
