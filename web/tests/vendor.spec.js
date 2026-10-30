import {test,expect} from '@playwright/test';
for(const [suite,count] of [['vendor',13],['vendor-selection',20]]) {
  test(`actual Chrome WASM ${suite}: locked original vendor and explicit selection bytes`,async({page},testInfo)=>{
    const methods=[];page.on('request',r=>methods.push(r.method()));
    page.on('console',v=>{if(v.text().startsWith('SYNTAX_'))console.log(v.text());});
    await page.goto('/');
    const receipt=await page.evaluate(async suite=>{
      const core=await import('/operation-pkg/liblivephoto_interface.js');await core.default();
      const {runSyntaxContracts}=await import('/syntax-contracts.mjs');
      return runSyntaxContracts({suite,label:`chrome-wasm-${suite}`,process:core.process_operation,load:async name=>{
        const response=await fetch((name==='contract-suites.json'?'/':`/${suite}/`)+name);
        if(!response.ok)throw Error(name);return new Uint8Array(await response.arrayBuffer());
      }});
    },suite);
    expect(receipt.cases).toBe(count);
    expect(methods.every(m=>['GET','HEAD'].includes(m))).toBe(true);
    await testInfo.attach(`${suite}-contracts.json`,{body:JSON.stringify({...receipt,requests:methods}),contentType:'application/json'});
  });
}
