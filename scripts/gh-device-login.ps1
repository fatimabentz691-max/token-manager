$ErrorActionPreference = 'Continue'
$workspace = Split-Path -Parent $PSScriptRoot
$configDirectory = Join-Path $workspace '.gh-cli'
$outputPath = Join-Path $configDirectory 'device-login.out'
$errorPath = Join-Path $configDirectory 'device-login.err'

$env:GH_CONFIG_DIR = $configDirectory
$env:GH_PROMPT_DISABLED = '1'

$command = '"C:\Program Files\GitHub CLI\gh.exe" auth login --hostname github.com --git-protocol https --web --skip-ssh-key 1>"' + $outputPath + '" 2>"' + $errorPath + '"'
& "$env:SystemRoot\System32\cmd.exe" /d /s /c $command
