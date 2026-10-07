param([Parameter(Mandatory=$true)][string]$CapyExe)
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$input | Out-String | & $CapyExe --claude-hook
exit 0
