/**
 * What the form says under a mode. The mode is shown by its real name with
 * the catalogue's words for it, and under that only what the fact tables
 * support about what Lys does and does not do (docs/design/home/PERMISSIONS-SCREEN.md,
 * proposal 1, and for Claude Code the clean start's own rows). Nothing here says a rule is enforced: no run has been watched
 * being refused. A program the tables do not cover gets the mode and no claim.
 */
import type { Program } from './choices';

/** The sentences for one program and mode; none for a program the tables do not cover. */
export function lysDoes(program: string, mode: string): string[] {
  // The clean start: docs/harness/reference/claude-code/CLEAN-START.md rows 1 to 4 (seen on 2.1.288), and rows 8 and 9 for what is not switched off.
  if (program === 'Claude Code') return ['Lys starts Claude Code without this computer’s own settings, plugins, hooks and connected tools.', 'Instruction files (CLAUDE.md) and Claude Code’s own memory on this computer may still be read. Lys does not check each action.'];
  // The login's own Codex setup reaches a Lys-started run: docs/harness/reference/codex/FACTS.md row 4, and CLEAN-START.md rows 14 and 15.
  const own = 'Codex also uses this computer’s own Codex settings, hooks, connected tools and instruction files.';
  if (program === 'Codex') return mode
    ? ['Lys sets where its commands may write and whether they may use the network, while they stay in the sandbox. It can read every file this login can read.', 'Web search and leaving the sandbox are decided by this computer’s own Codex settings.', own]
    : ['Lys has set nothing; this computer’s own Codex settings decide.', own];
  return [];
}

export function ModeWords({ program, mode, sentencesOnly = false }: { program: Program | undefined; mode: string; /** Leave the mode's own words out, where the picker above already shows them. */ sentencesOnly?: boolean }) {
  if (!program) return null;
  const meaning = program.modes.find((entry) => entry.id === mode)?.meaning;
  return <div className="mode-words">
    {meaning && !sentencesOnly ? <p><code>{mode}</code>{' ' + meaning}</p> : null}
    {lysDoes(program.name, mode).map((sentence) => <p key={sentence}>{sentence}</p>)}
  </div>;
}
