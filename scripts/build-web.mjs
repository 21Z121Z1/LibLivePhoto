import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync } from 'node:fs';
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
cpSync(path.join(root, 'web/src'), path.join(root, 'web/dist'), { recursive: true });
cpSync(path.join(root, 'web/public'), path.join(root, 'web/dist'), { recursive: true });
console.log('Built web/dist. Run npm --prefix web run dev (loopback only).');
