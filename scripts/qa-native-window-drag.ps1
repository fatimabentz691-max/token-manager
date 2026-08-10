param(
  [Parameter(Mandatory = $true)]
  [int]$ProcessId
)

$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class TokenManagerWindowQa {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool GetCursorPos(out POINT point);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

    [DllImport("gdi32.dll")]
    public static extern IntPtr CreateRectRgn(int left, int top, int right, int bottom);

    [DllImport("user32.dll")]
    public static extern int GetWindowRgn(IntPtr hWnd, IntPtr region);

    [DllImport("gdi32.dll")]
    public static extern bool PtInRegion(IntPtr region, int x, int y);

    [DllImport("gdi32.dll")]
    public static extern bool DeleteObject(IntPtr value);
}
'@

function Get-VisibleProcessWindow([int]$Id) {
  $matches = [System.Collections.Generic.List[object]]::new()
  $callback = [TokenManagerWindowQa+EnumWindowsProc]{
    param([IntPtr]$handle, [IntPtr]$parameter)
    [uint32]$owner = 0
    [void][TokenManagerWindowQa]::GetWindowThreadProcessId($handle, [ref]$owner)
    if ($owner -eq $Id -and [TokenManagerWindowQa]::IsWindowVisible($handle)) {
      $rect = New-Object TokenManagerWindowQa+RECT
      if ([TokenManagerWindowQa]::GetWindowRect($handle, [ref]$rect)) {
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top
        if ($width -gt 250 -and $height -gt 90) {
          $matches.Add([pscustomobject]@{ Handle = $handle; Rect = $rect; Area = $width * $height })
        }
      }
    }
    return $true
  }
  [void][TokenManagerWindowQa]::EnumWindows($callback, [IntPtr]::Zero)
  return $matches | Sort-Object Area | Select-Object -First 1
}

$window = Get-VisibleProcessWindow $ProcessId
if (-not $window) { throw 'Visible floating window was not found.' }

$before = $window.Rect
$width = $before.Right - $before.Left
$height = $before.Bottom - $before.Top
$cursor = New-Object TokenManagerWindowQa+POINT
[void][TokenManagerWindowQa]::GetCursorPos([ref]$cursor)
$region = [TokenManagerWindowQa]::CreateRectRgn(0, 0, 0, 0)
if ($region -eq [IntPtr]::Zero) { throw 'Could not allocate native window region.' }
$regionType = [TokenManagerWindowQa]::GetWindowRgn($window.Handle, $region)
$cornerIsCut = $regionType -ne 0 -and -not [TokenManagerWindowQa]::PtInRegion($region, 0, 0)
$oppositeCornerIsCut = $regionType -ne 0 -and -not [TokenManagerWindowQa]::PtInRegion($region, $width - 1, $height - 1)
$centerIsVisible = $regionType -ne 0 -and [TokenManagerWindowQa]::PtInRegion($region, [Math]::Floor($width / 2), [Math]::Floor($height / 2))

try {
  # Drag the empty center of the brand header and avoid the action buttons.
  $startX = $before.Left + [Math]::Round($width * 0.52)
  $startY = $before.Top + [Math]::Min([Math]::Round($height * 0.20), 54)
  [void][TokenManagerWindowQa]::SetCursorPos($startX, $startY)
  Start-Sleep -Milliseconds 120
  [TokenManagerWindowQa]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
  for ($step = 1; $step -le 8; $step++) {
    [void][TokenManagerWindowQa]::SetCursorPos($startX + 10 * $step, $startY + 6 * $step)
    Start-Sleep -Milliseconds 24
  }
  [TokenManagerWindowQa]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 320

  $afterWindow = Get-VisibleProcessWindow $ProcessId
  if (-not $afterWindow) { throw 'Floating window disappeared after drag.' }
  $after = $afterWindow.Rect
  $moved = $before.Left -ne $after.Left -or $before.Top -ne $after.Top
  [pscustomobject]@{
    beforeX = $before.Left
    beforeY = $before.Top
    afterX = $after.Left
    afterY = $after.Top
    width = $width
    height = $height
    moved = $moved
    nativeRegion = $regionType
    cornerIsCut = $cornerIsCut
    oppositeCornerIsCut = $oppositeCornerIsCut
    centerIsVisible = $centerIsVisible
  } | ConvertTo-Json -Compress
  if (-not $moved) { throw 'Window coordinates did not change after real pointer drag.' }
  if (-not ($cornerIsCut -and $oppositeCornerIsCut -and $centerIsVisible)) { throw 'Native rounded window region did not clip all corners cleanly.' }
}
finally {
  if ($region -ne [IntPtr]::Zero) { [void][TokenManagerWindowQa]::DeleteObject($region) }
  [void][TokenManagerWindowQa]::SetCursorPos($cursor.X, $cursor.Y)
}
