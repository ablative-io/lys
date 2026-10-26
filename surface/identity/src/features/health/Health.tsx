/** Every outbound wait remains visible until an answer, a close or a person's cancel. */
import { useEffect, useState } from 'react';
import { Refused, request } from '../../api';
import type { OutboundWait, WaitsView } from '../../generated';
import { Act } from '../../shell/Act';
import { PageHeader } from '../../shell/PageHeader';
import { ConfigTabs } from '../settings/ConfigTabs';
import { ErrorWords } from '../people/Words';
import { Elapsed } from './Elapsed';

const refusal = (error: unknown) => error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

export function WaitsList({ waits, cancel }: { waits: OutboundWait[]; cancel: (id: string) => Promise<void> }) {
  const [refused, setRefused] = useState<Refused | null>(null);
  const [cancelling, setCancelling] = useState<string[]>([]);
  const end = async (id: string) => {
    setCancelling((held) => [...held, id]); setRefused(null);
    try { await cancel(id); } catch (error) { setRefused(refusal(error)); }
    finally { setCancelling((held) => held.filter((entry) => entry !== id)); }
  };
  return <div className="stack">
    {refused ? <ErrorWords problem={refused} /> : null}
    {waits.length === 0 ? <p>No outbound calls are waiting.</p> : waits.map((wait) => <section className="card" key={wait.id}>
      <h2>{wait.service}: {wait.act}</h2><p>{wait.subject}</p><p className="mono">{wait.peer}</p>
      <p>Waiting for <Elapsed since={wait.started_at * 1000} /></p>
      <Act symbol="close" name={`Cancel ${wait.service} ${wait.act} for ${wait.subject}`} word="Cancel"
        disabled={cancelling.includes(wait.id)} onClick={() => { void end(wait.id); }} />
    </section>)}
  </div>;
}

export function Health() {
  const [view, setView] = useState<WaitsView | null>(null);
  const [refused, setRefused] = useState<Refused | null>(null);
  useEffect(() => {
    const caller = new AbortController();
    const read = async () => {
      let after: number | undefined;
      try {
        while (!caller.signal.aborted) {
          const answer = await request<WaitsView>('/waits' + (after === undefined ? '' : '?after=' + after), undefined, 'POST', caller.signal);
          if (caller.signal.aborted) return;
          setView(answer); after = answer.generation;
        }
      } catch (error) { if (!caller.signal.aborted) setRefused(refusal(error)); }
    };
    void read();
    return () => caller.abort();
  }, []);
  const cancel = async (id: string) => { await request('/waits/' + encodeURIComponent(id) + '/cancel', {}); };
  return <div className="page fill">
    <PageHeader title="Health" subtitle="What Lys is waiting on. A call ends on its peer, its caller, or a person's cancel." />
    <ConfigTabs on="health" />
    <div className="pane stack">
      {refused ? <ErrorWords problem={refused} /> : null}
      {view ? <WaitsList waits={view.waits} cancel={cancel} /> : refused ? null : <p role="status">Reading waits…</p>}
    </div>
  </div>;
}
