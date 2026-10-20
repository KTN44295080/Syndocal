import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
const exec=promisify(execFile),dir=path.resolve('target/qa/task20d'),rounds=[];
for(let round=1;round<=8;round++){
 console.log('ROUND '+round+' '+new Date().toISOString());
 const child=spawn(process.execPath,[path.join(dir,'batch.mjs'),'--phase','after','--seconds','20'],{windowsHide:true,stdio:'inherit'});
 const code=await new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',resolve)});
 if(code!==0)throw Error('Batch stopped for a non-retryable or restoration failure; inspect before continuing.');
 const archived=JSON.parse((await exec('python',[path.join(dir,'archive-contaminated.py')],{windowsHide:true,timeout:60000})).stdout);
 rounds.push({round,at:new Date().toISOString(),archived});
 await fs.writeFile(path.join(dir,'after-retry-rounds.json'),JSON.stringify(rounds,null,2));
 console.log('ARCHIVE '+JSON.stringify(archived));
 if(archived.archived===0){console.log('COMPLETE: all planned supported cases attempted without retained compiler-overlap evidence.');break}
 if(round===8)throw Error('Eight compiler-overlap retry rounds exhausted; preserved evidence requires review.');
}
