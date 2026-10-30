import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {runSyntaxContracts} from './syntax-contracts.mjs';
const require = createRequire(import.meta.url);
const core = require('../dist/operation-node/liblivephoto_interface.js');
test('actual Node WASM: ICC, PCE/wave, direct AAC, stsz/stz2, preservation and exact times', async () => {
  const receipt = await runSyntaxContracts({label:'node-wasm',process:core.process_operation,load:async name => new Uint8Array(await readFile(new URL('../../fixtures/'+(name==='contract-suites.json'?'':'syntax/')+name,import.meta.url)))});
  assert.equal(receipt.cases,42);
});
