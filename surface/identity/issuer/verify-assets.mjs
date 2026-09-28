// Refuse packaging unless the executable embeds every encoding of the actual font.
import { readFileSync, readdirSync } from 'node:fs';
import { brotliDecompressSync, gunzipSync } from 'node:zlib';
const assets = 'static/v1/_app/immutable/assets/';
const names = readdirSync(assets).filter((name) => name.startsWith('dm-sans.') && name.endsWith('.ttf'));
if (names.length !== 1) throw new Error('Expected one generated DM Sans font');
const filename = assets + names[0];
const plain = readFileSync(filename);
const brotli = readFileSync(filename + '.br');
const gzip = readFileSync(filename + '.gz');
if (!brotliDecompressSync(brotli).equals(plain)) throw new Error('Brotli font does not match');
if (!gunzipSync(gzip).equals(plain)) throw new Error('Gzip font does not match');
const binary = readFileSync('out/rauthy_arm64');
for (const [kind, bytes] of [['plain', plain], ['Brotli', brotli], ['gzip', gzip]]) {
  if (binary.indexOf(bytes) < 0) throw new Error(`Executable does not embed the ${kind} font`);
}
console.log('Executable embeds exact plain, Brotli and gzip fonts; all encodings decode identically.');
