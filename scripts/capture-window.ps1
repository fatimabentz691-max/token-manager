param(
  [Parameter(Mandatory = $true)]
  [int]$ProcessId,
  [Parameter(Mandatory = $true)]
  [string]$OutputPath
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class TokenManagerCapture {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern uint GetDpiForWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint flags);
}
"@

$process = Get-Process -Id $ProcessId -ErrorAction Stop
$handle = $process.MainWindowHandle
if ($handle -eq [IntPtr]::Zero) {
  throw "The process does not have a visible main window: $ProcessId"
}

$rect = New-Object TokenManagerCapture+RECT
if (-not [TokenManagerCapture]::GetWindowRect($handle, [ref]$rect)) {
  throw "Unable to read the window bounds."
}

$dpi = [TokenManagerCapture]::GetDpiForWindow($handle)
if ($dpi -le 0) { $dpi = 96 }
$scale = $dpi / 96.0
$width = [Math]::Max(1, [int](($rect.Right - $rect.Left) * $scale))
$height = [Math]::Max(1, [int](($rect.Bottom - $rect.Top) * $scale))

$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$hdc = $graphics.GetHdc()
try {
  if (-not [TokenManagerCapture]::PrintWindow($handle, $hdc, 2)) {
    throw "PrintWindow capture failed."
  }
} finally {
  $graphics.ReleaseHdc($hdc)
  $graphics.Dispose()
}

$absoluteOutput = [System.IO.Path]::GetFullPath($OutputPath)
$parent = [System.IO.Path]::GetDirectoryName($absoluteOutput)
if (-not [System.IO.Directory]::Exists($parent)) {
  [System.IO.Directory]::CreateDirectory($parent) | Out-Null
}
$bitmap.Save($absoluteOutput, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()

[PSCustomObject]@{
  Path = $absoluteOutput
  Width = $width
  Height = $height
  Dpi = $dpi
}
