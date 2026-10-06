param(
    [ValidateSet('Enable','Disable')][string]$Action = 'Enable',
    [string]$GeminiHome = $(if ($env:CAPY_ANTIGRAVITY_HOME) { $env:CAPY_ANTIGRAVITY_HOME } else { Join-Path $env:USERPROFILE '.gemini' }),
    [string]$CapyExe = (Join-Path (Split-Path -Parent $PSScriptRoot) 'src-tauri/target/release/capy.exe')
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($GeminiHome)
$exe = [IO.Path]::GetFullPath($CapyExe)
if ($exe.IndexOfAny([char[]]'"$`&|<>^%') -ge 0) { throw 'Unsupported hook executable path' }
if ($Action -eq 'Enable' -and -not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Build Capy first' }
$configDir = Join-Path $root 'config'
$settingsPath = Join-Path $configDir 'hooks.json'
$original = if (Test-Path -LiteralPath $settingsPath) { [IO.File]::ReadAllText($settingsPath) } else { $null }
$config = if ($null -ne $original) { $original | ConvertFrom-Json } else { [pscustomobject]@{} }
if ($config -isnot [pscustomobject]) { throw 'hooks.json must be an object' }
$key = 'capy-observer'
if ($config.PSObject.Properties[$key]) {
    $current = $config.$key
    foreach ($event in @('PreInvocation','PostToolUse','Stop')) {
        if (-not $current.PSObject.Properties[$event]) { throw 'Existing capy-observer group is incompatible; refusing overwrite' }
        $handlers = @(if ($event -eq 'PostToolUse') { $current.$event | ForEach-Object { $_.hooks } } else { $current.$event })
        $expectedCommand = '"' + $exe.Replace('\','/') + '" --antigravity-hook ' + $event
        if ($handlers.Count -ne 1 -or $handlers[0].type -ne 'command' -or $handlers[0].command -cne $expectedCommand) { throw 'Existing capy-observer group belongs to another configuration; refusing overwrite' }
    }
}
$config.PSObject.Properties.Remove($key)
if ($Action -eq 'Enable') {
    $group = [pscustomobject]@{}
    foreach ($event in @('PreInvocation','PostToolUse','Stop')) {
        $handler = [pscustomobject]@{ type='command'; command=('"' + $exe.Replace('\','/') + '" --antigravity-hook ' + $event); timeout=2 }
        $value = @(if ($event -eq 'PostToolUse') { [pscustomobject]@{ matcher='*'; hooks=@($handler) } } else { $handler })
        $group | Add-Member $event $value
    }
    $config | Add-Member $key $group
}
$dir = Join-Path $root 'capy-activity'
if ((Test-Path -LiteralPath $dir) -and ((Get-Item -LiteralPath $dir).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Refusing redirected activity directory' }
New-Item -ItemType Directory -Force -Path $dir | Out-Null
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
$temp = Join-Path $configDir ('capy-hooks-' + [Guid]::NewGuid() + '.tmp')
try {
    [IO.File]::WriteAllText($temp,($config | ConvertTo-Json -Depth 100),[Text.UTF8Encoding]::new($false))
    $latest = if (Test-Path -LiteralPath $settingsPath) { [IO.File]::ReadAllText($settingsPath) } else { $null }
    if ($latest -cne $original) { throw 'Hooks changed during update; retry' }
    Move-Item -LiteralPath $temp -Destination $settingsPath -Force
} finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp } }
$marker = Join-Path $dir 'disabled'
if ($Action -eq 'Disable') { [IO.File]::WriteAllText($marker,'disabled') }
elseif (Test-Path -LiteralPath $marker) { Remove-Item -LiteralPath $marker }
Write-Output "Capy Antigravity hooks: $Action. Restart sessions to load configuration."
