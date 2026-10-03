/** The sentences under a mode claim only what the fact tables support, and nothing for a program they do not cover. */
import { describe, expect, it } from 'vitest';
import { lysDoes } from '../src/features/provisioning/ModeWords';

describe('What Lys says under a mode', () => {
  it('says of Claude Code that it starts without the computer’s own setup, what may still be read, and that Lys checks no action', () => {
    expect(lysDoes('Claude Code', 'default')).toEqual(['Lys starts Claude Code without this computer’s own settings, plugins, hooks and connected tools.', 'Instruction files (CLAUDE.md) and Claude Code’s own memory on this computer may still be read. Lys does not check each action.']);
  });
  it('says of Codex with no mode that Lys has set nothing', () => {
    expect(lysDoes('Codex', '')).toEqual(['Lys has set nothing; this computer’s own Codex settings decide.', 'Codex also uses this computer’s own Codex settings, hooks, connected tools and instruction files.']);
  });
  it('says of Codex with a mode what Lys sets, and that it reads every file the login can', () => {
    const said = lysDoes('Codex', 'workspace-write').join(' ');
    expect(said).toContain('while they stay in the sandbox');
    expect(said).toContain('It can read every file this login can read.');
    expect(said).toContain('decided by this computer’s own Codex settings');
    expect(said).toContain('Codex also uses this computer’s own Codex settings, hooks, connected tools and instruction files.');
  });
  it('claims nothing for a program the tables do not cover', () => {
    expect(lysDoes('Care program', 'default')).toEqual([]);
  });
  it('never says a rule is enforced', () => {
    for (const said of [lysDoes('Claude Code', 'default'), lysDoes('Codex', ''), lysDoes('Codex', 'read-only')]) expect(said.join(' ')).not.toMatch(/enforce|cannot|is refused|is blocked/i);
  });
});
