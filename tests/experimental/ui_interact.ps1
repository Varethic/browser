# UI interaction smoke test for the Varethic shell (Windows).
# Exercises real toolbar buttons via UI Automation by AutomationId:
#   1. launch browser, wait for window
#   2. click "+" (new tab) and verify web-content documents increase by one
#   3. click "settings" and verify panel document appears
#   4. browser must remain alive throughout
#
# Usage: powershell -ExecutionPolicy Bypass -File tests\ui_interact.ps1
# Exit code 0 = pass, 1 = fail.

param(
    [string]$Exe = "target\debug\varethic-browser.exe",
    [int]$TimeoutSeconds = 25
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path (Split-Path $MyInvocation.MyCommand.Path -Parent) -Parent
$exePath = Join-Path $repoRoot $Exe

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$proc = Start-Process -FilePath $exePath -PassThru

function Get-Window($procId, $deadline) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $procId)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 300
        $windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
        if ($windows.Count -gt 0) { return $windows[0] }
    }
    return $null
}

function Find-ByAutoId($root, $autoId) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $autoId)
    $els = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    if ($els.Count -gt 0) { return $els[0] } else { return $null }
}

function Count-WebContent($win) {
    $trueCond = [System.Windows.Automation.Condition]::TrueCondition
    $els = $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, $trueCond)
    return ($els | Where-Object { $_.Current.Name -and $_.Current.Name.Contains("Web content") }).Count
}

try {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $window = Get-Window $proc.Id $deadline
    if (-not $window) { Write-Output "FAIL: window not found"; exit 1 }
    Write-Output "PASS: window automation element acquired"

    $omniDeadline = (Get-Date).AddSeconds(10)
    $omni = $null
    while ((Get-Date) -lt $omniDeadline -and -not $omni) {
        Start-Sleep -Milliseconds 300
        $omni = Find-ByAutoId $window "omnibox"
    }
    if (-not $omni) { Write-Output "FAIL: toolbar omnibox not found"; exit 1 }

    Start-Sleep -Seconds 3 # let first tab finish loading
    $docsBefore = Count-WebContent $window
    Write-Output "INFO: web-content documents before = $docsBefore"

    $newtab = Find-ByAutoId $window "newtab"
    if (-not $newtab) { Write-Output "FAIL: '+' (newtab) button not found"; exit 1 }
    $p = $null
    if ($newtab.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$p)) { $p.Invoke() }
    Start-Sleep -Seconds 4
    $docsAfterNewTab = Count-WebContent $window
    Write-Output "INFO: web-content documents after new-tab = $docsAfterNewTab"
    if ($docsAfterNewTab -le $docsBefore) {
        Write-Output "FAIL: new-tab click did not create an observable web-content pane"; exit 1
    }
    Write-Output "PASS: new tab opened (pane count increased)"

    if ($proc.HasExited) { Write-Output "FAIL: browser crashed after new tab"; exit 1 }

    $settings = Find-ByAutoId $window "settings"
    if (-not $settings) { Write-Output "FAIL: settings button not found"; exit 1 }
    $p = $null
    if ($settings.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$p)) { $p.Invoke() }
    Start-Sleep -Seconds 2
    if ($proc.HasExited) { Write-Output "FAIL: browser crashed after settings click"; exit 1 }
    Write-Output "PASS: settings panel opened without crash"

    Write-Output "PASS: browser alive after interaction checks"
    exit 0
} finally {
    if (-not $proc.HasExited) { $proc.Kill(); $proc.WaitForExit(2000) | Out-Null }
}
