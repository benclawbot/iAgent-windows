param(
    [Parameter(Mandatory = $true)]
    [string]$Goal,
    [string]$OutputDir = "",
    [bool]$OpenWhenDone = $true
)

$ErrorActionPreference = "Stop"

function Get-SafeFileName {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return "Document"
    }
    $clean = $Text -replace '[<>:"/\\|?*]', ""
    $clean = $clean.Trim()
    if ([string]::IsNullOrWhiteSpace($clean)) {
        return "Document"
    }
    if ($clean.Length -gt 80) {
        $clean = $clean.Substring(0, 80).Trim()
    }
    return $clean
}

function Get-DocTopic {
    param([string]$Text)

    $topicMatch = [regex]::Match($Text, '(?:on|about)\s+([a-z0-9][^.,;\n\r]+)', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($topicMatch.Success) {
        $topic = $topicMatch.Groups[1].Value.Trim()
        if (-not [string]::IsNullOrWhiteSpace($topic)) {
            return $topic
        }
    }
    return "the requested topic"
}

function Get-DocTitle {
    param([string]$Text)

    $titleMatch = [regex]::Match($Text, 'titled\s+"([^"]+)"', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($titleMatch.Success) {
        return $titleMatch.Groups[1].Value.Trim()
    }

    $topic = Get-DocTopic -Text $Text
    if (-not [string]::IsNullOrWhiteSpace($topic) -and $topic -ne "the requested topic") {
        return ($topic.Substring(0, 1).ToUpperInvariant() + $topic.Substring(1))
    }

    $clauseMatch = [regex]::Match(
        $Text,
        '(?:about|on|for|regarding|concerning)\s+(.+?)(?=(?:\s+(?:and|with|including|covering|save|open|then|please)\b)|[.;:\n\r]|$)',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
    if ($clauseMatch.Success) {
        $clause = $clauseMatch.Groups[1].Value.Trim()
        if (-not [string]::IsNullOrWhiteSpace($clause)) {
            return ($clause.Substring(0, 1).ToUpperInvariant() + $clause.Substring(1))
        }
    }

    return "Project Brief"
}

function Get-Sections {
    param(
        [string]$Text,
        [string]$Title,
        [string]$Topic
    )

    $subject = $Topic
    if ([string]::IsNullOrWhiteSpace($subject) -or $subject -eq "the requested topic") {
        $subject = $Title
    }
    if ([string]::IsNullOrWhiteSpace($subject)) {
        $subject = "the requested topic"
    }

    $sections = @()
    $matches = [regex]::Matches(
        $Text,
        '\d+\)\s*([^:;\n\r]+)(?::\s*([^;\n\r]+))?',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
    foreach ($m in $matches) {
        $h = $m.Groups[1].Value.Trim()
        $b = $m.Groups[2].Value.Trim()
        if ([string]::IsNullOrWhiteSpace($h)) {
            continue
        }
        if ([string]::IsNullOrWhiteSpace($b)) {
            $b = "This section explains the role of $h in relation to $subject, including practical details, tradeoffs, and next steps."
        }
        $sections += [PSCustomObject]@{
            Heading = $h
            Body = $b
        }
    }

    if ($sections.Count -gt 0) {
        return $sections
    }

    if ($subject -match '(?i)\b(ai|artificial intelligence|machine learning|large language model|llm|agent)\b') {
        return @(
            [PSCustomObject]@{
                Heading = "Executive Summary"
                Body    = "AI opportunities in 2026 are strongest where the workflow is repetitive, data-rich, and already expensive to do by hand. The best gains usually come from embedding AI into existing products and operations so it drafts, routes, summarizes, or recommends work that people can then approve."
            },
            [PSCustomObject]@{
                Heading = "High-Value Opportunities"
                Body    = "The clearest opportunities are customer support triage, software development assistance, document drafting, analytics, sales operations, and internal knowledge search. Each of these wins is concrete: AI saves time by narrowing choices, producing a first draft, or handling routine exceptions faster than a human can."
            },
            [PSCustomObject]@{
                Heading = "Constraints And Risks"
                Body    = "The main constraints are data quality, privacy, cost control, evaluation discipline, and change management. AI fails when teams measure novelty instead of outcomes, so every use case needs a baseline, a human review path for ambiguous cases, and a clear owner for quality."
            },
            [PSCustomObject]@{
                Heading = "Implementation Priorities"
                Body    = "Start with one workflow that already has a measurable bottleneck, define what success looks like, and run a short pilot with narrow scope. Measure the result with real business metrics such as time saved, faster turnaround, fewer errors, or improved conversion, then expand only after the result is repeatable."
            },
            [PSCustomObject]@{
                Heading = "Conclusion"
                Body    = "For $subject, the practical question in 2026 is not whether AI can help, but where it can remove friction without adding hidden risk. The strongest plan is to target a concrete workflow, keep a human in the loop, and scale only after the gains are proven."
            }
        )
    }

    return @(
        [PSCustomObject]@{ Heading = "Executive Summary"; Body = "This document focuses on $subject and explains the decisions that matter most. It is written to be specific enough to support action rather than staying at the level of a generic overview." },
        [PSCustomObject]@{ Heading = "What It Means"; Body = "$subject affects how teams plan work, allocate attention, and decide what to automate or delegate. The useful question is where the subject changes outcomes, not just where it sounds important." },
        [PSCustomObject]@{ Heading = "Practical Opportunities"; Body = "The best opportunities connected to $subject are the ones that remove friction, reduce delay, or improve quality in a process people already care about. A good document should name those opportunities directly instead of describing them in abstract terms." },
        [PSCustomObject]@{ Heading = "Risks And Constraints"; Body = "Any plan around $subject should be checked against cost, reliability, privacy, and operational complexity. If the subject is handled without measurement or ownership, it usually produces more noise than value." },
        [PSCustomObject]@{ Heading = "Conclusion"; Body = "$subject should be turned into a concrete plan with a clear owner, a measurable outcome, and a small first step. That is what makes the document useful rather than generic." }
    )
}

function ConvertTo-WordXmlText {
    param([string]$Text)
    if ($null -eq $Text) {
        return ""
    }
    return [System.Security.SecurityElement]::Escape($Text)
}

function New-DocxParagraphXml {
    param(
        [string]$Text,
        [bool]$Bold = $false,
        [int]$SizeHalfPoints = 22
    )

    if ([string]::IsNullOrEmpty($Text)) {
        return "<w:p/>"
    }

    $escaped = ConvertTo-WordXmlText -Text $Text
    $runProps = ""
    if ($Bold -or $SizeHalfPoints -ne 22) {
        $boldXml = ""
        if ($Bold) {
            $boldXml = "<w:b/>"
        }
        $runProps = "<w:rPr>$boldXml<w:sz w:val=`"$SizeHalfPoints`"/></w:rPr>"
    }
    return "<w:p><w:r>$runProps<w:t xml:space=`"preserve`">$escaped</w:t></w:r></w:p>"
}

function Add-ZipTextEntry {
    param(
        [System.IO.Compression.ZipArchive]$Archive,
        [string]$Name,
        [string]$Content
    )

    $entry = $Archive.CreateEntry($Name)
    $stream = $entry.Open()
    try {
        $writer = New-Object System.IO.StreamWriter($stream, [System.Text.UTF8Encoding]::new($false))
        try {
            $writer.Write($Content)
        }
        finally {
            $writer.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function New-DocxDocument {
    param(
        [string]$Path,
        [string]$Title,
        [string]$Subtitle,
        [object[]]$Sections
    )

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Force
    }

    $paragraphs = New-Object System.Collections.Generic.List[string]
    [void]$paragraphs.Add((New-DocxParagraphXml -Text $Title -Bold $true -SizeHalfPoints 32))
    [void]$paragraphs.Add((New-DocxParagraphXml -Text $Subtitle -Bold $false -SizeHalfPoints 20))
    [void]$paragraphs.Add((New-DocxParagraphXml -Text ""))

    foreach ($section in $Sections) {
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ([string]$section.Heading) -Bold $true -SizeHalfPoints 26))
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ([string]$section.Body) -Bold $false -SizeHalfPoints 22))
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ""))
    }

    $documentXml = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    $($paragraphs -join "`n    ")
    <w:sectPr>
      <w:pgSz w:w="12240" w:h="15840"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>
"@

    $contentTypesXml = @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
'@

    $relsXml = @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
'@

    $archive = [System.IO.Compression.ZipFile]::Open($Path, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        Add-ZipTextEntry -Archive $archive -Name "[Content_Types].xml" -Content $contentTypesXml
        Add-ZipTextEntry -Archive $archive -Name "_rels/.rels" -Content $relsXml
        Add-ZipTextEntry -Archive $archive -Name "word/document.xml" -Content $documentXml
    }
    finally {
        $archive.Dispose()
    }
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $env:USERPROFILE "Documents\iAgent Documents"
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

$topic = Get-DocTopic -Text $Goal
$title = Get-DocTitle -Text $Goal
$sections = Get-Sections -Text $Goal -Title $title -Topic $topic
$safe = Get-SafeFileName -Text $title
$outPath = Join-Path $OutputDir "$safe.docx"
$subtitle = "Generated by iAgent on $(Get-Date -Format 'yyyy-MM-dd HH:mm')"

try {
    New-DocxDocument -Path $outPath -Title $title -Subtitle $subtitle -Sections $sections

    if ($OpenWhenDone) {
        Start-Process -FilePath $outPath | Out-Null
    }

    Write-Output "created_document=$outPath"
}
catch {
    Write-Error ("Word document creation failed: " + $_.Exception.Message)
    exit 1
}
