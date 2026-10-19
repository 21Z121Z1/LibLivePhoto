import {test,expect} from '@playwright/test';
test('actual Chrome WASM: 42 syntax, byte preservation, exact-time and error vectors', async ({page}, testInfo) => {
  const methods = [];
  page.on('request', request => methods.push(request.method()));
  page.on('console', value => { if (value.text().startsWith('SYNTAX_')) console.log(value.text()); });
  await page.goto('/');
  const receipt = await page.evaluate(async () => {
    const core = await import('/operation-pkg/liblivephoto_interface.js');
    await core.default();
    const {runSyntaxContracts} = await import('/syntax-contracts.mjs');
    return runSyntaxContracts({label:'chrome-wasm',process:core.process_operation,load:async name => {
      const response = await fetch((name==='contract-suites.json'?'/':'/syntax/')+name); if (!response.ok) throw Error(name);
      return new Uint8Array(await response.arrayBuffer());
    }});
  });
  expect(receipt.cases).toBe(42);
  expect(methods.every(method => ['GET','HEAD'].includes(method))).toBe(true);
  await testInfo.attach('syntax-contracts.json',{body:JSON.stringify({...receipt,requests:methods}),contentType:'application/json'});
});
