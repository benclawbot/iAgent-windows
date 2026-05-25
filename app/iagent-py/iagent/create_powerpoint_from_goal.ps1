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
        return "Presentation"
    }
    $clean = $Text -replace '[<>:"/\\|?*]', ""
    $clean = $clean.Trim()
    if ([string]::IsNullOrWhiteSpace($clean)) {
        return "Presentation"
    }
    return $clean
}

function Convert-ToRgbInt {
    param(
        [int]$R,
        [int]$G,
        [int]$B
    )
    return ($R -bor ($G -shl 8) -bor ($B -shl 16))
}

function Get-GoalTitle {
    param([string]$Text)

    $title = "AI Latest Evolutions"
    $titleMatch = [regex]::Match($Text, 'titled\s+"([^"]+)"', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($titleMatch.Success) {
        return $titleMatch.Groups[1].Value.Trim()
    }

    $topicMatch = [regex]::Match($Text, '(?:on|about)\s+([a-z0-9][^.,;\n\r]+)', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($topicMatch.Success) {
        $topic = $topicMatch.Groups[1].Value.Trim()
        if (-not [string]::IsNullOrWhiteSpace($topic)) {
            return ($topic.Substring(0,1).ToUpperInvariant() + $topic.Substring(1))
        }
    }

    return $title
}

function Get-RequestedSlideCount {
    param([string]$Text)

    $match = [regex]::Match($Text, '\b(\d{1,2})\s*(?:slides?|pages?)\b', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if (-not $match.Success) {
        return 7
    }
    $value = [int]$match.Groups[1].Value
    if ($value -lt 4) { return 4 }
    if ($value -gt 16) { return 16 }
    return $value
}

function New-BodyForHeading {
    param(
        [string]$Heading,
        [string]$Topic
    )

    $h = $Heading.Trim()
    $t = $Topic.Trim()
    if ([string]::IsNullOrWhiteSpace($t)) {
        $t = "this topic"
    }
    return @(
        "why this matters for $t"
        "what changed recently in $h"
        "practical takeaway and next step"
    ) -join "`r`n"
}

function Get-DefaultSections {
    param([string]$Topic)

    $topicL = $Topic.ToLowerInvariant()
    if ($topicL -like "*ai*" -or $topicL -like "*artificial intelligence*") {
        return @(
            "Market Snapshot",
            "Foundation Model Advances",
            "Multimodal Breakthroughs",
            "Agentic Workflows",
            "Enterprise Adoption",
            "Risk, Safety, and Governance",
            "Near-Term Outlook"
        )
    }
    return @(
        "Overview",
        "Current Landscape",
        "Key Trends",
        "Opportunities",
        "Risks and Constraints",
        "Recommendations",
        "Next Steps"
    )
}

function Build-Sections {
    param(
        [string]$Text,
        [string]$Topic,
        [int]$TargetContentSlides
    )

    $sections = @()
    $numbered = [regex]::Matches(
        $Text,
        '\d+\)\s*([^:;\n\r]+)(?::\s*([^;\n\r]+))?',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )
    foreach ($m in $numbered) {
        $h = $m.Groups[1].Value.Trim()
        $b = $m.Groups[2].Value.Trim()
        if ([string]::IsNullOrWhiteSpace($h)) {
            continue
        }
        if ([string]::IsNullOrWhiteSpace($b)) {
            $b = New-BodyForHeading -Heading $h -Topic $Topic
        }
        $sections += [PSCustomObject]@{
            Heading = $h
            Body = $b
        }
    }

    if ($sections.Count -eq 0) {
        $defaults = Get-DefaultSections -Topic $Topic
        foreach ($heading in $defaults) {
            $sections += [PSCustomObject]@{
                Heading = $heading
                Body = (New-BodyForHeading -Heading $heading -Topic $Topic)
            }
        }
    }

    while ($sections.Count -lt $TargetContentSlides) {
        $idx = $sections.Count + 1
        $heading = "Key Insight $idx"
        $sections += [PSCustomObject]@{
            Heading = $heading
            Body = (New-BodyForHeading -Heading $heading -Topic $Topic)
        }
    }

    if ($sections.Count -gt $TargetContentSlides) {
        return $sections[0..($TargetContentSlides - 1)]
    }
    return $sections
}

function Apply-ModernSlideStyle {
    param(
        $Slide,
        [bool]$IsTitleSlide
    )

    $Slide.FollowMasterBackground = $false
    $fill = $Slide.Background.Fill
    $fill.Visible = -1
    $fill.Solid()
    if ($IsTitleSlide) {
        $fill.ForeColor.RGB = (Convert-ToRgbInt -R 20 -G 34 -B 64)
    }
    else {
        $fill.ForeColor.RGB = (Convert-ToRgbInt -R 239 -G 244 -B 250)
    }

    if ($Slide.Shapes.Title -ne $null) {
        $titleRange = $Slide.Shapes.Title.TextFrame.TextRange
        $titleRange.Font.Name = "Aptos Display"
        $titleRange.Font.Size = 42
        if ($IsTitleSlide) {
            $titleRange.Font.Color.RGB = (Convert-ToRgbInt -R 255 -G 255 -B 255)
        }
        else {
            $titleRange.Font.Color.RGB = (Convert-ToRgbInt -R 17 -G 37 -B 70)
        }
    }

    try {
        $bodyRange = $Slide.Shapes.Item(2).TextFrame.TextRange
        $bodyRange.Font.Name = "Aptos"
        $bodyRange.Font.Size = 24
        if ($IsTitleSlide) {
            $bodyRange.Font.Color.RGB = (Convert-ToRgbInt -R 220 -G 232 -B 245)
        }
        else {
            $bodyRange.Font.Color.RGB = (Convert-ToRgbInt -R 42 -G 56 -B 77)
        }
    }
    catch {
        # Some templates do not include a body placeholder.
    }
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $env:USERPROFILE "Documents\\iAgent Presentations"
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

$title = Get-GoalTitle -Text $Goal

$subtitle = "Generated by iAgent"
$subtitleMatch = [regex]::Match($Goal, 'subtitle\s+"([^"]+)"', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
if ($subtitleMatch.Success) {
    $subtitle = $subtitleMatch.Groups[1].Value.Trim()
}

$topic = $title
$requestedSlides = Get-RequestedSlideCount -Text $Goal
$contentSlideCount = [Math]::Max(3, $requestedSlides - 1)
$sections = Build-Sections -Text $Goal -Topic $topic -TargetContentSlides $contentSlideCount
$useModernStyle = $Goal.ToLowerInvariant().Contains("modern")

$ppLayoutTitle = 1
$ppLayoutText = 2

$ppt = $null
$presentation = $null

try {
    $ppt = New-Object -ComObject PowerPoint.Application

    $presentation = $ppt.Presentations.Add(0)

    $titleSlide = $presentation.Slides.Add(1, $ppLayoutTitle)
    $titleSlide.Shapes.Title.TextFrame.TextRange.Text = $title
    try {
        $titleSlide.Shapes.Item(2).TextFrame.TextRange.Text = $subtitle
    }
    catch {
        # Some templates may not have subtitle placeholder.
    }
    if ($useModernStyle) {
        Apply-ModernSlideStyle -Slide $titleSlide -IsTitleSlide $true
    }

    foreach ($section in $sections) {
        $idx = $presentation.Slides.Count + 1
        $slide = $presentation.Slides.Add($idx, $ppLayoutText)
        $slide.Shapes.Title.TextFrame.TextRange.Text = $section.Heading
        try {
            $slide.Shapes.Item(2).TextFrame.TextRange.Text = $section.Body
        }
        catch {
            # Some templates may not have body placeholder.
        }
        if ($useModernStyle) {
            Apply-ModernSlideStyle -Slide $slide -IsTitleSlide $false
        }
    }

    $safe = Get-SafeFileName -Text $title
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $outPath = Join-Path $OutputDir "$safe-$stamp.pptx"
    $presentation.SaveAs($outPath)
    $presentation.Close()
    $ppt.Quit()

    if ($OpenWhenDone) {
        if (Get-Command powerpnt.exe -ErrorAction SilentlyContinue) {
            Start-Process -FilePath "powerpnt.exe" -ArgumentList @($outPath) | Out-Null
        }
        else {
            Start-Process -FilePath $outPath | Out-Null
        }
    }

    Write-Output "created_presentation=$outPath"
}
catch {
    Write-Error ("PowerPoint creation failed: " + $_.Exception.Message)
    exit 1
}
finally {
    if ($presentation -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($presentation) } catch {}
    }
    if ($ppt -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($ppt) } catch {}
    }
}
