<# Run on the designated Windows graphical test machine as a standard user. Does not launch PhotoCraft or Photoshop. #>
param(
  [Parameter(Mandatory=$true)][string]$CandidateDirectory,
  [Parameter(Mandatory=$true)][string]$EvidenceDirectory,
  [Parameter(Mandatory=$true)][string]$PhotoshopBuild,
  [Parameter(Mandatory=$true)][string]$DisplayScalingAndMonitors
)
$ErrorActionPreference = 'Stop'
$identity = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if ($identity.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Use a standard-user graphical test session.' }
$candidate = (Resolve-Path -LiteralPath $CandidateDirectory).Path
if (Test-Path -LiteralPath $EvidenceDirectory) { throw 'Use a new evidence directory for each attempt.' }
New-Item -ItemType Directory -Path $EvidenceDirectory | Out-Null
$session = [ordered]@{
  started_utc = (Get-Date).ToUniversalTime().ToString('o')
  candidate_directory = $candidate
  photoshop_build = $PhotoshopBuild
  display_scaling_and_monitors = $DisplayScalingAndMonitors
  windows = @(Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber,OSArchitecture)
  video = @(Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,CurrentHorizontalResolution,CurrentVerticalResolution)
  files = @(Get-ChildItem -LiteralPath $candidate -File | Where-Object Name -in @('photocraft.exe','photocraft-cli.exe','source-commit.txt','source-tree.txt') | Get-FileHash -Algorithm SHA256 | Select-Object Path,Hash)
  verdict = 'NOT_RUN'
}
$session | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $EvidenceDirectory 'session.json') -Encoding utf8
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'results-template.json') -Destination (Join-Path $EvidenceDirectory 'results.json')
Write-Output 'Session recorded. Populate results.json with actual evidence; no graphical test has been marked PASS.'

