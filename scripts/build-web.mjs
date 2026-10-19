import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.error || result.status !== 0) throw result.error ?? new Error(`${command} failed (${result.status})`);
}
run('cargo', ['build', '--locked', '--release', '-p', 'liblivephoto-bridge', '--target', 'wasm32-unknown-unknown']);
mkdirSync(path.join(root, 'web/dist/pkg'), { recursive: true });
run('wasm-bindgen', ['--version']);
run('wasm-bindgen', [path.join(root, 'target/wasm32-unknown-unknown/release/liblivephoto_bridge.wasm'), '--target', 'web', '--out-dir', path.join(root, 'web/dist/pkg')]);
// Exercise the existing shared operations through WASM too. Inspection keeps
// its original module; this adapter forwards the native LLP1 protocol directly.
run('cargo', ['build', '--locked', '--release', '-p', 'liblivephoto-interface', '--target', 'wasm32-unknown-unknown']);
for (const [target, directory] of [['web', 'operation-pkg'], ['nodejs', 'operation-node']]) {
  const out = path.join(root, 'web/dist', directory);
  run('wasm-bindgen', [path.join(root, 'target/wasm32-unknown-unknown/release/liblivephoto_interface.wasm'), '--target', target, '--out-dir', out]);
  if (target === 'nodejs') writeFileSync(path.join(out, 'package.json'), '{"type":"commonjs"}\n');
}
cpSync(path.join(root, 'web/src'), path.join(root, 'web/dist'), { recursive: true });
cpSync(path.join(root, 'web/public'), path.join(root, 'web/dist'), { recursive: true });
cpSync(path.join(root, 'fixtures/syntax'), path.join(root, 'web/dist/syntax'), { recursive: true });
cpSync(path.join(root, 'web/tests/syntax-contracts.mjs'), path.join(root, 'web/dist/syntax-contracts.mjs'));
console.log('Built web/dist. Run npm --prefix web run dev (loopback only).');
