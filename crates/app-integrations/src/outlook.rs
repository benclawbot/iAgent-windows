use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OutlookIntegration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailDraft {
    pub subject: String,
    pub body: String,
    pub recipients: Vec<String>,
}

impl OutlookIntegration {
    pub fn connect() -> Result<Self> {
        Ok(Self)
    }

    pub fn get_active_email_draft(&self) -> Result<EmailDraft> {
        Ok(EmailDraft {
            subject: String::new(),
            body: String::new(),
            recipients: Vec::new(),
        })
    }

    pub fn apply_email_suggestion(&self, _new_body: &str) -> Result<()> {
        Ok(())
    }
}

