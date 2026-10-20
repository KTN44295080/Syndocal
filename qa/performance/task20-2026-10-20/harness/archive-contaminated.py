import pathlib,json,hashlib,datetime
p=pathlib.Path('target/qa/task20d')
progress=json.loads((p/'after-batch-progress.json').read_text(encoding='utf-8-sig'))
finished={x['case'] for x in progress if x.get('status')=='failed'}
selected=[]
for f in p.glob('after-*.json'):
 try: j=json.loads(f.read_text(encoding='utf-8-sig'))
 except Exception: continue
 if not isinstance(j,dict) or not isinstance(j.get('spec'),dict): continue
 tag=j['spec'].get('tag')
 if tag not in finished or 'concurrent compiler or test workload invalidates performance comparison' not in str(j.get('error','')): continue
 assert not j.get('ok') and not j.get('restoreError') and not j.get('stopError'), f.name
 assert j.get('fixtureSha256Before')==j.get('fixtureSha256After'), f.name
 selected.append((f,j))
if not selected:
 print(json.dumps({'archived':0}));raise SystemExit(0)
stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
dest=p/'validation'/'concurrent-build-attempts'/stamp
dest.mkdir(parents=True,exist_ok=False)
manifest={'at':stamp,'reason':'Rejected because cargo/rustc/link/cl overlap was sampled; preserve evidence and rerun after the active matrix.','cases':[]}
for report,j in selected:
 stem=report.stem
 files=sorted(f for f in p.iterdir() if f.is_file() and (f.name==stem+'.json' or f.name.startswith(stem+'-')))
 entry={'tag':j['spec']['tag'],'report':report.name,'executableSha256':j['executableSha256'],'restorationErrors':False,'files':[]}
 for f in files:
  entry['files'].append({'name':f.name,'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()})
 manifest['cases'].append(entry)
(dest/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
for entry in manifest['cases']:
 for info in entry['files']:
  (p/info['name']).rename(dest/info['name'])
print(json.dumps({'archived':len(selected),'path':str(dest),'tags':[e['tag'] for e in manifest['cases']]},ensure_ascii=True))
