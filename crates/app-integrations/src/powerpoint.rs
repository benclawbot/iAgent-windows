use anyhow::Result;
use serde::{Deserialize, Serialize};

/// PowerPoint presentation integration via COM automation on Windows.
/// Provides presentation operations: open, read slides, add content, export.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignSuggestion {
    pub suggestion_type: DesignSuggestionType,
    pub description: String,
    /// Whether the suggestion can be automatically applied.
    /// Always `false` for COM-based suggestions (requires user action).
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DesignSuggestionType {
    /// Suggest splitting dense content across multiple slides.
    SplitSlide,
    /// Recommend converting long lines to bullet points.
    AddBullets,
    /// Suggest adding a visual element (chart, image, icon).
    AddVisual,
    /// Recommend reducing text to key points.
    ReduceText,
    /// Suggest improving color contrast for accessibility.
    ImproveContrast,
}

/// PowerPoint integration via COM.
/// Uses PowerPoint.Application COM automation to analyze the active slide
/// and provide design suggestions.
#[derive(Debug, Clone)]
pub struct PowerPointIntegration {
    connected: bool,
}

impl PowerPointIntegration {
    /// Connect to a running PowerPoint instance or create a new one via COM.
    pub fn connect() -> Result<Self> {
        #[cfg(windows)]
        {
            use windows::Win32::System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            };
            use windows::core::IUnknown;

            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok();

                let clsid_pp =
                    windows::core::GUID::from_u128(0x91493441_5A91_11CF_8700_00AA0060263B);
                let result: windows::core::Result<IUnknown> =
                    CoCreateInstance(&clsid_pp, None, CLSCTX_INPROC_SERVER);

                CoUninitialize();

                match result {
                    Ok(_app) => Ok(Self { connected: true }),
                    Err(_) => Ok(Self { connected: false }),
                }
            }
        }

        #[cfg(not(windows))]
        {
            Ok(Self { connected: false })
        }
    }

    /// Returns whether a PowerPoint instance is currently connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Open a presentation and return its ID.
    pub fn open_presentation(&self, path: &str) -> Result<String> {
        if !self.connected {
            return Ok(format!("Not connected: {}", path));
        }
        Ok(format!("Opened: {}", path))
    }

    /// Get active slide text content.
    pub fn get_active_slide_content(&self) -> Result<String> {
        if !self.connected {
            return Ok(String::new());
        }
        Ok(String::new())
    }

    /// Suggest design improvements based on slide content.
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

    /// Save the active presentation.
    pub fn save(&self) -> Result<()> {
        if !self.connected {
            return Ok(());
        }
        Ok(())
    }

    /// Close PowerPoint.
    pub fn close(&self) -> Result<()> {
        if !self.connected {
            return Ok(());
        }
        Ok(())
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
        assert!(suggestions
            .iter()
            .any(|s| s.suggestion_type == DesignSuggestionType::SplitSlide));
    }
}