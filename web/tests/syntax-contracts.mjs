// Shared Node/actual-browser acceptance. Only transport decoding and assertions;
// every operation executes the real Rust interface WASM export.
export function decodeEnvelope(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let at = 0;
  const take = n => { if (n > bytes.length - at) throw Error('truncated LLP1'); const v = bytes.slice(at, at+n); at += n; return v; };
  const integer = () => { const v = view.getUint32(at); at += 4; return v; };
  const field = () => take(integer());
  const text = b => new TextDecoder('utf-8', {fatal:true}).decode(b);
  if (text(take(4)) !== 'LLP1') throw Error('LLP1 magic');
  const header = JSON.parse(text(field()));
  const count = integer(); if (count > 4096) throw Error('LLP1 count');
  const files = [];
  for (let i = 0; i < count; i++) {
    const mime = text(field()), name = text(field());
    const resourceId = view.getBigInt64(at); at += 8;
    files.push({mime, name, resourceId, bytes:field()});
  }
  if (at !== bytes.length) throw Error('LLP1 trailing data');
  return {...header, files};
}
function equal(a, b) {
  if (a === b) return true;
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  const keys = Object.keys(a), others = Object.keys(b);
  return keys.length === others.length && keys.every(k => Object.hasOwn(b,k) && equal(a[k],b[k]));
}
function check(value, context) { if (!value) throw Error(context); }
export async function runSyntaxContracts({load, process, label}) {
  const manifestBytes = await load('cases.json');
  const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));
  const digest = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)), b => b.toString(16).padStart(2,'0')).join('');
  for (const [name, expected] of Object.entries(manifest.sha256)) {
    const hash = await digest(await load(name));
    check(hash === expected, `fixture hash ${name}`);
  }
  const previous = new Map(), receipts = [];
  const data = async source => source == null ? new Uint8Array() : typeof source === 'string' ? load(source) : previous.get(source.case).files[source.file ?? 0].bytes;
  for (const c of manifest.cases) {
    const result = decodeEnvelope(process(JSON.stringify(c.request), await data(c.primary), await data(c.movie)));
    check(result.success === c.success, `${c.id}: status ${JSON.stringify(result.metadata)}`);
    for (const fact of c.expect) {
      const value = fact.path.reduce((v,k) => v[k], result);
      check(equal(value,fact.value), `${c.id}: ${fact.path.join('.')} ${JSON.stringify(value)}`);
    }
    check(result.files.length === (c.files ?? []).length, `${c.id}: file count`);
    for (let i = 0; i < result.files.length; i++) {
      const file = result.files[i], expected = c.files[i], bytes = await data(expected.fixture);
      check(file.mime === expected.mime, `${c.id}: MIME`);
      check(file.bytes.length === bytes.length && file.bytes.every((b,j) => b === bytes[j]), `${c.id}: byte preservation`);
    }
    previous.set(c.id,result);
    receipts.push({id:c.id,fields:c.expect.length,files:result.files.length});
    console.log(`SYNTAX_CONTRACT_CASE ${label} ${c.id} fields=${c.expect.length} files=${result.files.length}`);
  }
  const receipt = {entry:label,cases:receipts.length,checks:receipts,manifest_sha256:await digest(manifestBytes)};
  console.log('SYNTAX_CONTRACTS_PASSED '+JSON.stringify(receipt));
  return receipt;
}
