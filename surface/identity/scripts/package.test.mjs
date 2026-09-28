/** Verify exact installer bytes and refusal before publishing an incomplete package. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { packageSurface } from './package.mjs';
const commit = 'a'.repeat(40);
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'lys-package-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const source = join(root, 'dist'); const output = join(root, 'package');
  mkdirSync(join(source, 'assets'), { recursive: true });
  writeFileSync(join(source, 'index.html'), '<script src="/assets/app-abcd.js"></script>');
  writeFileSync(join(source, 'assets/app-abcd.js'), 'document.title="Lys";');
  return { root, source, output };
}
test('packages exact bytes under the installer contract', (t) => {
  const { source, output } = fixture(t);
  const manifest = packageSurface(source, output, commit);
  assert.equal(manifest.format, 'lys-identity-surface/v1'); assert.equal(manifest.commit, commit);
  assert.deepEqual(JSON.parse(readFileSync(join(output, 'surface-manifest.json'), 'utf8')), manifest);
  assert.equal(manifest.files.length, 2);
  for (const file of manifest.files) {
    const data = readFileSync(join(output, file.path));
    assert.equal(file.bytes, data.length);
    assert.equal(file.sha256, createHash('sha256').update(data).digest('hex'));
  }
});
for (const [name, prepare, message] of [
  ['symlink', (source) => symlinkSync(join(source, 'index.html'), join(source, 'linked')), /symlink refused/],
  ['missing index', (source) => rmSync(join(source, 'index.html')), /no index.html/],
  ['missing asset', (source) => writeFileSync(join(source, 'index.html'), '<script src="/assets/absent.js"></script>'), /absent asset/],
  ['uncompiled output', (source) => rmSync(join(source, 'assets'), { recursive: true }), /no compiled assets/],
  ['old manifest', (source) => writeFileSync(join(source, 'surface-manifest.json'), '{}'), /prior surface manifest/],
]) {
  test(`refuses ${name} before creating destination`, (t) => {
    const { source, output } = fixture(t); prepare(source);
    assert.throws(() => packageSurface(source, output, commit), message);
    assert.equal(existsSync(output), false);
  });
}
test('retains an existing destination', (t) => {
  const { source, output } = fixture(t); mkdirSync(output); writeFileSync(join(output, 'retained'), 'original');
  assert.throws(() => packageSurface(source, output, commit), /EEXIST/);
  assert.equal(readFileSync(join(output, 'retained'), 'utf8'), 'original');
});
test('refuses symbolic commit', (t) => {
  const { source, output } = fixture(t);
  assert.throws(() => packageSurface(source, output, 'main'), /full Git commit/);
});
test('refuses symlinked source directory', (t) => {
  const { root, source, output } = fixture(t); const alias = join(root, 'alias'); symlinkSync(source, alias);
  assert.throws(() => packageSurface(alias, output, commit), /must not be a symlink/);
});
