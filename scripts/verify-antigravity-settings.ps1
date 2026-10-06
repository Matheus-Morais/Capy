$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('capy-antigravity-settings-' + [Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path (Join-Path $root 'config') | Out-Null
try {
    $path = Join-Path $root 'config/hooks.json'
    $original = '{"existing":{"Stop":[{"command":"echo existing"}]}}'
    [IO.File]::WriteAllText($path,$original)
    $exe = Join-Path $root 'capy.exe'; [IO.File]::WriteAllText($exe,'fixture')
    & "$PSScriptRoot/antigravity-hooks.ps1" -Action Enable -GeminiHome $root -CapyExe $exe | Out-Null
    $once = [IO.File]::ReadAllText($path)
    & "$PSScriptRoot/antigravity-hooks.ps1" -Action Enable -GeminiHome $root -CapyExe $exe | Out-Null
    if ([IO.File]::ReadAllText($path) -cne $once) { throw 'Enable is not idempotent' }
    $config = $once | ConvertFrom-Json
    if ($config.existing.Stop[0].command -ne 'echo existing') { throw 'Existing hooks changed' }
    if ($config.'capy-observer'.PreInvocation.Count -ne 1 -or $config.'capy-observer'.PostToolUse[0].hooks.Count -ne 1) { throw 'Wrong event shape' }
    & "$PSScriptRoot/antigravity-hooks.ps1" -Action Disable -GeminiHome $root -CapyExe $exe | Out-Null
    if ((Get-Content -LiteralPath $path -Raw | ConvertFrom-Json | ConvertTo-Json -Depth 100) -cne ($original | ConvertFrom-Json | ConvertTo-Json -Depth 100)) { throw 'Disable changed existing hooks' }
    if (-not (Test-Path -LiteralPath (Join-Path $root 'capy-activity/disabled'))) { throw 'Old hooks are not invalidated' }
    & "$PSScriptRoot/antigravity-hooks.ps1" -Action Enable -GeminiHome $root -CapyExe $exe | Out-Null
    if (Test-Path -LiteralPath (Join-Path $root 'capy-activity/disabled')) { throw 'Enable did not reactivate hooks' }
    [IO.File]::WriteAllText($path,'{broken')
    $rejected = $false
    try { & "$PSScriptRoot/antigravity-hooks.ps1" -Action Enable -GeminiHome $root -CapyExe $exe | Out-Null } catch { $rejected=$true }
    if (-not $rejected -or [IO.File]::ReadAllText($path) -cne '{broken') { throw 'Invalid config overwritten' }
    Write-Output 'PASS: idempotent enable, correct shapes, preservation, reversible disable, invalidation, malformed input'
} finally {
    $resolved = [IO.Path]::GetFullPath($root)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath())) -or [IO.Path]::GetFileName($resolved) -notlike 'capy-antigravity-settings-*') { throw 'Unsafe cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
