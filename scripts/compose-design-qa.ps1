param(
  [Parameter(Mandatory = $true)]
  [string]$ReferencePath,
  [Parameter(Mandatory = $true)]
  [string]$ImplementationPath,
  [Parameter(Mandatory = $true)]
  [string]$OutputPath
)

Add-Type -AssemblyName System.Drawing

$reference = [System.Drawing.Image]::FromFile([System.IO.Path]::GetFullPath($ReferencePath))
$implementation = [System.Drawing.Image]::FromFile([System.IO.Path]::GetFullPath($ImplementationPath))
$targetHeight = 900
$headerHeight = 52
$gap = 18
$referenceWidth = [int]($reference.Width * $targetHeight / $reference.Height)
$implementationWidth = [int]($implementation.Width * $targetHeight / $implementation.Height)
$canvas = New-Object System.Drawing.Bitmap(($referenceWidth + $implementationWidth + $gap), ($targetHeight + $headerHeight))
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$graphics.Clear([System.Drawing.Color]::FromArgb(15, 22, 36))

$font = New-Object System.Drawing.Font('Segoe UI', 15, [System.Drawing.FontStyle]::Bold)
$brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$graphics.DrawString('VISUAL TARGET', $font, $brush, 18, 14)
$graphics.DrawString('TOKEN MANAGER 0.7.0', $font, $brush, ($referenceWidth + $gap + 18), 14)
$graphics.DrawImage($reference, 0, $headerHeight, $referenceWidth, $targetHeight)
$graphics.DrawImage($implementation, ($referenceWidth + $gap), $headerHeight, $implementationWidth, $targetHeight)

$absoluteOutput = [System.IO.Path]::GetFullPath($OutputPath)
$parent = [System.IO.Path]::GetDirectoryName($absoluteOutput)
if (-not [System.IO.Directory]::Exists($parent)) {
  [System.IO.Directory]::CreateDirectory($parent) | Out-Null
}
$canvas.Save($absoluteOutput, [System.Drawing.Imaging.ImageFormat]::Png)

$brush.Dispose()
$font.Dispose()
$graphics.Dispose()
$canvas.Dispose()
$reference.Dispose()
$implementation.Dispose()

$absoluteOutput
