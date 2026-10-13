// Actual installation/removal is restricted to a disposable GitHub-hosted Mac.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {execFileSync,spawnSync} from 'node:child_process';
assert.equal(process.platform,'darwin');
assert.equal(process.env.GITHUB_ACTIONS,'true');
assert.equal(process.env.RUNNER_ENVIRONMENT,'github-hosted');
const [pkg,reportPath,sourceApp]=process.argv.slice(2);
assert.ok(pkg && reportPath && sourceApp && process.argv.length===5);
const app='/Applications/Syndocal.app', directory='/Library/Application Support/jp.seraf.ktn.syndocal';
assert.equal(fs.existsSync(app),false,'Never overwrite an existing app on the test host');
assert.equal(fs.existsSync(directory),false,'Never overwrite existing installation settings');
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'syndocal-pkg-lifecycle-'));
const external=path.join(temp,'keep.sdc');fs.writeFileSync(external,'external-project');
const data=path.join(os.homedir(),'Library/Application Support/jp.seraf.ktn.syndocal');
assert.equal(fs.existsSync(data),false,'Never overwrite existing user state');fs.mkdirSync(data,{recursive:true});
fs.writeFileSync(path.join(data,'sentinel'),'preserved-user-setting');
const ini=path.join(directory,'extensions.ini'), checks=[];
function record(name,fn){fn();checks.push(name);}
function sudo(args){return execFileSync('/usr/bin/sudo',args,{encoding:'utf8',stdio:['ignore','pipe','pipe']});}
function choices(){const xml=execFileSync('/usr/sbin/installer',['-showChoiceChangesXML','-pkg',pkg],{encoding:'utf8'});return JSON.parse(execFileSync('/usr/bin/plutil',['-convert','json','-o','-','--','-'],{input:xml,encoding:'utf8'}));}
function selected(items,id){const value=items.find(x=>x.choiceIdentifier===id&&x.choiceAttribute==='selected');assert.ok(value);return Number(value.attributeSetting);}
function install(enabled){const args=['/usr/sbin/installer','-pkg',pkg,'-target','/'];if(enabled!==undefined){const file=path.join(temp,'choices.plist');fs.writeFileSync(file,`<?xml version="1.0"?><plist version="1.0"><array><dict><key>choiceIdentifier</key><string>djlink</string><key>choiceAttribute</key><string>selected</string><key>attributeSetting</key><integer>${enabled}</integer></dict></array></plist>`);args.push('-applyChoiceChangesXML',file);}sudo(args);}
function readChoice(expected){assert.equal(fs.readFileSync(ini,'utf8'),`[SyndocalExtensions]\nSchema=1\nDJLink=${expected}\n`);assert.equal(fs.readFileSync(path.join(data,'sentinel'),'utf8'),'preserved-user-setting');}
const report={passed:false,sourceCommit:process.env.GITHUB_SHA,checks,physicalOutput:false,packageSha256:crypto.createHash('sha256').update(fs.readFileSync(pkg)).digest('hex')};
try {
 record('Fresh installer defaults to core with an editable DJ Link choice',()=>assert.equal(selected(choices(),'djlink'),0));
 install();record('Default core choice is installed and user state preserved',()=>readChoice(0));
 install(1);record('Installer enables DJ Link and preserves user state',()=>readChoice(1));
 record('Update selection reflects the previous installed choice',()=>assert.equal(selected(choices(),'djlink'),1));
 install();record('Update preserves enabled integration',()=>readChoice(1));
 install(0);record('Installer removes DJ Link selection',()=>readChoice(0));
 const future='[SyndocalExtensions]\nSchema=2\nDJLink=1\n';
 execFileSync('/usr/bin/sudo',['/usr/bin/tee',ini],{input:future,stdio:['pipe','ignore','pipe']});
 const invalid=spawnSync('/usr/bin/sudo',['/usr/sbin/installer','-pkg',pkg,'-target','/'],{encoding:'utf8'});
 record('Future configuration rejects installation without overwriting data',()=>{assert.notEqual(invalid.status,0);assert.equal(fs.readFileSync(ini,'utf8'),future);});
 execFileSync('/usr/bin/sudo',['/usr/bin/tee',ini],{input:'[SyndocalExtensions]\nSchema=1\nDJLink=0\n',stdio:['pipe','ignore','pipe']});
 record('Installed app retains its exact signed executable',()=>{execFileSync('/usr/bin/codesign',['--verify','--deep','--strict',app]);const executable='Contents/MacOS/syndocal';assert.deepEqual(fs.readFileSync(path.join(app,executable)),fs.readFileSync(path.join(sourceApp,executable)));execFileSync(path.join(app,executable),['--syndocal-process-path-v1',String(process.pid)]);});
 execFileSync('/bin/bash',[new URL('../../tools/distribution/Remove-Syndocal.command',import.meta.url).pathname,'--remove-approved'],{encoding:'utf8'});
 record('Complete removal purges app, installer choice and user state',()=>{assert.equal(fs.existsSync(app),false);assert.equal(fs.existsSync(directory),false);assert.equal(fs.existsSync(data),false);});
 record('Complete removal forgets exact package receipts',()=>{for(const id of ['app','core','djlink'])assert.notEqual(spawnSync('/usr/sbin/pkgutil',['--pkg-info',`jp.seraf.ktn.syndocal.pkg.${id}`]).status,0);});
 record('External project survives complete removal',()=>assert.equal(fs.readFileSync(external,'utf8'),'external-project'));
 report.passed=true;
} finally {fs.mkdirSync(path.dirname(reportPath),{recursive:true});fs.writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n');}
console.log(JSON.stringify(report));
