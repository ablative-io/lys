import { describe, expect, it } from 'vitest';
import { readMachineHarness } from '../src/features/team/Start';

const machine = 'chosen';
const catalogue = { machine, programs: [{ name: 'Codex', builds: [{ program: 'codex', from: 'runner' }] }, { name: 'Claude Code', builds: [{ program: 'claude', from: 'runner' }] }] };

describe('the chosen computer checks the declared program', () => {
  it('refuses cdx when only codex is reported', async () => {
    const refusal = await readMachineHarness(machine, { name: 'Codex', program: 'cdx' }, async () => catalogue);
    expect(refusal?.refusal).toBe('harness_absent_on_machine');
    expect(refusal?.reason).toContain('cdx');
  });
  it('matches the reported command independently of its display name', async () => {
    expect(await readMachineHarness(machine, { name: 'My Codex fork', program: 'codex' }, async () => catalogue)).toBeNull();
  });
  it('accepts a legacy pin through the reported claude command', async () => {
    expect(await readMachineHarness(machine, { name: 'Claude Code', program: '/Users/tom/.local/share/claude/versions/2.1.288/claude' }, async () => catalogue)).toBeNull();
  });
  it.each([
    '/Users/tom/.local/share/claude/versions/2.1.288/claude/',
    '/Users/tom/.local/share/claude/versions/2.1.288/claude\u007f',
    '/Users/tom/.local/share/claude/versions/2.1.288/../claude',
  ])('refuses a malformed legacy path before mapping it: %s', async (program) => {
    const refusal = await readMachineHarness(machine, { name: 'Claude Code', program }, async () => catalogue);
    expect(refusal?.refusal).toBe('harness_program_invalid');
  });
  it('compares an older reported pin through the same command rule', async () => {
    const older = { machine, programs: [{ name: 'Claude Code', builds: [{ program: '/Users/tom/.local/share/claude/versions/2.1.288/claude', from: 'runner' }] }] };
    expect(await readMachineHarness(machine, { name: 'Claude Code', program: 'claude' }, async () => older)).toBeNull();
  });
  it('keeps a failed catalogue read named for the advisory beside Start', async () => {
    const refusal = new Error('discovery unavailable');
    await expect(readMachineHarness(machine, { name: 'Codex', program: '/missing/codex-fork' }, async () => { throw refusal; })).rejects.toBe(refusal);
  });
  it('accepts an explicit absolute fork for the runner to check at spawn', async () => {
    expect(await readMachineHarness(machine, { name: 'Codex', program: '/opt/forks/codex-fork' }, async () => catalogue)).toBeNull();
  });
});

it('accepts the runner command when its installed build reports an absolute executable', async () => {
  const reported = { machine, programs: [{ name: 'Claude Code', builds: [{ program: '/installed/bin/claude', command: 'claude', from: 'installed' }] }] };
  expect(await readMachineHarness(machine, { name: 'Claude Code', program: 'claude' }, async () => reported)).toBeNull();
});
