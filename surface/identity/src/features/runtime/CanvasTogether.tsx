/**
 * Typing to several agents at once. The person chooses agents' windows together (a press on a window's bar with Command,
 * Control or Shift held), writes once, and what was written is typed into each one's terminal, then Enter. Each agent's
 * outcome is said by name: delivered, or why not. Nothing is sent twice: an outcome that is not known is said so.
 */
import { useState } from 'react';
import { counted } from '../../shell/count';
import { typed } from './terminal-transport';

export interface Together { node: string; session: string; name: string }
type Outcome = { name: string; failed: string | null };

export function TypeTogether({ chosen, drop, clear, all }: {
  chosen: Together[];
  /** Takes one agent out of those typed to; `clear` takes them all out; `all` chooses every running agent. */
  drop: (node: string) => void; clear: () => void; all: (() => void) | null;
}) {
  const [words, setWords] = useState('');
  const [busy, setBusy] = useState(false);
  const [outcomes, setOutcomes] = useState<Outcome[] | null>(null);
  const send = async () => {
    if (!words.trim() || busy) return;
    setBusy(true);
    setOutcomes(null);
    const bytes = Array.from(new TextEncoder().encode(words));
    // The words, then Enter on its own, so a terminal that takes pasted text as a paste still takes the Enter as a key.
    const done = await Promise.all(chosen.map(async ({ session, name }): Promise<Outcome> => {
      try { await typed(session, bytes); await typed(session, [13]); return { name, failed: null }; } catch (problem) { return { name, failed: problem instanceof Error ? problem.message : String(problem) }; }
    }));
    setOutcomes(done);
    if (done.every((each) => each.failed === null)) setWords('');
    setBusy(false);
  };
  const failed = outcomes?.filter((each) => each.failed !== null) ?? [];
  return <form className="canvas-together" aria-label={'Type to ' + counted(chosen.length, 'agents')} onSubmit={(event) => { event.preventDefault(); void send(); }}>
    <div className="canvas-together-whom">
      <span className="sec">To</span>
      {chosen.map((each) => <span key={each.node} className="canvas-together-chip" data-together={each.node}>{each.name}
        <button type="button" aria-label={'Leave ' + each.name + ' out'} title="Leave out" onClick={() => drop(each.node)}>×</button></span>)}
      {all ? <button type="button" className="btn" data-act="together-all" onClick={all}>Every running agent</button> : null}
      <button type="button" className="btn" data-act="together-clear" onClick={clear}>Done</button>
    </div>
    <div className="canvas-together-line">
      <textarea aria-label={'What to type to ' + counted(chosen.length, 'agents')} placeholder={'Type once; it goes to ' + counted(chosen.length, 'agents') + '. Enter sends, Shift and Enter makes a new line.'} rows={Math.min(8, words.split('\n').length)} value={words} disabled={busy}
        onChange={(event) => setWords(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void send(); } }} />
      <button type="submit" className="btn primary" disabled={busy || !words.trim()}>Send</button>
    </div>
    {outcomes && !failed.length ? <p className="sec" role="status">Typed to {counted(outcomes.length, 'agents')}.</p> : null}
    {failed.length ? <p className="why-not" role="alert">Not typed to {failed.map((each) => each.name + ': ' + each.failed).join('; ')}{outcomes && outcomes.length > failed.length ? '. Typed to ' + outcomes.filter((each) => each.failed === null).map((each) => each.name).join(', ') + '.' : ''}</p> : null}
  </form>;
}
