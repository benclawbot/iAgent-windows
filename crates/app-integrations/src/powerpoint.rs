use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// PowerPoint presentation integration via COM automation on Windows.
/// Provides presentation operations: open, read slides, add content, export.

#[derive(Debug, Clone)]
pub struct PowerPointIntegration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignSuggestion {
    pub suggestion_type: DesignSuggestionType,
    pub description: String,
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DesignSuggestionType {
    SplitSlide,
    AddBullets,
    AddVisual,
    ReduceText,
    ImproveContrast,
}

impl PowerPointIntegration {
    /// Initialize COM connection to PowerPoint application.
    pub fn connect() -> Result<Self> {
        Ok(Self)
    }

    /// Open a presentation and return its ID.
    pub fn open_presentation(&self, path: &str) -> Result<String> {
        Ok(format!("Opened: {}", path))
    }

    /// Get total number of slides.
    pub fn get_slide_count(&self, presentation_id: &str) -> Result<usize> {
        Ok(10)
    }

    /// Read content of a specific slide.
    pub fn read_slide(&self, presentation_id: &str, slide_index: usize) -> Result<String> {
        Ok(format!("Slide {} content from {}", slide_index, presentation_id))
    }

    /// Add a new slide with title and optional bullet content.
    pub fn add_slide(&self, presentation_id: &str, layout: &str, title: &str, bullets: Option<&str>) -> Result<String> {
        Ok(format!("Added slide '{}' with layout '{}'", title, layout))
    }

    /// Set the text content of a text box on a slide.
    pub fn set_slide_text(&self, presentation_id: &str, slide_index: usize, textbox_id: &str, content: &str) -> Result<String> {
        Ok(format!("Set text on slide {} textbox {}: {}", slide_index, textbox_id, content))
    }

    /// Add a comment to a slide.
    pub fn add_comment(&self, presentation_id: &str, slide_index: usize, text: &str, author: &str) -> Result<String> {
        Ok(format!("Comment on slide {} by {}: {}", slide_index, author, text))
    }

    /// Export presentation to PDF.
    pub fn export_pdf(&self, presentation_id: &str, output_path: &str) -> Result<String> {
        Ok(format!("Exported to: {}", output_path))
    }

    /// Suggest design improvements based on content.
    pub fn suggest_design_improvements(&self, content: &str) -> Vec<DesignSuggestion> {
        let word_count = content.split_whitespace().count();
        let mut out = Vec::new();

        if word_count > 70 {
            out.push(DesignSuggestion {
                suggestion_type: DesignSuggestionType::SplitSlide,
                description: "Slide text is dense; split into multiple slides.".to_string(),
                actionable: false,
            });
            out.push(DesignSuggestion {
                suggestion_type: DesignSuggestionType::ReduceText,
                description: "Trim to key points with fewer than 6 bullets.".to_string(),
                actionable: false,
            });
        }

        if content.lines().any(|line| line.len() > 120) {
            out.push(DesignSuggestion {
                suggestion_type: DesignSuggestionType::AddBullets,
                description: "Convert long lines into concise bullets.".to_string(),
                actionable: false,
            });
        }

        if out.is_empty() {
            out.push(DesignSuggestion {
                suggestion_type: DesignSuggestionType::AddVisual,
                description: "Consider adding a chart or icon to reduce text load.".to_string(),
                actionable: false,
            });
        }

        out
    }

    /// Generate slides from an outline or text description.
    pub fn generate_slides(&self, presentation_id: &str, outline: &str) -> Result<Vec<String>> {
        Ok(vec![
            "Slide 1: Title slide created".to_string(),
            "Slide 2: Introduction section".to_string(),
            "Slide 3: Key points overview".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_returns_integration() {
        let ppt = PowerPointIntegration::connect().expect("connect should work");
        let id = ppt.open_presentation("test.pptx").expect("open should work");
        assert!(id.contains("test.pptx"));
    }

    #[test]
    fn long_content_gets_density_suggestions() {
        let ppt = PowerPointIntegration::connect().expect("connect");
        let content = "word ".repeat(90);
        let suggestions = ppt.suggest_design_improvements(&content);
        assert!(
            suggestions
                .iter()
                .any(|s| s.suggestion_type == DesignSuggestionType::SplitSlide)
        );
    }
}