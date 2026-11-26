# Lists every network connection FilesViewHub and its WebView2 processes open while it runs, for
# the "What this app does" screen (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`). Run on Windows:
#   powershell -ExecutionPolicy Bypass -File scripts\net-check.ps1 -exe <FilesViewHub.exe> -data <a StoredData copy> [-secs 180]
# The data folder keeps the test away from the person's own StoredData.
param([Parameter(Mandatory)][string]$exe, [Parameter(Mandatory)][string]$data, [int]$secs = 180)
$env:FVH_DATA_DIR = $data
Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $exe -PassThru
$seen = @{}
$end = (Get-Date).AddSeconds($secs)
while ((Get-Date) -lt $end -and -not $p.HasExited) {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
  $tree = @($p.Id); $grew = $true
  while ($grew) { $grew = $false; foreach ($x in $all) { if ($tree -contains $x.ParentProcessId -and -not ($tree -contains $x.ProcessId)) { $tree += $x.ProcessId; $grew = $true } } }
  foreach ($c in Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $tree -contains $_.OwningProcess -and $_.RemoteAddress -notin '0.0.0.0', '::', '127.0.0.1', '::1' }) {
    $k = "TCP $($c.RemoteAddress):$($c.RemotePort) process $($c.OwningProcess)"
    if (-not $seen[$k]) { $seen[$k] = (Get-Date).ToString('HH:mm:ss') }
  }
  Start-Sleep -Milliseconds 500
}
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
if ($seen.Count) { $seen.GetEnumerator() | Sort-Object Value | ForEach-Object { "$($_.Value) $($_.Key)" } } else { "No connections." }
