import csv, json, math, sys
from pathlib import Path
stem, pid, seconds = Path(sys.argv[1]), int(sys.argv[2]), float(sys.argv[3])
usage=json.loads(Path(str(stem)+'-usage.json').read_text(encoding='utf-8-sig'))
assert isinstance(usage,list) and len(usage)>=3, 'at least three process-usage samples'
span=usage[-1]['atSeconds']-usage[0]['atSeconds']
assert span >= seconds*0.6, ('usage observation too short',span)
for row in usage:
 for key in ['atSeconds','totalCpuSeconds','workingSetBytes','privateBytes']:
  assert isinstance(row[key],(int,float)) and math.isfinite(row[key]) and row[key]>=0, (key,row)
 assert row['workingSetBytes']>0 and row['privateBytes']>0, 'process memory was observed'
assert usage[-1]['totalCpuSeconds']>=usage[0]['totalCpuSeconds'], 'cumulative CPU did not regress'
assert all(not r.get('hostBuildProcesses',[]) for r in usage), 'concurrent compiler or test workload invalidates performance comparison'
gpu=[r for r in usage if not r.get('gpuError') and r.get('gpuEngineCount',0)>0]
present_file_missing=False
try:
 with Path(str(stem)+'-presentmon.csv').open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
except FileNotFoundError:
 # Missing acquisition is distinct from a present CSV containing zero rows.
 # No PresentMon identity, delivery rate, or physical scanout is established.
 rows=[]
 present_file_missing=True
if rows:
 assert {int(r['ProcessID']) for r in rows}=={pid}, 'PresentMon exact native process'
 assert {r['Application'].lower() for r in rows}=={'syndocal.exe'}, 'PresentMon exact native application'
chains=sorted({r['SwapChainAddress'] for r in rows})
assert len(chains)<=1, ('ambiguous native output swapchain',chains)
intervals=[float(r['MsBetweenPresents']) for r in rows if r.get('MsBetweenPresents') not in [None,'','NA']]
assert all(math.isfinite(v) and v>=0 for v in intervals), 'finite nonnegative present intervals'
partial=['presentmon_file_missing'] if present_file_missing else []
if len(rows)<10 or len(intervals)<10: partial.append('presentmon_insufficient_samples')
result={'backgroundBuildSamplingAvailable':all('hostBuildProcesses' in r for r in usage),'backgroundBuildOverlap':False,'valid':not partial,'partialReasons':partial,'usageSamples':len(usage),'usageSpanSeconds':span,'gpuProviderAvailable':bool(gpu),'gpuValidSamples':len(gpu),'gpuErrors':sorted({r['gpuError'] for r in usage if r.get('gpuError')}),'presentRows':None if present_file_missing else len(rows),'presentFileMissing':present_file_missing,'presentProcessId':pid,'presentIdentityObserved':bool(rows),'swapChains':chains,'presentApplication':sorted({r['Application'] for r in rows}),'presentMonBoundary':'Present calls on the single native Display swapchain; not unique decoded frames or verified physical scanout.'}
print(json.dumps(result))
