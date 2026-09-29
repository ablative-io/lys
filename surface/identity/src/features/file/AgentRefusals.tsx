/** The tool calls an agent was refused, newest first, with the runners whose refusals have been read. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import type { RefusalRecord, RefusalsView } from './policyContract';

const when = (ms: number) => new Date(ms).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne', timeZoneName: 'short' });

function lift(refusal: RefusalRecord): string {
  if (!refusal.grantable) return 'not grantable';
  if (!refusal.permission) return 'grantable';
  const { resource, action } = refusal.permission;
  return 'grantable by a grant of ' + action + ' on ' + resource.kind + ' ' + resource.id + (refusal.grantor ? ', from ' + refusal.grantor : '');
}

export function AgentRefusals({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const view = await request<RefusalsView>('/agents/' + encodeURIComponent(id) + '/refusals');
    if (view.agent !== id || !Array.isArray(view.refusals) || !Array.isArray(view.read_from)) throw new Error('The refusals answer did not name this agent.');
    return view;
  }, 'agent-refusals:' + id + ':' + revision);
  return (
    <>
      <div className="head"><div><h2>Refused tool calls</h2><p>What this agent's policy stopped, newest first.</p></div>
        <button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh refusals</button></div>
      <Gate load={load} title="Refused tool calls" ok={(view) => (
        <section className="card">
          <p className="note" data-coverage>
            {view.read_from.length
              ? 'Read from runners ' + view.read_from.join(', ') + '. Refusals on any other runner are not yet covered here.'
              : 'No runner’s refusals have been read yet, so this list may be incomplete.'}
          </p>
          {view.refusals.length ? view.refusals.map((refusal) => (
            <article className="row" key={refusal.source + ':' + refusal.attempt} data-refusal>
              <div>
                <div><strong>{refusal.tool}</strong> <span className="mono">{refusal.target}</span></div>
                <div className="note">{refusal.words}</div>
                <div className="note">Rule {refusal.rule ?? 'none'} · check {refusal.check} · policy version {refusal.policy_version} · {lift(refusal)}</div>
              </div>
              <span className="dim">{when(refusal.at)}</span>
            </article>
          )) : <p>No refused tool calls have been read.</p>}
        </section>
      )} />
    </>
  );
}
