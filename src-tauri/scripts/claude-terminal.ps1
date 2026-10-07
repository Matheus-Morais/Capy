param([Parameter(Mandatory=$true)][string]$LaunchFile)
$ErrorActionPreference = 'Stop'
try {
    $capyLaunch = Get-Content -Raw -LiteralPath $LaunchFile | ConvertFrom-Json
    Remove-Item -LiteralPath $LaunchFile
    Set-Location -LiteralPath $capyLaunch.cwd
    foreach ($capyVariable in $capyLaunch.removeEnv) {
        [Environment]::SetEnvironmentVariable($capyVariable, $null, 'Process')
    }
    [Environment]::SetEnvironmentVariable('CLAUDE_CONFIG_DIR', $capyLaunch.configDir, 'Process')
    $capyArguments = @($capyLaunch.arguments)
    & $capyLaunch.executable @capyArguments
} catch {
    Write-Host ('Capy: ' + $_.Exception.Message) -ForegroundColor Red
}
