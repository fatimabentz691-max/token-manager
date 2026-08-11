param(
  [Parameter(Mandatory = $true)]
  [int]$ProcessId,
  [string]$OutputDirectory = '.tmp',
  [ValidateSet('both', 'full', 'half')]
  [string]$Only = 'both'
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class TokenManagerResponsiveQa {
  public delegate bool EnumWindowsProc(IntPtr handle, IntPtr parameter);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr handle, StringBuilder text, int length);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr handle, int command);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr handle, IntPtr after, int x, int y, int width, int height, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out RECT rect);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr handle, IntPtr target, uint flags);
}
'@

function Find-MainWindow([int]$Id) {
  $matches = [System.Collections.Generic.List[object]]::new()
  $callback = [TokenManagerResponsiveQa+EnumWindowsProc]{
    param([IntPtr]$handle, [IntPtr]$parameter)
    [uint32]$owner = 0
    [void][TokenManagerResponsiveQa]::GetWindowThreadProcessId($handle, [ref]$owner)
    if ($owner -eq $Id) {
      $title = [System.Text.StringBuilder]::new(256)
      [void][TokenManagerResponsiveQa]::GetWindowText($handle, $title, 256)
      if ($title.ToString() -eq 'Token Manager') { $matches.Add($handle) }
    }
    return $true
  }
  [void][TokenManagerResponsiveQa]::EnumWindows($callback, [IntPtr]::Zero)
  return $matches | Select-Object -First 1
}

function Save-WindowScreenshot([IntPtr]$Handle, [int]$Width, [int]$Height, [string]$Name) {
  $dpi = [TokenManagerResponsiveQa]::GetDpiForWindow($Handle)
  if ($dpi -le 0) { $dpi = 96 }
  [void][TokenManagerResponsiveQa]::ShowWindow($Handle, 5)
  [void][TokenManagerResponsiveQa]::SetWindowPos($Handle, [IntPtr]::Zero, 24, 24, $Width, $Height, 0x0040)
  [void][TokenManagerResponsiveQa]::SetForegroundWindow($Handle)
  Start-Sleep -Milliseconds 900
  $rect = New-Object TokenManagerResponsiveQa+RECT
  [void][TokenManagerResponsiveQa]::GetWindowRect($Handle, [ref]$rect)
  $actualWidth = $rect.Right - $rect.Left
  $actualHeight = $rect.Bottom - $rect.Top
  $scale = $dpi / 96.0
  $captureWidth = [int][Math]::Round($actualWidth * $scale)
  $captureHeight = [int][Math]::Round($actualHeight * $scale)
  $bitmap = [System.Drawing.Bitmap]::new($captureWidth, $captureHeight)
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $target = $graphics.GetHdc()
    try {
      if (-not [TokenManagerResponsiveQa]::PrintWindow($Handle, $target, 2)) { throw 'PrintWindow failed.' }
    } finally {
      $graphics.ReleaseHdc($target)
    }
    $path = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) "health-native-$Name.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
  [pscustomobject]@{ Name=$Name; Path=$path; WindowWidth=$actualWidth; WindowHeight=$actualHeight; CaptureWidth=$captureWidth; CaptureHeight=$captureHeight; Dpi=$dpi }
}

$handle = Find-MainWindow $ProcessId
if (-not $handle) { throw 'Token Manager main window was not found.' }
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetFullPath($OutputDirectory)) | Out-Null
Start-Sleep -Seconds 2
$captures = @()
if ($Only -in @('both', 'full')) { $captures += Save-WindowScreenshot $handle 1440 900 'full' }
if ($Only -in @('both', 'half')) { $captures += Save-WindowScreenshot $handle 980 760 'half' }
$captures | ConvertTo-Json -Compress
