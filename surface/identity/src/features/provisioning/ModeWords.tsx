/**
 * What the form says under a mode. The mode is shown by its real name with
 * the catalogue's words for it, and under that only what the fact tables
 * support about what Lys does and does not do (docs/design/home/PERMISSIONS-SCREEN.md,
 * proposal 1). Nothing here says a rule is enforced: no run has been watched
 * being refused. A program the tables do not cover gets the mode and no claim.
 */
import type { Program } from './choices';

/** The sentences for one program and mode; none for a program the tables do not cover. */
export function lysDoes(program: string, mode: string): string[] {
  if (program === 'Claude Code') return ['This computer’s own Claude Code settings, plugins and hooks also apply. Lys does not read them and does not check each action.'];
  if (program === 'Codex') return mode
    ? ['Lys sets where its commands may write and whether they may use the network, while they stay in the sandbox. It can read every file this login can read.', 'Web search and leaving the sandbox are decided by this computer’s own Codex settings.']
    : ['Lys has set nothing; this computer’s own Codex settings decide.'];
  return [];
}

export function ModeWords({ program, mode }: { program: Program | undefined; mode: string }) {
  if (!program) return null;
  const meaning = program.modes.find((entry) => entry.id === mode)?.meaning;
  return <div className="mode-words">
    {meaning ? <p><code>{mode}</code>{' ' + meaning}</p> : null}
    {lysDoes(program.name, mode).map((sentence) => <p key={sentence}>{sentence}</p>)}
  </div>;
}
