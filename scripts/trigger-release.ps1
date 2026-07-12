# trigger-release.ps1
# -----------------------------------------------------------------------------
# v0.7.4+ — One-shot dispatcher for the manual `release.yml` workflow.
#
# Uses the GitHub REST API directly (no `gh` CLI required).  Reads a Personal
# Access Token from $env:GH_TOKEN (or prompts if missing), then runs the
# workflow on the `main` branch with the version passed as an argument.
#
# First-time setup (one-off, ~30s):
#   1. Open https://github.com/settings/tokens/new?scopes=workflow
#      - Note: "release dispatcher"
#      - Expiration: your call (90 days is the max for fine-grained PATs)
#      - Repository access: only `ZhSMM/admin-suite`
#      - Permission: Actions → "Read and write"
#      - Generate and copy the token (starts with `ghp_` or `github_pat_`)
#   2. Either paste it when prompted, or store it once:
#        [Environment]::SetEnvironmentVariable("GH_TOKEN","ghp_...","User")
#      (re-open PowerShell after `SetEnvironmentVariable` so the new value
#      is visible to the current process).
#
# Usage:
#   .\trigger-release.ps1                                  # version from tauri.conf.json
#   .\trigger-release.ps1 -Version v0.7.4                   # override tag
#   .\trigger-release.ps1 -Version v0.7.4 -CreateRelease:$false   # rebuild only
# -----------------------------------------------------------------------------
[CmdletBinding()]
param(
    [string]$Version = "",
    [bool]$CreateRelease = $true,
    [string]$Ref = "main"
)

$ErrorActionPreference = "Stop"

# --- Locate the version in tauri.conf.json when not given ---------------------
if (-not $Version) {
    $conf = Join-Path $PSScriptRoot "src-tauri\tauri.conf.json"
    if (-not (Test-Path $conf)) {
        throw "tauri.conf.json not found at $conf — pass -Version explicitly"
    }
    $v = (Get-Content $conf -Raw | ConvertFrom-Json).package.version
    if (-not $v) { throw "Could not read package.version from $conf" }
    $Version = "v$($v.TrimStart('v'))"
    Write-Host "→ Using version from tauri.conf.json: $Version" -ForegroundColor Cyan
}

# --- Resolve a token ---------------------------------------------------------
if (-not $env:GH_TOKEN) {
    Write-Host "GH_TOKEN is not set." -ForegroundColor Yellow
    Write-Host "  Generate one at: https://github.com/settings/tokens/new?scopes=workflow"
    Write-Host "  Required scope: Actions: write (on the admin-suite repo only)"
    $secure = Read-Host -Prompt "Paste your PAT" -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    $env:GH_TOKEN = [Runtime.InteropServices.Marshal]::PtrToStringAuto($bstr)
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
    if (-not $env:GH_TOKEN) { throw "No token provided." }
}

# --- Build the dispatch payload ---------------------------------------------
$body = @{
    ref     = $Ref
    inputs  = @{
        version        = $Version
        create_release = $CreateRelease
    }
} | ConvertTo-Json -Depth 4 -Compress

# Use the API endpoint, NOT the UI `dispatches` event — they accept the same
# inputs but the REST one gives us the dispatch ID in the response header.
$url = "https://api.github.com/repos/ZhSMM/admin-suite/actions/workflows/release.yml/dispatches"
$hdr = @{
    "Accept"        = "application/vnd.github+json"
    "Authorization" = "Bearer $env:GH_TOKEN"
    "X-GitHub-Api-Version" = "2022-11-28"
    "User-Agent"    = "admin-suite-trigger-release/1.0"
}

Write-Host ""
Write-Host "Dispatching release.yml on $Ref" -ForegroundColor Cyan
Write-Host "  version        = $Version"
Write-Host "  create_release = $CreateRelease"
Write-Host ""

try {
    $resp = Invoke-RestMethod -Method Post -Uri $url -Headers $hdr -Body $body -TimeoutSec 30
    # 204 No Content is the success code; we just won't get a body.
    Write-Host "✓ Workflow dispatch accepted." -ForegroundColor Green
    Write-Host "  Watch the run at: https://github.com/ZhSMM/admin-suite/actions/workflows/release.yml" -ForegroundColor Green
} catch {
    $code = $_.Exception.Response.StatusCode.value__
    $msg  = $_.Exception.Message
    Write-Host "✗ Dispatch failed (HTTP $code): $msg" -ForegroundColor Red
    if ($code -eq 401) {
        Write-Host "  Hint: the token is invalid, expired, or missing the `workflow` scope." -ForegroundColor Yellow
    } elseif ($code -eq 403) {
        Write-Host "  Hint: the token does not have Actions: write on this repo." -ForegroundColor Yellow
    } elseif ($code -eq 422) {
        Write-Host "  Hint: the workflow file may be invalid or inputs don't match the schema." -ForegroundColor Yellow
    }
    exit 1
}
