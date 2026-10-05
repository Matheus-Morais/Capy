$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'src-tauri\target\release\capy.exe'
$report = Join-Path $root '.checks\native-smoke.json'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Build first: npm run desktop:build' }
if (Get-Process capy -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe }) {
    throw 'Close the running Capy instance before the native smoke test'
}
if (Test-Path -LiteralPath $report) { Remove-Item -LiteralPath $report }
$process = Start-Process -FilePath $exe -ArgumentList @('--self-test', ('"' + $report + '"')) -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(30000)) { $process.Kill(); throw 'Native smoke test timed out' }
if ($process.ExitCode -ne 0) { throw "Native smoke failed (exit $($process.ExitCode)); inspect $report" }
$result = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
$result | Format-List
if (-not $result.passed) { throw 'Native smoke assertions failed' }
