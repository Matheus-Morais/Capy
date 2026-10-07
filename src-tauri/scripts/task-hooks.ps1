param([Parameter(Mandatory=$true)][string]$CapyExe,[string]$LogPath='')
$OutputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
$capyPayload = [Console]::In.ReadToEnd()
try { $hookEvent = ($capyPayload | ConvertFrom-Json).hook_event_name } catch { $hookEvent = 'unknown' }
if($LogPath){Add-Content -LiteralPath $LogPath -Value "start $hookEvent $(Get-Date -Format o)" -Encoding utf8}
try {
    $capyStart = [Diagnostics.ProcessStartInfo]::new()
    $capyStart.FileName = $CapyExe
    $capyStart.Arguments = '--claude-hook'
    $capyStart.UseShellExecute = $false
    $capyStart.CreateNoWindow = $true
    $capyStart.RedirectStandardInput = $true
    $capyStart.RedirectStandardOutput = $true
    $capyStart.RedirectStandardError = $true
    $capyProcess = [Diagnostics.Process]::Start($capyStart)
    try {
        $capyOutput = $capyProcess.StandardOutput.ReadToEndAsync()
        $capyError = $capyProcess.StandardError.ReadToEndAsync()
        $capyBytes = [Text.UTF8Encoding]::new($false).GetBytes($capyPayload)
        $capyProcess.StandardInput.BaseStream.Write($capyBytes, 0, $capyBytes.Length)
        $capyProcess.StandardInput.Close()
        $capyProcess.WaitForExit()
        if($LogPath){Add-Content -LiteralPath $LogPath -Value "exit $hookEvent $($capyProcess.ExitCode) $(Get-Date -Format o)" -Encoding utf8}
        [void]$capyOutput.GetAwaiter().GetResult()
        [void]$capyError.GetAwaiter().GetResult()
    } finally { $capyProcess.Dispose() }
} catch {}
exit 0
