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
        return "Workbook"
    }
    $clean = $Text -replace '[<>:"/\\|?*]', ""
    $clean = $clean.Trim()
    if ([string]::IsNullOrWhiteSpace($clean)) {
        return "Workbook"
    }
    return $clean
}

function Get-WorkbookTitle {
    param([string]$Text)
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
    return "Operations Dashboard"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $env:USERPROFILE "Documents\\iAgent Workbooks"
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

$title = Get-WorkbookTitle -Text $Goal
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$safe = Get-SafeFileName -Text $title
$outPath = Join-Path $OutputDir "$safe-$timestamp.xlsx"

$excel = $null
$workbook = $null
$dataSheet = $null
$summarySheet = $null

try {
    $excel = New-Object -ComObject Excel.Application
    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $workbook = $excel.Workbooks.Add()
    $dataSheet = $workbook.Worksheets.Item(1)
    $dataSheet.Name = "Data"

    $headers = @("Category", "Metric", "Q1", "Q2", "Q3", "Q4", "Annual Total")
    for ($i = 0; $i -lt $headers.Count; $i++) {
        $dataSheet.Cells.Item(1, $i + 1).Value2 = $headers[$i]
    }

    $rows = @(
        @("Models", "Quality Index", 74, 79, 83, 88),
        @("Models", "Latency Score", 61, 67, 72, 78),
        @("Products", "Feature Adoption", 48, 56, 64, 73),
        @("Operations", "Automation Rate", 31, 40, 52, 61),
        @("Operations", "Support Deflection", 22, 28, 35, 44),
        @("Governance", "Compliance Coverage", 38, 46, 55, 66)
    )

    $rowIdx = 2
    foreach ($row in $rows) {
        $dataSheet.Cells.Item($rowIdx, 1).Value2 = $row[0]
        $dataSheet.Cells.Item($rowIdx, 2).Value2 = $row[1]
        $dataSheet.Cells.Item($rowIdx, 3).Value2 = $row[2]
        $dataSheet.Cells.Item($rowIdx, 4).Value2 = $row[3]
        $dataSheet.Cells.Item($rowIdx, 5).Value2 = $row[4]
        $dataSheet.Cells.Item($rowIdx, 6).Value2 = $row[5]
        $dataSheet.Cells.Item($rowIdx, 7).Formula = "=SUM(C$rowIdx:F$rowIdx)"
        $rowIdx++
    }

    $lastDataRow = $rowIdx - 1

    $headerRange = $dataSheet.Range("A1:G1")
    $headerRange.Font.Bold = $true
    $headerRange.Interior.Color = 15395562
    $dataSheet.Range("A1:G$lastDataRow").Columns.AutoFit() | Out-Null

    $summarySheet = $workbook.Worksheets.Add()
    $summarySheet.Name = "Summary"
    $summarySheet.Cells.Item(1, 1).Value2 = "Metric"
    $summarySheet.Cells.Item(1, 2).Value2 = "Annual Total"
    $summarySheet.Cells.Item(1, 1).Font.Bold = $true
    $summarySheet.Cells.Item(1, 2).Font.Bold = $true

    $summaryRow = 2
    for ($r = 2; $r -le $lastDataRow; $r++) {
        $summarySheet.Cells.Item($summaryRow, 1).Formula = "=Data!B$r"
        $summarySheet.Cells.Item($summaryRow, 2).Formula = "=Data!G$r"
        $summaryRow++
    }
    $lastSummaryRow = $summaryRow - 1
    $summarySheet.Range("A1:B$lastSummaryRow").Columns.AutoFit() | Out-Null

    $chartShape = $summarySheet.Shapes.AddChart2(240, 51, 300, 60, 800, 420)
    $chart = $chartShape.Chart
    $chart.SetSourceData($summarySheet.Range("A1:B$lastSummaryRow"))
    $chart.HasTitle = $true
    $chart.ChartTitle.Text = "$title - Annual Totals"

    $xlOpenXMLWorkbook = 51
    $workbook.SaveAs($outPath, $xlOpenXMLWorkbook)
    $workbook.Close($false)
    $excel.Quit()

    if ($OpenWhenDone) {
        if (Get-Command excel.exe -ErrorAction SilentlyContinue) {
            Start-Process -FilePath "excel.exe" -ArgumentList @($outPath) | Out-Null
        }
        else {
            Start-Process -FilePath $outPath | Out-Null
        }
    }

    Write-Output "created_workbook=$outPath"
}
catch {
    Write-Error ("Excel workbook creation failed: " + $_.Exception.Message)
    exit 1
}
finally {
    if ($summarySheet -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($summarySheet) } catch {}
    }
    if ($dataSheet -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($dataSheet) } catch {}
    }
    if ($workbook -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($workbook) } catch {}
    }
    if ($excel -ne $null) {
        try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($excel) } catch {}
    }
}
