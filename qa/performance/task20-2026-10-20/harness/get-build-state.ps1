$ErrorActionPreference='Stop'
$active=@(Get-Process cargo,rustc,link,cl -ErrorAction SilentlyContinue | Select-Object Id,ProcessName)
[pscustomobject]@{at=[DateTime]::UtcNow.ToString('o');idle=($active.Count -eq 0);processes=$active} | ConvertTo-Json -Depth 4 -Compress
