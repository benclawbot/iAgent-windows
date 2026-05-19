use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub fn connect() -> Result<Self> {
        Ok(Self)
    }

    pub fn get_active_slide_content(&self) -> Result<String> {
        Ok(String::new())
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

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

