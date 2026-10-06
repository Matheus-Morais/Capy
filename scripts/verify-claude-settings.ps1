$ErrorActionPreference = 'Stop'
$dir = Join-Path ([IO.Path]::GetTempPath()) ('capy-settings-test-' + [Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $dir | Out-Null
try {
    $settings = Join-Path $dir 'settings.json'
    $fixture = '{"theme":"dark","env":{"EXISTING":"keep"},"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"echo existing"}]}],"CustomEvent":[{"hooks":[{"type":"command","command":"echo custom"}]}]}}'
    [IO.File]::WriteAllText($settings,$fixture)
    $exe = Join-Path $dir 'capy.exe'; [IO.File]::WriteAllText($exe,'test')
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    $once = [IO.File]::ReadAllText($settings)
    & "$PSScriptRoot/claude-hooks.ps1" -Action Enable -ConfigDir $dir -CapyExe $exe | Out-Null
    if ([IO.File]::ReadAllText($settings) -cne $once) { throw 'Enable is not idempotent' }
    $config = $once | ConvertFrom-Json
    if ($config.hooks.Stop.Count -ne 2 -or $config.hooks.Stop[0].hooks[0].command -ne 'echo existing') { throw 'Existing hooks changed' }
    New-Item -ItemType Directory -Force -Path (Join-Path $dir 'capy-activity') | Out-Null
    [IO.File]::WriteAllText((Join-Path $dir 'capy-activity/stale.json'),'{}')
    & "$PSScriptRoot/claude-hooks.ps1" -Action Disable -ConfigDir $dir -CapyExe $exe | Out-Null
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
    Write-Output 'PASS: idempotent enable, preservation, reversible disable, invalidation, invalid JSON'
} finally {
    $resolved = [IO.Path]::GetFullPath($dir)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath())) -or [IO.Path]::GetFileName($resolved) -notlike 'capy-settings-test-*') { throw 'Unsafe cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
