# End-to-end navigation smoke test for the Varethic shell.
# Types "example.com" into the omnibox, presses Enter, and waits for the
# content webview to navigate away from Google/about:blank to Example Domain.
#
# Usage: powershell -ExecutionPolicy Bypass -File tests\ui_navigate.ps1
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
Add-Type -AssemblyName System.Windows.Forms

$proc = Start-Process -FilePath $exePath -PassThru

function Find-ByAutoId($root, $autoId) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $autoId)
    $els = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    if ($els.Count -gt 0) { return $els[0] } else { return $null }
}

function Any-WebElementContains($root, $needle) {
    $trueCond = [System.Windows.Automation.Condition]::TrueCondition
    $els = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $trueCond)
    foreach ($el in $els) {
        $n = $el.Current.Name
        if ($n -and $n.Contains($needle)) { return $true }
    }
    return $false
}

try {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $window = $null
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 300
        if ($proc.HasExited) { Write-Output "FAIL: exited early $($proc.ExitCode)"; exit 1 }
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        $cond = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $proc.Id)
        $windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
        if ($windows.Count -gt 0) { $window = $windows[0]; break }
    }
    if (-not $window) { Write-Output "FAIL: window not found"; exit 1 }
    Write-Output "PASS: window automation element acquired"

    $omnibox = $null
    $omniDeadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $omniDeadline -and -not $omnibox) {
        Start-Sleep -Milliseconds 300
        $omnibox = Find-ByAutoId $window "omnibox"
    }
    if (-not $omnibox) { Write-Output "FAIL: omnibox not found in UIA tree"; exit 1 }
    Write-Output "PASS: omnibox acquired"

    $omnibox.SetFocus()
    $val = $null
    if ($omnibox.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$val)) {
        $val.SetValue("example.com")
        Write-Output "PASS: omnibox value set to 'example.com'"
    } else {
        Write-Output "FAIL: omnibox does not support ValuePattern"
        exit 1
    }

    Start-Sleep -Milliseconds 200
    $omnibox.SetFocus()
    Start-Sleep -Milliseconds 200
    $sig = @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
'@
    $user32 = Add-Type -MemberDefinition $sig -Name KeySender -Namespace Win32 -PassThru
    [void]$user32::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 300
    $user32::keybd_event(0x0D, 0, 0, [UIntPtr]::Zero)  # VK_RETURN down
    $user32::keybd_event(0x0D, 0, 2, [UIntPtr]::Zero)  # VK_RETURN up
    Write-Output "INFO: Enter sent to focused window"

    $deadline2 = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline2) {
        Start-Sleep -Milliseconds 500
        if ($proc.HasExited) { Write-Output "FAIL: browser crashed during navigation (code $($proc.ExitCode))"; exit 1 }
        if (Any-WebElementContains $window "Example Domain") {
            Write-Output "PASS: content navigated to Example Domain"
            exit 0
        }
    }
    Write-Output "FAIL: Example Domain did not appear within $TimeoutSeconds s"
    exit 1
} finally {
    if (-not $proc.HasExited) { $proc.Kill(); $proc.WaitForExit(2000) | Out-Null }
}
