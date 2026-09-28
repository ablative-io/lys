/** Build a committed identity surface and package its exact bytes for the native installer. */
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const manifestName = 'surface-manifest.json';

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
}

export function committedSource(cwd) {
  if (git(['status', '--porcelain', '--untracked-files=all', '--', '.'], cwd)) {
    throw new Error(`Surface source has uncommitted changes: ${cwd}`);
  }
  return git(['rev-parse', 'HEAD'], cwd);
}

function collect(directory, base = directory) {
  if (!lstatSync(directory).isDirectory() || lstatSync(directory).isSymbolicLink()) {
    throw new Error(`Surface directory must not be a symlink: ${directory}`);
  }
  const files = [];
  for (const name of readdirSync(directory).sort()) {
    const path = join(directory, name);
    const info = lstatSync(path);
    if (info.isSymbolicLink()) throw new Error(`Surface symlink refused: ${path}`);
    if (info.isDirectory()) files.push(...collect(path, base));
    else if (info.isFile()) files.push({ path: relative(base, path).split('\\').join('/'), data: readFileSync(path) });
    else throw new Error(`Surface entry is not a regular file: ${path}`);
  }
  return files;
}

export function packageSurface(source, destination, commit) {
  if (!/^[0-9a-f]{40}$/.test(commit)) throw new Error('Surface commit must be a full Git commit id');
  const files = collect(source);
  const names = new Set(files.map((file) => file.path));
  const index = files.find((file) => file.path === 'index.html');
  if (!index) throw new Error('Surface package has no index.html');
  if (names.has(manifestName)) throw new Error('Build output must not contain a prior surface manifest');
  if (!files.some((file) => file.path.startsWith('assets/'))) throw new Error('Surface package has no compiled assets');
  for (const match of index.data.toString('utf8').matchAll(/(?:src|href)=["']([^"']+)["']/g)) {
    const url = match[1];
    if (url.startsWith('/assets/') && !names.has(url.slice(1).split(/[?#]/)[0])) {
      throw new Error(`Surface index names an absent asset: ${url}`);
    }
  }
  // Exclusive creation: a refused or interrupted package cannot overwrite another release.
  mkdirSync(destination);
  const entries = [];
  for (const file of files) {
    const target = join(destination, file.path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, file.data, { flag: 'wx' });
    entries.push({ path: file.path, bytes: file.data.length, sha256: createHash('sha256').update(file.data).digest('hex') });
  }
  const manifest = { format: 'lys-identity-surface/v1', commit, files: entries };
  // The manifest is the completion marker, never present in a partial copy.
  writeFileSync(join(destination, manifestName), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 3) throw new Error('Usage: npm run package -- <new package directory>');
    const commit = committedSource(root);
    execFileSync('npm', ['run', 'build'], { cwd: root, stdio: 'inherit' });
    if (committedSource(root) !== commit) throw new Error('Surface source changed during the build');
    const destination = resolve(process.argv[2]);
    const manifest = packageSurface(join(root, 'dist'), destination, commit);
    process.stdout.write(`Packaged ${manifest.files.length} files from ${commit} at ${destination}\n`);
  } catch (error) {
    process.stderr.write(`Surface package refused: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
