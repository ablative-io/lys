import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { describe, expect, it } from 'vitest';
import { STANDING, STOOD_AT_THE_START } from './bare-buttons.standing';

const SRC = join(__dirname, '..', 'src');
const CONTROL = 'shell/Act.tsx';

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sources(path);
    return entry.name.endsWith('.tsx') ? [path] : [];
  });
}

/** The bare `<button` elements written in each source file, by its path under src. */
function standing(): Record<string, number> {
  const found: Record<string, number> = {};
  for (const path of sources(SRC)) {
    const count = readFileSync(path, 'utf8').match(/<button[\s>]/g)?.length ?? 0;
    if (count > 0) found[relative(SRC, path).split(sep).join('/')] = count;
  }
  return found;
}

describe('Bare buttons', () => {
  it('stand only where the table says, and the act control holds the one button every act is drawn with', () => {
    const { [CONTROL]: control, ...outside } = standing();
    expect(control).toBe(1);
    // A file with more than its number has a new bare button: write an `Act`. A file with fewer has had
    // buttons moved to the control: lower its number here, in the same commit.
    expect(outside).toEqual(STANDING);
  });

  it('never number more than stood when the count began', () => {
    const total = Object.values(STANDING).reduce((sum, count) => sum + count, 0);
    expect(total).toBeLessThanOrEqual(STOOD_AT_THE_START);
    expect(Object.keys(STANDING)).not.toContain(CONTROL);
    expect(Object.values(STANDING).every((count) => Number.isInteger(count) && count > 0)).toBe(true);
  });
});
