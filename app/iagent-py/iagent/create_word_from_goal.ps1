param(
    [Parameter(Mandatory = $true)]
    [string]$Goal,
    [string]$OutputDir = "",
    [switch]$OpenWhenDone
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

function Normalize-GoalForDocument {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return ""
    }
    $clean = $Text
    $clean = [regex]::Replace($clean, '(?i)\bwith\s+proper\s+formatting\b', '')
    $clean = [regex]::Replace($clean, '(?i)\bsave\s+as\s+["''][^"'']+["'']\s*(?:in\s+the\s+user''s\s+documents\s+folder)?', '')
    $clean = [regex]::Replace($clean, '(?i)\bsave\s+as\s+[A-Za-z0-9_.\-]+\s*(?:in\s+the\s+user''s\s+documents\s+folder)?', '')
    $clean = [regex]::Replace($clean, '(?i)\b(and|then)\s+open\s+(?:it|the\s+result|in\s+word)\b', '')
    $clean = [regex]::Replace($clean, '\s+', ' ')
    $clean = $clean.Trim(" ", ".", ",", ";", ":")
    return $clean
}

function Get-DocTopic {
    param([string]$Text)

    $titledMatch = [regex]::Match($Text, 'titled\s+"([^"]+)"', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($titledMatch.Success) {
        $rawTitle = $titledMatch.Groups[1].Value.Trim()
        if ($rawTitle -like "*:*") {
            $parts = $rawTitle -split ":", 2
            $subject = $parts[0].Trim()
            if (-not [string]::IsNullOrWhiteSpace($subject)) {
                return $subject
            }
        }
        if (-not [string]::IsNullOrWhiteSpace($rawTitle)) {
            return $rawTitle
        }
    }

    $topicMatch = [regex]::Match($Text, '(?:on|about)\s+([a-z0-9][^.,;\n\r]+)', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($topicMatch.Success) {
        $topic = $topicMatch.Groups[1].Value.Trim()
        $topic = [regex]::Replace($topic, '(?i)\bwith\s+proper\s+formatting\b', '').Trim()
        $topic = [regex]::Replace($topic, '(?i)\bsave\s+as\b.*$', '').Trim()
        $topic = [regex]::Replace($topic, '(?i)\b(and|then)\s+open\b.*$', '').Trim()
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

function Get-PageCount {
    param([string]$Text)

    $pageMatch = [regex]::Match($Text, '(\d+)\s*(?:page|pages)\b', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($pageMatch.Success) {
        $pages = [int]$pageMatch.Groups[1].Value
        if ($pages -lt 1) { $pages = 1 }
        if ($pages -gt 25) { $pages = 25 }
        return $pages
    }
    return 3
}

function Get-KeyPhrases {
    param([string]$Text)

    $stop = @("the","and","for","with","from","that","this","your","you","into","about","will","have","has","are","was","were","can","should","would","could","document","create","make","build","draft","write","please","word","doc","docx","page","pages","save","saved","open","titled","formatting","proper","generated","folder","documents")
    $tokens = [regex]::Matches(($Text.ToLowerInvariant()), '[a-z][a-z0-9\-]{2,}') | ForEach-Object { $_.Value }
    $seen = New-Object System.Collections.Generic.HashSet[string]
    $result = New-Object System.Collections.Generic.List[string]
    foreach ($token in $tokens) {
        if ($stop -contains $token) { continue }
        if ($seen.Add($token)) {
            [void]$result.Add($token)
        }
        if ($result.Count -ge 10) { break }
    }
    return $result
}

function Get-ContentStyle {
    param(
        [string]$Text,
        [string]$Subject
    )
    $combined = "$Text $Subject".ToLowerInvariant()
    if ($combined -match '(?i)\b(love|heart|poem|poetry|philosophy|relationship|emotion|meaning|human connection|culture|art|literature|ethics)\b') {
        return "humanities"
    }
    if ($combined -match '(?i)\b(architecture|system|implementation|api|engineering|software|technical|model|infrastructure)\b') {
        return "technical"
    }
    return "business"
}

function Expand-SectionBody {
    param(
        [string]$Body,
        [string]$Heading,
        [string]$Subject,
        [string]$Goal,
        [string[]]$KeyPhrases,
        [int]$PageCount,
        [string]$Style = "business"
    )

    $paragraphs = New-Object System.Collections.Generic.List[string]
    [void]$paragraphs.Add($Body)

    if ($KeyPhrases.Count -gt 0) {
        [void]$paragraphs.Add("Key focus terms from your request: " + ($KeyPhrases -join ", ") + ".")
    }

    if ($Style -eq "humanities") {
        [void]$paragraphs.Add("This section explores $Heading through the lens of $Subject, combining emotional meaning, lived experience, and reflective interpretation.")
        [void]$paragraphs.Add("It highlights how people understand and express this theme in intimate relationships, family bonds, friendship, and broader social life.")
        if ($PageCount -ge 6) {
            [void]$paragraphs.Add("The discussion also considers historical and cultural context, showing how beliefs about $Subject shift across eras while still preserving enduring human questions.")
        }
        if ($PageCount -ge 10) {
            [void]$paragraphs.Add("To deepen the analysis, this chapter connects theory with concrete examples and contrasts competing viewpoints so the reader can form a nuanced, well-supported perspective.")
        }
    } elseif ($Style -eq "technical") {
        [void]$paragraphs.Add("For $Heading, this section links recommendations to $Subject with clear assumptions, implementation options, and practical tradeoffs.")
        [void]$paragraphs.Add("It emphasizes system behavior, constraints, and measurable outcomes so decisions remain testable and actionable.")
        if ($PageCount -ge 6) {
            [void]$paragraphs.Add("Additional depth covers integration patterns, operational risk, and validation criteria to support reliable execution.")
        }
    } else {
        [void]$paragraphs.Add("For $Heading, this document ties recommendations directly to $Subject and emphasizes practical decisions, ownership, and measurable outputs.")
        [void]$paragraphs.Add("Execution guidance: define scope boundaries, validate assumptions, record decisions, and keep a review loop so each iteration improves quality and relevance.")
    }

    if ($PageCount -ge 5) {
        if ($Style -eq "humanities") {
            [void]$paragraphs.Add("Detailed reflections examine tensions, contradictions, and unanswered questions, allowing the topic to be treated with both intellectual rigor and emotional depth.")
        } else {
            [void]$paragraphs.Add("Detailed considerations: include stakeholders, dependencies, milestones, acceptance criteria, and delivery risks. Use explicit examples grounded in the request so the section remains actionable.")
            [void]$paragraphs.Add("Operational follow-through: establish timeline checkpoints, assign accountable owners, and track quality with objective metrics such as completion rate, review turnaround time, and output accuracy.")
        }
    }

    return ($paragraphs -join " ")
}

function Get-Sections {
    param(
        [string]$Text,
        [string]$Title,
        [string]$Topic,
        [int]$PageCount,
        [string]$Style = "business"
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

    if ($sections.Count -eq 0) {
        if ($Style -eq "humanities" -and $subject -match '(?i)\b(love|heart|relationship|connection)\b') {
            $sections = @(
            [PSCustomObject]@{ Heading = "Introduction: Why Love Matters"; Body = "Love is one of the most powerful human experiences, shaping identity, values, and the way people build meaning across a lifetime." },
            [PSCustomObject]@{ Heading = "Biological Foundations"; Body = "Love has a biological layer involving attachment, reward, and stress regulation systems, showing how emotion and physiology are deeply connected." },
            [PSCustomObject]@{ Heading = "Psychological Dimensions"; Body = "Psychology frames love through attachment patterns, trust, vulnerability, and emotional regulation, offering tools to understand healthy and unhealthy dynamics." },
            [PSCustomObject]@{ Heading = "Philosophical Perspectives"; Body = "Philosophical traditions ask whether love is a feeling, a commitment, a virtue, or a moral practice, and what obligations emerge from truly loving another person." },
            [PSCustomObject]@{ Heading = "Love In Literature And Art"; Body = "Stories, poems, music, and visual art reveal how love is experienced in longing, devotion, grief, reconciliation, and transformation." },
            [PSCustomObject]@{ Heading = "Cultural Variations"; Body = "Different cultures define love through distinct rituals, expectations, and social structures, showing both diversity and shared human patterns." },
            [PSCustomObject]@{ Heading = "Love Across The Lifespan"; Body = "Love evolves from adolescence to old age, changing in expression, priorities, and emotional texture as people grow through life stages." },
            [PSCustomObject]@{ Heading = "Conflict, Repair, And Forgiveness"; Body = "Sustained love depends on communication, conflict navigation, and repair after rupture rather than the absence of difficulty." },
            [PSCustomObject]@{ Heading = "Love In The Digital Era"; Body = "Technology expands connection but can also distort intimacy through speed, performance pressure, and fragmented attention." },
            [PSCustomObject]@{ Heading = "Ethics Of Love"; Body = "Ethical love balances care, freedom, respect, and responsibility, avoiding possession while protecting commitment." },
            [PSCustomObject]@{ Heading = "Conclusion"; Body = "Love is not only an emotion but a practice that combines intention, empathy, and courage in everyday choices." }
        )
        } elseif ($subject -match '(?i)\b(ai|artificial intelligence|machine learning|large language model|llm|agent)\b') {
            $sections = @(
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
        } else {
            $sections = @(
        [PSCustomObject]@{ Heading = "Executive Summary"; Body = "This document focuses on $subject and explains the decisions that matter most. It is written to be specific enough to support action rather than staying at the level of a generic overview." },
        [PSCustomObject]@{ Heading = "What It Means"; Body = "$subject affects how teams plan work, allocate attention, and decide what to automate or delegate. The useful question is where the subject changes outcomes, not just where it sounds important." },
        [PSCustomObject]@{ Heading = "Practical Opportunities"; Body = "The best opportunities connected to $subject are the ones that remove friction, reduce delay, or improve quality in a process people already care about. A good document should name those opportunities directly instead of describing them in abstract terms." },
        [PSCustomObject]@{ Heading = "Risks And Constraints"; Body = "Any plan around $subject should be checked against cost, reliability, privacy, and operational complexity. If the subject is handled without measurement or ownership, it usually produces more noise than value." },
        [PSCustomObject]@{ Heading = "Conclusion"; Body = "$subject should be turned into a concrete plan with a clear owner, a measurable outcome, and a small first step. That is what makes the document useful rather than generic." }
    )
        }
    }

    $extraHeadings = @(
        "Objectives",
        "Scope And Assumptions",
        "Implementation Plan",
        "Quality And Validation",
        "Risks And Mitigations",
        "Timeline And Milestones",
        "Success Metrics",
        "Next Steps"
    )
    if ($Style -eq "humanities") {
        $extraHeadings = @(
            "Memory, Desire, And Longing",
            "Love And Identity",
            "Care, Responsibility, And Commitment",
            "Love, Loss, And Resilience",
            "Community, Friendship, And Belonging",
            "Rituals And Symbolism",
            "Language Of Affection",
            "Future Questions"
        )
    }

    $minSections = [Math]::Max(5, $PageCount)
    $existing = @($sections | ForEach-Object { $_.Heading.ToLowerInvariant() })
    foreach ($heading in $extraHeadings) {
        if ($sections.Count -ge $minSections) { break }
        if ($existing -contains $heading.ToLowerInvariant()) { continue }
        $sections += [PSCustomObject]@{
            Heading = $heading
            Body    = "This section details $heading for $subject with concrete guidance derived from the request."
        }
    }

    $phrases = @(Get-KeyPhrases -Text $Text)
    for ($i = 0; $i -lt $sections.Count; $i++) {
        $sections[$i].Body = Expand-SectionBody -Body ([string]$sections[$i].Body) -Heading ([string]$sections[$i].Heading) -Subject $subject -Goal $Text -KeyPhrases $phrases -PageCount $PageCount -Style $Style
    }
    if ($sections.Count -gt $PageCount) {
        $sections = @($sections | Select-Object -First $PageCount)
    }
    return $sections
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
        [int]$SizeHalfPoints = 22,
        [string]$Align = "left",
        [string]$ColorHex = "",
        [int]$SpacingBefore = 0,
        [int]$SpacingAfter = 140,
        [int]$LineSpacing = 340
    )

    if ([string]::IsNullOrEmpty($Text)) {
        return "<w:p/>"
    }

    $escaped = ConvertTo-WordXmlText -Text $Text
    $alignVal = "left"
    if ($Align -in @("left", "center", "right", "both", "justify")) {
        if ($Align -eq "justify") {
            $alignVal = "both"
        } else {
            $alignVal = $Align
        }
    }
    $pPr = "<w:pPr><w:jc w:val=`"$alignVal`"/><w:spacing w:before=`"$SpacingBefore`" w:after=`"$SpacingAfter`" w:line=`"$LineSpacing`" w:lineRule=`"auto`"/></w:pPr>"
    $runProps = ""
    if ($Bold -or $SizeHalfPoints -ne 22 -or -not [string]::IsNullOrWhiteSpace($ColorHex)) {
        $boldXml = ""
        if ($Bold) {
            $boldXml = "<w:b/>"
        }
        $colorXml = ""
        if (-not [string]::IsNullOrWhiteSpace($ColorHex)) {
            $colorXml = "<w:color w:val=`"$ColorHex`"/>"
        }
        $runProps = "<w:rPr>$boldXml$colorXml<w:sz w:val=`"$SizeHalfPoints`"/></w:rPr>"
    }
    return "<w:p>$pPr<w:r>$runProps<w:t xml:space=`"preserve`">$escaped</w:t></w:r></w:p>"
}

function New-DocxPageBreakXml {
    return "<w:p><w:r><w:br w:type=`"page`"/></w:r></w:p>"
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
        [object[]]$Sections,
        [int]$PageCount = 3
    )

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Force
    }

    $paragraphs = New-Object System.Collections.Generic.List[string]
    [void]$paragraphs.Add((New-DocxParagraphXml -Text $Title -Bold $true -SizeHalfPoints 40 -Align "center" -ColorHex "1F4E79" -SpacingBefore 120 -SpacingAfter 280 -LineSpacing 420))
    [void]$paragraphs.Add((New-DocxParagraphXml -Text ""))

    for ($index = 0; $index -lt $Sections.Count; $index++) {
        $section = $Sections[$index]
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ([string]$section.Heading) -Bold $true -SizeHalfPoints 28 -Align "left" -ColorHex "2E5E88" -SpacingBefore 120 -SpacingAfter 120 -LineSpacing 360))
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ([string]$section.Body) -Bold $false -SizeHalfPoints 22 -Align "justify" -ColorHex "202A33" -SpacingBefore 0 -SpacingAfter 180 -LineSpacing 360))
        [void]$paragraphs.Add((New-DocxParagraphXml -Text ""))
        if ($index -lt ($PageCount - 1)) {
            [void]$paragraphs.Add((New-DocxPageBreakXml))
        }
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

$contentGoal = Normalize-GoalForDocument -Text $Goal
if ([string]::IsNullOrWhiteSpace($contentGoal)) {
    $contentGoal = $Goal
}
$topic = Get-DocTopic -Text $contentGoal
$title = Get-DocTitle -Text $contentGoal
$pageCount = Get-PageCount -Text $Goal
$style = Get-ContentStyle -Text $contentGoal -Subject $topic
$sections = Get-Sections -Text $contentGoal -Title $title -Topic $topic -PageCount $pageCount -Style $style
$safe = Get-SafeFileName -Text $title
$outPath = Join-Path $OutputDir "$safe.docx"

try {
    New-DocxDocument -Path $outPath -Title $title -Sections $sections -PageCount $pageCount

    if ($OpenWhenDone) {
        try {
            Start-Process -FilePath $outPath | Out-Null
        } catch {
            # Non-fatal in headless/background launch contexts.
            Write-Warning ("Document created but auto-open failed: " + $_.Exception.Message)
        }
    }

    Write-Output "created_document=$outPath"
}
catch {
    Write-Error ("Word document creation failed: " + $_.Exception.Message)
    exit 1
}
