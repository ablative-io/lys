import { DropHandle } from './DropHandle';
/** Broker handle metadata is discoverability-filtered; this screen never receives or displays a credential value. */
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from './time';

interface HeldHandle { id: string; secret: string; max_uses: number; used: number; not_after_ms: number; dropped: boolean; spend_cap: number | null; settled: number; parent: string | null }
interface HeldHandles { holder: string; handles: HeldHandle[] }
export function AgentCredentials({ id }: { id: string }) {
  const caller = useLoad(api.me, 'handle-caller');
  const load = useLoad(async () => {
    const answer = await request<HeldHandles>('/secrets/handles?holder=' + encodeURIComponent(id));
    if (answer.holder !== id || !Array.isArray(answer.handles)) throw new Error('The broker did not answer handles for ' + id);
    return answer;
  }, 'agent-handles:' + id);
  return <section className="card"><h2>Credential handles</h2><p>The agent uses handles, not secret values. Only handles on secrets you may discover are shown.</p>
    <Gate load={load} title="Credential handles" ok={({ handles }) => handles.length ? <table><thead><tr><th>Secret</th><th>Handle identifier</th><th>Value</th><th>Usage</th><th>Expires</th><th>Recorded state</th><th>Action</th></tr></thead><tbody>{handles.map((handle) => <tr key={handle.id}>
      <td>{handle.secret}</td><td><span className="mono">{handle.id}</span>{handle.parent ? <details><summary>Parent handle</summary>{handle.parent}</details> : null}</td><td>Never shown</td>
      <td>{handle.used} of {handle.max_uses} uses<details><summary>Spend accounting</summary><p>Settled: {handle.settled}</p><p>Recorded cap: {handle.spend_cap === null ? 'none' : handle.spend_cap}</p></details></td>
      <td>{clock(Math.floor(handle.not_after_ms / 1000))}</td><td>{handle.dropped ? 'Dropped' : 'Not marked dropped'}</td><td>{caller.status === 'ok' ? <DropHandle handle={handle.id} person={caller.data.person.id} dropped={handle.dropped} /> : null}</td>
    </tr>)}</tbody></table> : <p>No handles visible to you were returned for this agent.</p>} />
    {caller.status === 'refused' ? <p className="why-not">{caller.refused.refusal.refusal}: {caller.refused.refusal.reason}</p> : null}
    <p className="note">Usage, expiry and recorded drop state do not promise that a future call will be permitted.</p>
  </section>;
}
