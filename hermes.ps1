# Hermes Agent Windows Launcher
# Usage: .\hermes.ps1 [command] [args...]
# Example: .\hermes.ps1 chat
#          .\hermes.ps1 dashboard
#          .\hermes.ps1 status

param(
    [Parameter(ValueFromRemainingArguments=$true)]
    [string[]]$Arguments
)

$ScriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$VenvActivate = Join-Path $ScriptPath "venv\Scripts\Activate.ps1"

if (-not (Test-Path $VenvActivate)) {
    Write-Host "❌ Virtual environment not found at: $VenvActivate" -ForegroundColor Red
    Write-Host "Run: bash setup-hermes.sh" -ForegroundColor Yellow
    exit 1
}

# Activate venv
& $VenvActivate

# Run hermes with all arguments
if ($Arguments.Count -eq 0) {
    python (Join-Path $ScriptPath "cli.py")
} else {
    python (Join-Path $ScriptPath "cli.py") @Arguments
}
