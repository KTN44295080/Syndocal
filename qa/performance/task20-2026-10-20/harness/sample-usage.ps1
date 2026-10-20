param([int]$RootProcessId,[int]$Seconds=25,[string]$Out)
$ErrorActionPreference='Stop'
$rows=@();$clock=[Diagnostics.Stopwatch]::StartNew();$cpuLast=@{}
$ids=@($RootProcessId)
while($clock.Elapsed.TotalSeconds -lt $Seconds){
  $processes=@(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,Name)
  $hostBuilds=@($processes|Where-Object {$_.Name -in @('cargo.exe','rustc.exe','link.exe','cl.exe')}|Select-Object ProcessId,Name)
  $ids=@($RootProcessId)
  for($depth=0;$depth -lt 4;$depth++){ $ids=@($ids)+@($processes | Where-Object {$ids -contains [int]$_.ParentProcessId} | ForEach-Object {[int]$_.ProcessId});$ids=@($ids|Select-Object -Unique) }
  $working=0L;$private=0L;$active=@();foreach($procId in $ids){$p=Get-Process -Id $procId -ErrorAction SilentlyContinue;if($p){$cpuKey=[string]$procId+"-"+[string]$p.StartTime.ToUniversalTime().Ticks;$cpuLast[$cpuKey]=$p.TotalProcessorTime.TotalSeconds;$working+=$p.WorkingSet64;$private+=$p.PrivateMemorySize64;$active+= $procId}}
  $cpu=($cpuLast.Values | Measure-Object -Sum).Sum;$cpuAtSeconds=$clock.Elapsed.TotalSeconds
  $gpu=$null;$gpu3d=$null;$gpuDecode=$null;$gpuCopy=$null;$gpuError=$null;$engineCount=0;$count3d=0;$countDecode=0;$countCopy=0
  try {
    $engines=@(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -Property Name,UtilizationPercentage | Where-Object { $m=[regex]::Match($_.Name,'pid_(\d+)_'); $m.Success -and $active -contains [int]$m.Groups[1].Value });$engineCount=$engines.Count
    if($engineCount){$gpu=($engines|Measure-Object UtilizationPercentage -Sum).Sum}
    $e3d=@($engines|Where-Object Name -Match 'engtype_3D');$count3d=$e3d.Count;if($count3d){$gpu3d=($e3d|Measure-Object UtilizationPercentage -Sum).Sum}
    $eDecode=@($engines|Where-Object Name -Match 'engtype_VideoDecode');$countDecode=$eDecode.Count;if($countDecode){$gpuDecode=($eDecode|Measure-Object UtilizationPercentage -Sum).Sum}
    $eCopy=@($engines|Where-Object Name -Match 'engtype_Copy');$countCopy=$eCopy.Count;if($countCopy){$gpuCopy=($eCopy|Measure-Object UtilizationPercentage -Sum).Sum}
  }catch {$gpuError=$_.Exception.Message}
  $rows+=[pscustomobject]@{atSeconds=$cpuAtSeconds;gpuAtSeconds=$clock.Elapsed.TotalSeconds;hostBuildProcesses=$hostBuilds;totalCpuSeconds=$cpu;processCount=$active.Count;processIds=$active;workingSetBytes=$working;privateBytes=$private;gpuEngineCount=$engineCount;gpu3dCount=$count3d;gpuDecodeCount=$countDecode;gpuCopyCount=$countCopy;gpuEngineSumPercent=$gpu;gpu3dPercent=$gpu3d;gpuDecodePercent=$gpuDecode;gpuCopyPercent=$gpuCopy;gpuError=$gpuError;gpuEngines=@($engines|Select-Object Name,UtilizationPercentage)}
  Start-Sleep -Milliseconds 750
}
$rows|ConvertTo-Json -Depth 4|Set-Content -LiteralPath $Out -Encoding utf8
