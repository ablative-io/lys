/** The sentences under a mode claim only what the fact tables support, and nothing for a program they do not cover. */
import { describe, expect, it } from 'vitest';
import { lysDoes } from '../src/features/provisioning/ModeWords';

describe('What Lys says under a mode', () => {
  it('says of Claude Code that the computer’s own settings apply and Lys checks no action', () => {
    expect(lysDoes('Claude Code', 'default')).toEqual(['This computer’s own Claude Code settings, plugins and hooks also apply. Lys does not read them and does not check each action.']);
  });
  it('says of Codex with no mode that Lys has set nothing', () => {
    expect(lysDoes('Codex', '')).toEqual(['Lys has set nothing; this computer’s own Codex settings decide.']);
  });
  it('says of Codex with a mode what Lys sets, and that it reads every file the login can', () => {
    const said = lysDoes('Codex', 'workspace-write').join(' ');
    expect(said).toContain('while they stay in the sandbox');
    expect(said).toContain('It can read every file this login can read.');
    expect(said).toContain('decided by this computer’s own Codex settings');
  });
  it('claims nothing for a program the tables do not cover', () => {
    expect(lysDoes('Care program', 'default')).toEqual([]);
  });
  it('never says a rule is enforced', () => {
    for (const said of [lysDoes('Claude Code', 'default'), lysDoes('Codex', ''), lysDoes('Codex', 'read-only')]) expect(said.join(' ')).not.toMatch(/enforce|cannot|is refused|is blocked/i);
  });
});
