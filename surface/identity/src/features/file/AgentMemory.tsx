/** Agent home metadata shows provenance and context receipts without exposing memory notes or transcripts. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';

export interface MemoryAnswer {
  agent: string;
  home: boolean;
  memories: { id: string; session: string; point: string; lit_by: string; lit_at: string; lit_in: string | null; epilogues: number }[];
  skipped: { session: string; reason: string }[];
  last_given: null | { session: string; entry: string; given_at: string; harness: string; harness_version: string; documents: number; environment: number };
  visible_to: { agent: string; responsible: string | null; administrator: boolean };
  notes_shown: false;
}
const date = (value: string) => new Date(value).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne', timeZoneName: 'short' });
export function AgentMemory({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const answer = await request<MemoryAnswer>('/agents/' + encodeURIComponent(id) + '/memory');
    if (answer.agent !== id || answer.visible_to.agent !== id || answer.notes_shown !== false || typeof answer.home !== 'boolean' || !Array.isArray(answer.memories) || !Array.isArray(answer.skipped)) throw new Error('The memory answer did not name this agent and its visibility.');
    if (!answer.home && (answer.memories.length || answer.skipped.length || answer.last_given !== null)) throw new Error('The memory answer contains records without an agent home.');
    return answer;
  }, 'agent-memory:' + id + ':' + revision);
  return <><div className="head"><div><h2>Memory and home</h2><p>Where this agent’s memories came from and the last context it received.</p></div><button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh memory</button></div>
    <Gate load={load} title="Memory and home" ok={(answer) => <>
      <section className="card"><h3>Who can see this record</h3><p>The agent, {answer.visible_to.responsible ? <a href={'#/file/' + answer.visible_to.responsible}>its responsible person</a> : 'no responsible person currently recorded'}{answer.visible_to.administrator ? ', and directory administrators' : ''}.</p><p className="note">Memory notes and conversation transcripts are not shown here.</p></section>
      {!answer.home ? <section className="card"><p>No home has been recorded for this agent.</p></section> : <>
        <section className="card"><h3>Recorded memories</h3>{answer.memories.length ? answer.memories.map((memory) => <article key={memory.id} className="card"><h4 className="mono">{memory.id}</h4><dl className="facts"><dt>Session</dt><dd>{memory.session}</dd><dt>Point</dt><dd>{memory.point}</dd><dt>Recorded by</dt><dd>{memory.lit_by}</dd><dt>Recorded at</dt><dd>{date(memory.lit_at)}</dd><dt>Recorded in</dt><dd>{memory.lit_in ?? 'Not recorded'}</dd><dt>Epilogues</dt><dd>{memory.epilogues}</dd></dl></article>) : <p>No memories have been recorded in this home.</p>}</section>
        <section className="card"><h3>Last context given</h3>{answer.last_given ? <dl className="facts"><dt>Session</dt><dd>{answer.last_given.session}</dd><dt>Entry</dt><dd>{answer.last_given.entry}</dd><dt>Given at</dt><dd>{date(answer.last_given.given_at)}</dd><dt>Harness</dt><dd>{answer.last_given.harness} {answer.last_given.harness_version}</dd><dt>Documents</dt><dd>{answer.last_given.documents}</dd><dt>Environment entries</dt><dd>{answer.last_given.environment}</dd></dl> : <p>No context delivery has been recorded.</p>}</section>
      </>}
      {answer.skipped.length ? <section className="card"><h3>Records that could not be read</h3><p>This list is incomplete for the reasons below.</p><ul>{answer.skipped.map((entry, index) => <li key={index}>{entry.session}: {entry.reason}</li>)}</ul></section> : null}
    </>} />
  </>;
}
