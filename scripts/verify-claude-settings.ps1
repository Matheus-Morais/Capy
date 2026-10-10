$ErrorActionPreference = 'Stop'
$dir = Join-Path ([IO.Path]::GetTempPath()) ('capy-settings-test-' + [Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $dir | Out-Null
try {
    $settings = Join-Path $dir 'settings.json'
    $fixture = '{"theme":"dark","env":{"EXISTING":"keep"},"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"echo existing"}]}],"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo pre-existing"}]}],"PermissionRequest":[{"hooks":[{"type":"command","command":"echo permission-existing"}]}],"CustomEvent":[{"hooks":[{"type":"command","command":"echo custom"}]}]}}'
    [IO.File]::WriteAllText($settings,$fixture)
    $exe = Join-Path $dir 'capy.exe'; [IO.File]::WriteAllText($exe,'test')
    $observerCommand = '"' + $exe.Replace('\','/') + '" --claude-hook'
    $responseCommand = '"' + $exe.Replace('\','/') + '" --claude-intervention'

    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    $once = [IO.File]::ReadAllText($settings)
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    if ([IO.File]::ReadAllText($settings) -cne $once) { throw 'Default Enable is not idempotent' }
    $config = $once | ConvertFrom-Json
    if (@($config.hooks.Stop).Count -ne 2 -or $config.hooks.Stop[0].hooks[0].command -ne 'echo existing') { throw 'Existing Stop hook changed' }
    if (@($config.hooks.InstructionsLoaded).Count -ne 1 -or $config.hooks.InstructionsLoaded[0].hooks[0].command -ne $observerCommand -or $config.hooks.InstructionsLoaded[0].hooks[0].timeout -ne 10) { throw 'Instructions observer missing or timeout changed' }
    if (@($config.hooks.PreToolUse | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $responseCommand }).Count -ne 0) { throw 'Responses hook installed without opt-in' }
    if (@($config.hooks.PermissionRequest | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $responseCommand }).Count -ne 0) { throw 'Permission response hook installed without opt-in' }

    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -Responses -ConfigDir $dir -CapyExe $exe | Out-Null
    $responsesMarker = Join-Path $dir 'capy-activity/responses-enabled'
    if (-not (Test-Path -LiteralPath $responsesMarker -PathType Leaf)) { throw 'Responses Enable did not create lifecycle marker' }
    $responseOnce = [IO.File]::ReadAllText($settings)
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -Responses -ConfigDir $dir -CapyExe $exe | Out-Null
    if ([IO.File]::ReadAllText($settings) -cne $responseOnce) { throw 'Responses Enable is not idempotent' }
    if (-not (Test-Path -LiteralPath $responsesMarker -PathType Leaf)) { throw 'Repeated Responses Enable removed lifecycle marker' }
    $config = $responseOnce | ConvertFrom-Json
    foreach ($event in @('PreToolUse','PermissionRequest')) {
        $responseHooks = @($config.hooks.$event | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $responseCommand })
        if ($responseHooks.Count -ne 1 -or $responseHooks[0].timeout -ne 120) { throw "$event response hook missing or timeout changed" }
        $responseGroup = @($config.hooks.$event | Where-Object { @($_.hooks | Where-Object { $_.command -eq $responseCommand }).Count -gt 0 })[0]
        if ($event -eq 'PreToolUse' -and $responseGroup.matcher -ne '^AskUserQuestion$') { throw 'PreToolUse response matcher changed' }
        if ($event -eq 'PermissionRequest' -and $responseGroup.matcher -ne '') { throw 'PermissionRequest response hook must include Bash and other permission requests' }
        $observerHooks = @($config.hooks.$event | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $observerCommand })
        if ($observerHooks.Count -ne 1 -or $observerHooks[0].timeout -ne 2) { throw "$event observer hook or timeout changed" }
    }
    if ($config.hooks.PreToolUse[0].matcher -ne 'Bash' -or $config.hooks.PreToolUse[0].hooks[0].command -ne 'echo pre-existing') { throw 'PreToolUse hook changed' }
    if ($config.hooks.PermissionRequest[0].hooks[0].command -ne 'echo permission-existing') { throw 'PermissionRequest hook changed' }

    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    if (Test-Path -LiteralPath $responsesMarker) { throw 'Default Enable did not remove lifecycle marker' }
    $downgraded = Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json
    foreach ($event in @('PreToolUse','PermissionRequest')) {
        if (@($downgraded.hooks.$event | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $responseCommand }).Count -ne 0) { throw 'Default Enable did not remove response hook' }
        $observerHooks = @($downgraded.hooks.$event | ForEach-Object { $_.hooks } | Where-Object { $_.command -eq $observerCommand })
        if ($observerHooks.Count -ne 1 -or $observerHooks[0].timeout -ne 2) { throw 'Downgrade changed observer hook' }
    }

    New-Item -ItemType Directory -Force -Path (Join-Path $dir 'capy-activity') | Out-Null
    [IO.File]::WriteAllText((Join-Path $dir 'capy-activity/stale.json'),'{}')
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -Responses -ConfigDir $dir -CapyExe $exe | Out-Null
    if (-not (Test-Path -LiteralPath $responsesMarker -PathType Leaf)) { throw 'Responses Enable did not restore lifecycle marker' }
    & "$PSScriptRoot/claude-hooks.ps1" -Action Disable -ConfigDir $dir -CapyExe $exe | Out-Null
    if (Test-Path -LiteralPath $responsesMarker) { throw 'Disable did not remove lifecycle marker' }
    $after = Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json
    $expected = $fixture | ConvertFrom-Json | ConvertTo-Json -Depth 100
    if (($after | ConvertTo-Json -Depth 100) -cne $expected) { throw 'Disable changed unrelated configuration' }
    if (Test-Path -LiteralPath (Join-Path $dir 'capy-activity/stale.json')) { throw 'Old activity was not invalidated' }
    if (-not (Test-Path -LiteralPath (Join-Path $dir 'capy-activity/disabled'))) { throw 'Disabled hooks can still publish activity' }
    [IO.File]::WriteAllText($settings,'{"theme":"dark"}')
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    if (Test-Path -LiteralPath (Join-Path $dir 'capy-activity/disabled')) { throw 'Enable did not reactivate observation' }
    & "$PSScriptRoot/claude-hooks.ps1" -Action Disable -ConfigDir $dir -CapyExe $exe | Out-Null
    $empty = Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json
    if ($empty.theme -ne 'dark' -or $empty.PSObject.Properties['hooks']) { throw 'Enable/Disable left hooks in an originally hookless config' }
    [IO.File]::WriteAllText($settings,'{invalid')
    $rejected=$false
    try { & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null } catch { $rejected=$true }
    if (-not $rejected -or [IO.File]::ReadAllText($settings) -cne '{invalid') { throw 'Invalid JSON was overwritten' }
    Write-Output 'PASS: idempotent observer/responses, preservation, opt-in/downgrade, disable, invalidation, invalid JSON'
} finally {
    $resolved = [IO.Path]::GetFullPath($dir)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath())) -or [IO.Path]::GetFileName($resolved) -notlike 'capy-settings-test-*') { throw 'Unsafe cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
