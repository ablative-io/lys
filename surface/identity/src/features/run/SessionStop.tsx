/**
 * Stop beside one live run (AGENTS-017 R2): the ordinary Stop, the same question and the same route as the agent's
 * own page (`useRunActs`, `POST /runtime/sessions/{session}/end`), never the emergency act. The agent's certificate is
 * untouched, so the agent starts again from the same seat file; the receipt says so. A refusal is shown as Lys
 * answered it. Used on the Dashboard's started lines and beside each live session on the agent's Sessions tab.
 */
import { useState } from 'react';
import { Act } from '../../shell/Act';
import { RUN_WORDS, RunConfirm, useRunActs } from './RunConfirm';
import type { SessionEnd } from '../runtime/Terminal';

/** The receipt of an ordinary Stop, in words. */
export const stopped = (name: string, ended: SessionEnd | null): string =>
  `${name}’s run ended${ended ? ' (' + ended.how.replace(/_/g, ' ') + ')' : ''}. Its certificate stands: it starts again from the same seat file.`;

export function SessionStop({ agent, session, name, where }: { agent: string; session: string; name: string; where: string | null }) {
  const [asking, setAsking] = useState(false);
  const [receipt, setReceipt] = useState<string | null>(null);
  const acts = useRunActs(agent, session, (_doing, ended) => { setAsking(false); setReceipt(stopped(name, ended)); });
  if (receipt !== null) return <p role="status" data-session-stopped>{receipt}</p>;
  return <>
    <Act symbol="stop" name={`${RUN_WORDS.stop} ${name}`} word={RUN_WORDS.stop} data-act="stop" disabled={asking || acts.busy}
      onClick={() => { acts.clear(); setAsking(true); }} />
    {asking ? <RunConfirm className="run-ask" small doing="stop" name={name} where={where} busy={acts.busy} refused={acts.refused}
      waitingSince={acts.waitingSince} endingNow={acts.endingNow} endNow={() => { void acts.endNow(); }}
      confirm={() => { void acts.act('stop'); }} cancel={() => { acts.clear(); setAsking(false); }} /> : null}
  </>;
}
