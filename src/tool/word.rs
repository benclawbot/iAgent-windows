use super::{Tool, ToolContext, ToolOutput};
#[cfg(windows)]
use anyhow::Context;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
#[cfg(windows)]
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

pub struct WordTool;

impl WordTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WordAction {
    CreateDocument,
    ExtractText,
    ReviewDocument,
}

#[derive(Debug, Deserialize, Serialize)]
struct WordInput {
    action: WordAction,
    title: Option<String>,
    content: Option<String>,
    path: Option<String>,
    visible: Option<bool>,
    save: Option<bool>,
    #[serde(default)]
    suggestions: Vec<WordSuggestion>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WordSuggestion {
    target: Option<String>,
    comment: String,
    replacement: Option<String>,
}

#[async_trait]
impl Tool for WordTool {
    fn name(&self) -> &str {
        "word"
    }

    fn description(&self) -> &str {
        concat!(
            "Automate Microsoft Word documents. Use this instead of generic desktop typing ",
            "when the user asks to create a Word document, read an existing Word document, ",
            "or propose updates with Word's Review features."
        )
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["create_document", "extract_text", "review_document"],
                    "description": "Word automation action. Use create_document for generated drafts, extract_text before reviewing an existing document, and review_document to add Review comments and suggested replacements."
                },
                "title": {
                    "type": "string",
                    "description": "Optional document title for action='create_document'."
                },
                "content": {
                    "type": "string",
                    "description": "Document body for action='create_document'."
                },
                "path": {
                    "type": "string",
                    "description": "Optional .docx path. For create_document this saves the new file; for extract_text/review_document this opens that file. If omitted, extract_text/review_document use the active Word document."
                },
                "visible": {
                    "type": "boolean",
                    "description": "Whether Word should be visible. Defaults to true."
                },
                "save": {
                    "type": "boolean",
                    "description": "Whether review_document should save after adding comments. Defaults to true."
                },
                "suggestions": {
                    "type": "array",
                    "description": "Review comments to add for action='review_document'. Each suggestion is attached to the first matching target text, or to the start of the document when target is omitted/not found.",
                    "items": {
                        "type": "object",
                        "required": ["comment"],
                        "properties": {
                            "target": {
                                "type": "string",
                                "description": "Exact text in the document to anchor the comment to."
                            },
                            "comment": {
                                "type": "string",
                                "description": "Review comment explaining the proposed update."
                            },
                            "replacement": {
                                "type": "string",
                                "description": "Optional suggested replacement text shown in the comment."
                            }
                        },
                        "additionalProperties": false
                    }
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: WordInput = serde_json::from_value(input)?;
        validate_input(&params)?;
        run_word_automation(params).await
    }
}

fn validate_input(params: &WordInput) -> Result<()> {
    match params.action {
        WordAction::CreateDocument => {
            if params.title.as_deref().unwrap_or("").trim().is_empty()
                && params.content.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(anyhow!(
                    "title or content is required for action='create_document'"
                ));
            }
        }
        WordAction::ExtractText => {}
        WordAction::ReviewDocument => {
            if params.suggestions.is_empty() {
                return Err(anyhow!(
                    "suggestions is required for action='review_document'"
                ));
            }
            if params
                .suggestions
                .iter()
                .any(|s| s.comment.trim().is_empty())
            {
                return Err(anyhow!("review suggestion comments must not be empty"));
            }
        }
    }
    Ok(())
}

async fn run_word_automation(params: WordInput) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        let output = tokio::task::spawn_blocking(move || run_word_powershell(&params))
            .await
            .context("word automation task failed")??;
        Ok(format_word_output(output))
    }

    #[cfg(not(windows))]
    {
        let _ = params;
        Err(anyhow!(
            "Microsoft Word automation is only available on Windows"
        ))
    }
}

