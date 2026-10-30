import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {runSyntaxContracts} from './syntax-contracts.mjs';
const require=createRequire(import.meta.url);
const core=require('../dist/operation-node/liblivephoto_interface.js');
for(const [suite,count] of [['vendor',13],['vendor-selection',20]]) {
  test(`actual Node WASM ${suite}: locked IDs, source ranges and original movie bytes`,async()=>{
    const receipt=await runSyntaxContracts({suite,label:`node-wasm-${suite}`,process:core.process_operation,
      load:async name=>new Uint8Array(await readFile(new URL('../../fixtures/'+(name==='contract-suites.json'?'':suite+'/')+name,import.meta.url)))});
    assert.equal(receipt.cases,count);
  });
}
