$ErrorActionPreference = 'SilentlyContinue'
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$capyPayload = $input | Out-String
$capyBridge = Get-Content -Encoding UTF8 -Raw -LiteralPath (Join-Path $PSScriptRoot 'bridge.json') | ConvertFrom-Json
if ($capyBridge.original) {
    $capyPayload | & $capyBridge.executable --claude-statusline $capyBridge.configDir | Out-Null
    if ($capyBridge.bash) {
        $capyPayload | & $capyBridge.bash -c $capyBridge.original.command
    } else {
        $capyPayload | & powershell -NoProfile -Command $capyBridge.original.command
    }
} else {
    $capyPayload | & $capyBridge.executable --claude-statusline $capyBridge.configDir
}
exit 0