#[cfg(windows)]
fn run_word_powershell(params: &WordInput) -> Result<Value> {
    let json_path = temp_word_json_path();
    let json_bytes = serde_json::to_vec(params).context("serialize Word input")?;
    std::fs::write(&json_path, json_bytes)
        .with_context(|| format!("write Word automation input {}", json_path.display()))?;

    let script = word_powershell_script(&json_path.to_string_lossy());
    let encoded = encode_powershell_command(&script);
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .output()
        .context("run powershell.exe for Word automation")?;

    let _ = std::fs::remove_file(&json_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "Word automation failed (exit {}): {}{}{}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            stderr.trim(),
            if stderr.trim().is_empty() || stdout.trim().is_empty() {
                ""
            } else {
                "\n"
            },
            stdout.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| anyhow!("Word automation returned no JSON output"))?;
    serde_json::from_str(json_line.trim()).context("parse Word automation output")
}

#[cfg(windows)]
fn word_powershell_script(json_path: &str) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$InputPath = '{input_path}'
$payload = Get-Content -LiteralPath $InputPath -Raw | ConvertFrom-Json

function Get-WordApplication([bool]$Visible) {{
    try {{
        $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application')
    }} catch {{
        $word = New-Object -ComObject Word.Application
    }}
    $word.Visible = $Visible
    return $word
}}

function Get-Document([object]$Word, [string]$Path) {{
    if ([string]::IsNullOrWhiteSpace($Path)) {{
        if ($Word.Documents.Count -lt 1) {{
            throw 'No active Word document is available. Open a document or pass path.'
        }}
        return $Word.ActiveDocument
    }}
    $full = (Resolve-Path -LiteralPath $Path).Path
    foreach ($doc in @($Word.Documents)) {{
        try {{
            if ($doc.FullName -eq $full) {{
                return $doc
            }}
        }} catch {{}}
    }}
    return $Word.Documents.Open($full)
}}

function Add-ReviewComment([object]$Document, [string]$Target, [string]$Comment, [string]$Replacement) {{
    if (-not [string]::IsNullOrWhiteSpace($Replacement)) {{
        $Comment = $Comment + "`r`n`r`nSuggested replacement: " + $Replacement
    }}

    $range = $null
    if (-not [string]::IsNullOrWhiteSpace($Target)) {{
        $range = $Document.Content.Duplicate
        $find = $range.Find
        $find.ClearFormatting()
        $found = $find.Execute($Target)
        if (-not $found) {{
            $range = $null
        }}
    }}
    if ($null -eq $range) {{
        $start = 0
        $end = [Math]::Min(1, [Math]::Max(0, $Document.Content.End - 1))
        $range = $Document.Range($start, $end)
    }}

    [void]$Document.Comments.Add($range, $Comment)
}}

$visible = $true
if ($null -ne $payload.visible) {{
    $visible = [bool]$payload.visible
}}

