use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeSuggestionRequest {
    pub file_path: String,
    pub selection: String,
    pub language_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeSuggestionResponse {
    pub summary: String,
    pub suggestion: String,
}

pub fn build_lsp_payload(request: &CodeSuggestionRequest) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "method": "workspace/executeCommand",
        "params": {
            "command": "jcode.suggestRefactor",
            "arguments": [{
                "filePath": request.file_path,
                "selection": request.selection,
                "languageId": request.language_id
            }]
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_payload_shape_is_stable() {
        let req = CodeSuggestionRequest {
            file_path: "src/main.rs".to_string(),
            selection: "let x = x + 1;".to_string(),
            language_id: "rust".to_string(),
        };
        let payload = build_lsp_payload(&req).expect("payload");
        assert_eq!(payload["method"], "workspace/executeCommand");
        assert_eq!(payload["params"]["arguments"][0]["languageId"], "rust");
    }
}

