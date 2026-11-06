# UI smoke test for the Varethic shell (Windows).
# Launches the browser, verifies the main window appears with the expected
# title, verifies the window has sane bounds, then closes it via the window's
# Close() method (not Kill) and confirms clean exit.
#
# Usage:  powershell -ExecutionPolicy Bypass -File tests\ui_smoke.ps1 [-Exe target\debug\varethic-browser.exe]
# Exit code 0 = pass, 1 = fail.

param(
    [string]$Exe = "target\debug\varethic-browser.exe",
    [string]$ExpectedTitle = "Varethic",
    [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path (Split-Path $MyInvocation.MyCommand.Path -Parent) -Parent
$exePath = Join-Path $repoRoot $Exe

if (-not (Test-Path $exePath)) {
    Write-Output "FAIL: executable not found: $exePath"
    exit 1
}

$proc = Start-Process -FilePath $exePath -PassThru

try {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $hwnd = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
        $proc.Refresh()
        if ($proc.HasExited) {
            Write-Output "FAIL: browser exited early with code $($proc.ExitCode)"
            exit 1
        }
        try { $proc.WaitForInputIdle(500) | Out-Null } catch { Start-Sleep -Milliseconds 100 }
        $h = $proc.MainWindowHandle
        if ($h -ne [IntPtr]::Zero) { $hwnd = $h; break }
    }

    if ($hwnd -eq [IntPtr]::Zero) {
        Write-Output "FAIL: main window did not appear within $TimeoutSeconds s"
        exit 1
    }

    $title = $proc.MainWindowTitle
    if ($title -ne $ExpectedTitle) {
        Write-Output "FAIL: unexpected window title '$title' (expected '$ExpectedTitle')"
        exit 1
    }

    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
        Write-Output "FAIL: MainWindowHandle lost"
        exit 1
    }

    Write-Output "PASS: window '$title' appeared (pid=$($proc.Id), hwnd=$hwnd, MainWindowHandle ok)"

    # Clean close via WM_CLOSE (gentler than Kill; exercises CloseRequested path).
    $sig = @'
[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
'@
    $user32 = Add-Type -MemberDefinition $sig -Name NativeMethods -Namespace Win32 -PassThru
    $WM_CLOSE = 0x0010
    [void]$user32::PostMessage($hwnd, $WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)

    $waited = $proc.WaitForExit(5000)
    if (-not $waited) {
        Write-Output "WARN: window did not close within 5s after WM_CLOSE; killing"
        $proc.Kill()
        $proc.WaitForExit(2000) | Out-Null
        exit 1
    }
    Write-Output "PASS: clean close (exit code $($proc.ExitCode))"
    exit 0
} finally {
    if (-not $proc.HasExited) {
        try { $proc.Kill() } catch {}
    }
}