switch ($payload.action) {{
    'create_document' {{
        $word = Get-WordApplication $visible
        $doc = $word.Documents.Add()
        $title = [string]$payload.title
        $content = [string]$payload.content
        if ([string]::IsNullOrWhiteSpace($title)) {{
            $body = $content
        }} elseif ([string]::IsNullOrWhiteSpace($content)) {{
            $body = $title
        }} else {{
            $body = $title + "`r`r" + $content
        }}
        $doc.Range().Text = $body
        if (-not [string]::IsNullOrWhiteSpace($title)) {{
            try {{
                $doc.Paragraphs.Item(1).Range.Font.Bold = $true
                $doc.Paragraphs.Item(1).Range.Font.Size = 16
            }} catch {{}}
        }}
        if (-not [string]::IsNullOrWhiteSpace([string]$payload.path)) {{
            $savePath = [string]$payload.path
            $parent = Split-Path -Parent $savePath
            if (-not [string]::IsNullOrWhiteSpace($parent)) {{
                [void](New-Item -ItemType Directory -Force -Path $parent)
            }}
            $doc.SaveAs([ref]$savePath)
        }}
        $doc.Activate()
        $result = @{{
            action = 'create_document'
            title = $title
            full_name = $doc.FullName
            characters = $body.Length
            visible = $word.Visible
        }}
    }}
    'extract_text' {{
        $word = Get-WordApplication $visible
        $doc = Get-Document $word ([string]$payload.path)
        $doc.Activate()
        $text = [string]$doc.Content.Text
        $truncated = $false
        if ($text.Length -gt 30000) {{
            $text = $text.Substring(0, 30000)
            $truncated = $true
        }}
        $result = @{{
            action = 'extract_text'
            full_name = $doc.FullName
            name = $doc.Name
            text = $text
            characters = $doc.Content.Text.Length
            truncated = $truncated
        }}
    }}
    'review_document' {{
        $word = Get-WordApplication $visible
        $doc = Get-Document $word ([string]$payload.path)
        $doc.Activate()
        $doc.TrackRevisions = $true
        try {{
            $word.ActiveWindow.View.ShowRevisionsAndComments = $true
        }} catch {{}}
        $count = 0
        foreach ($suggestion in @($payload.suggestions)) {{
            if ($null -eq $suggestion) {{ continue }}
            $comment = [string]$suggestion.comment
            if ([string]::IsNullOrWhiteSpace($comment)) {{ continue }}
            Add-ReviewComment $doc ([string]$suggestion.target) $comment ([string]$suggestion.replacement)
            $count++
        }}
        $save = $true
        if ($null -ne $payload.save) {{
            $save = [bool]$payload.save
        }}
        if ($save) {{
            $doc.Save()
        }}
        $result = @{{
            action = 'review_document'
            full_name = $doc.FullName
            comments_added = $count
            track_revisions = $doc.TrackRevisions
            saved = $save
        }}
    }}
    default {{
        throw ('Unsupported Word action: ' + $payload.action)
    }}
}}

$result | ConvertTo-Json -Compress -Depth 8
"#,
        input_path = escape_powershell_single_quoted(json_path)
    )
}

#[cfg(windows)]
fn escape_powershell_single_quoted(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(windows)]
fn encode_powershell_command(script: &str) -> String {
    let bytes: Vec<u8> = script
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    STANDARD.encode(bytes)
}

#[cfg(windows)]
fn temp_word_json_path() -> std::path::PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("iagent-word-{}-{millis}.json", std::process::id()))
}

fn format_word_output(output: Value) -> ToolOutput {
    let action = output
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("word");
    let full_name = output
        .get("full_name")
        .and_then(Value::as_str)
        .unwrap_or("<unsaved Word document>");

    match action {
        "create_document" => ToolOutput::new(format!(
            "Created Word document: {full_name} ({} characters).",
            output
                .get("characters")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ))
        .with_title("word: create_document")
        .with_metadata(output),
        "extract_text" => {
            let text = output.get("text").and_then(Value::as_str).unwrap_or("");
            let truncated = output
                .get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut metadata = output.clone();
            if let Some(object) = metadata.as_object_mut() {
                object.remove("text");
            }
            let suffix = if truncated {
                "\n\n[Document text truncated to 30000 characters.]"
            } else {
                ""
            };
            ToolOutput::new(format!(
                "Extracted text from Word document: {full_name}\n\n{text}{suffix}"
            ))
            .with_title("word: extract_text")
            .with_metadata(metadata)
        }
        "review_document" => ToolOutput::new(format!(
            "Added {} Word review comment(s) to {full_name}. Track Changes is enabled.",
            output
                .get("comments_added")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ))
        .with_title("word: review_document")
        .with_metadata(output),
        _ => ToolOutput::new(format!("Completed Word action '{action}' on {full_name}."))
            .with_title("word")
            .with_metadata(output),
    }
}

#[cfg(test)]
#[path = "word_tests.rs"]
mod word_tests;
