param([Parameter(Mandatory=$true)][string]$LaunchFile)
$ErrorActionPreference = 'Stop'
function ConvertTo-CapyNativeArgument([string]$Value) {
    $capyQuoted = [System.Text.StringBuilder]::new()
    [void]$capyQuoted.Append('"')
    $capySlashes = 0
    foreach ($capyCharacter in $Value.ToCharArray()) {
        if ($capyCharacter -eq '\') {
            $capySlashes++
            continue
        }
        if ($capyCharacter -eq '"') {
            [void]$capyQuoted.Append('\', 2 * $capySlashes + 1)
        } elseif ($capySlashes -gt 0) {
            [void]$capyQuoted.Append('\', $capySlashes)
        }
        [void]$capyQuoted.Append($capyCharacter)
        $capySlashes = 0
    }
    [void]$capyQuoted.Append('\', 2 * $capySlashes)
    [void]$capyQuoted.Append('"')
    return $capyQuoted.ToString()
}
try {
    $capyLaunch = Get-Content -Encoding UTF8 -Raw -LiteralPath $LaunchFile | ConvertFrom-Json
    Remove-Item -LiteralPath $LaunchFile
    Set-Location -LiteralPath $capyLaunch.cwd
    foreach ($capyVariable in $capyLaunch.removeEnv) {
        [Environment]::SetEnvironmentVariable($capyVariable, $null, 'Process')
    }
    [Environment]::SetEnvironmentVariable('CLAUDE_CONFIG_DIR', $capyLaunch.configDir, 'Process')
    $capyStart = [System.Diagnostics.ProcessStartInfo]::new()
    $capyStart.FileName = $capyLaunch.executable
    $capyStart.WorkingDirectory = $capyLaunch.cwd
    $capyStart.UseShellExecute = $false
    $capyStart.Arguments = (@($capyLaunch.arguments | ForEach-Object { ConvertTo-CapyNativeArgument $_ }) -join ' ')
    $capyProcess = [System.Diagnostics.Process]::Start($capyStart)
    try { $capyProcess.WaitForExit() } finally { $capyProcess.Dispose() }
} catch {
    Write-Host ('Capy: ' + $_.Exception.Message) -ForegroundColor Red
}
