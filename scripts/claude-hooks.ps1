param(
    [ValidateSet('Enable','Disable')][string]$Action = 'Enable',
    [string]$ConfigDir = $(if ($env:CLAUDE_CONFIG_DIR) { $env:CLAUDE_CONFIG_DIR } else { Join-Path $env:USERPROFILE '.claude' }),
    [string]$CapyExe = (Join-Path (Split-Path -Parent $PSScriptRoot) 'src-tauri/target/release/capy.exe')
)
$ErrorActionPreference = 'Stop'
$configRoot = [IO.Path]::GetFullPath($ConfigDir)
$settingsPath = Join-Path $configRoot 'settings.json'
$exePath = [IO.Path]::GetFullPath($CapyExe)
if ($exePath.Contains('"') -or $exePath.Contains('$') -or $exePath.Contains('`')) { throw 'Unsupported executable path for Claude hook shell' }
if ($Action -eq 'Enable' -and -not (Test-Path -LiteralPath $exePath -PathType Leaf)) { throw 'Build Capy first' }
$command = '"' + $exePath.Replace('\','/') + '" --claude-hook'
$original = if (Test-Path -LiteralPath $settingsPath) { [IO.File]::ReadAllText($settingsPath) } else { $null }
$config = if ($null -ne $original) { $original | ConvertFrom-Json } else { [pscustomobject]@{} }
if ($null -eq $config -or $config -isnot [pscustomobject]) { throw 'Settings must be a JSON object' }
if (-not $config.PSObject.Properties['hooks']) { $config | Add-Member hooks ([pscustomobject]@{}) }
if ($null -eq $config.hooks -or $config.hooks -isnot [pscustomobject]) { throw 'hooks must be a JSON object' }
$events = @('SessionStart','UserPromptSubmit','PreToolUse','PermissionRequest','PermissionDenied','PostToolUse','PostToolUseFailure','PostToolBatch','Notification','Stop','StopFailure','SessionEnd')
foreach ($event in $events) {
    $groups = @()
    if ($config.hooks.PSObject.Properties[$event]) {
        foreach ($group in @($config.hooks.$event)) {
            if ($group -isnot [pscustomobject] -or -not $group.PSObject.Properties['hooks']) { throw "Invalid hook group: $event" }
            $kept = @($group.hooks | Where-Object { -not ($_.type -eq 'command' -and $_.command -eq $command) })
            if ($kept.Count -eq @($group.hooks).Count) { $groups += $group }
            elseif ($kept.Count -gt 0) { $group.hooks = $kept; $groups += $group }
        }
    }
    if ($Action -eq 'Enable') { $groups += [pscustomobject]@{ hooks = @([pscustomobject]@{ type='command'; command=$command; timeout=2 }) } }
    if ($groups.Count -gt 0) {
        if ($config.hooks.PSObject.Properties[$event]) { $config.hooks.$event = $groups }
        else { $config.hooks | Add-Member $event $groups }
    } else { $config.hooks.PSObject.Properties.Remove($event) }
}
if (@($config.hooks.PSObject.Properties).Count -eq 0) { $config.PSObject.Properties.Remove('hooks') }
$updated = $config | ConvertTo-Json -Depth 100
New-Item -ItemType Directory -Force -Path $configRoot | Out-Null
$tempPath = Join-Path $configRoot ('capy-settings-' + [Guid]::NewGuid().ToString() + '.tmp')
try {
    [IO.File]::WriteAllText($tempPath, $updated, [Text.UTF8Encoding]::new($false))
    $current = if (Test-Path -LiteralPath $settingsPath) { [IO.File]::ReadAllText($settingsPath) } else { $null }
    if ($current -cne $original) { throw 'Settings changed during update; retry' }
    Move-Item -LiteralPath $tempPath -Destination $settingsPath -Force
} finally { if (Test-Path -LiteralPath $tempPath) { Remove-Item -LiteralPath $tempPath } }
$activityDir = Join-Path $configRoot 'capy-activity'
if (Test-Path -LiteralPath $activityDir) {
    if ((Get-Item -LiteralPath $activityDir).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing redirected activity directory' }
}
New-Item -ItemType Directory -Force -Path $activityDir | Out-Null
if ($Action -eq 'Disable') {
    [IO.File]::WriteAllText((Join-Path $activityDir 'disabled'),'disabled')
    if (Test-Path -LiteralPath $activityDir) {
        $item = Get-Item -LiteralPath $activityDir
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing to clear redirected activity directory' }
        Get-ChildItem -LiteralPath $activityDir -File -Filter '*.json' | ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    }
} elseif (Test-Path -LiteralPath (Join-Path $activityDir 'disabled')) { Remove-Item -LiteralPath (Join-Path $activityDir 'disabled') }
Write-Output "Capy hooks: $Action. Restart Claude Code sessions to apply."
