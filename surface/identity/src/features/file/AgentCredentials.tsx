import { DropHandle } from './DropHandle';
/** Broker handle metadata is discoverability-filtered; this screen never receives or displays a credential value. */
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import { clock } from './time';

interface HeldHandle { id: string; secret: string; max_uses: number; used: number; not_after_ms: number; dropped: boolean; spend_cap: number | null; settled: number; parent: string | null }
interface HeldHandles { holder: string; handles: HeldHandle[] }
export function AgentCredentials({ id }: { id: string }) {
  const caller = useLoad(api.me, 'handle-caller');
  const load = useLoad(async () => {
    const answer = await request<HeldHandles>('/secrets/handles?holder=' + encodeURIComponent(id));
    if (answer.holder !== id || !Array.isArray(answer.handles)) throw new Error('The credential service did not answer this agent’s handles.');
    return answer;
  }, 'agent-handles:' + id);
  return <section className="card"><h2>Credential handles</h2><p>The agent is given a handle to use a secret, never the secret itself. Only handles on secrets you may see are listed.</p>
    <Gate load={load} title="Credential handles" ok={({ handles }) => handles.length ? <table><thead><tr><th>Secret</th><th>Uses</th><th>Spending</th><th>Works until</th><th>State</th><th>Change</th></tr></thead><tbody>{handles.map((handle) => <tr key={handle.id} title={'Handle ' + handle.id}>
      <td>{handle.secret}{handle.parent ? <p className="note">Passed on from another handle</p> : null}</td>
      <td>{handle.used} of {handle.max_uses}</td>
      <td>{handle.settled} spent{handle.spend_cap === null ? ', no cap' : ' of a cap of ' + handle.spend_cap}</td>
      <td>{clock(Math.floor(handle.not_after_ms / 1000))}</td><td>{handle.dropped ? 'Ended' : 'In use'}</td><td>{caller.status === 'ok' ? <DropHandle handle={handle.id} person={caller.data.person.id} dropped={handle.dropped} /> : null}</td>
    </tr>)}</tbody></table> : <p>No handles visible to you were returned for this agent.</p>} />
    {caller.status === 'refused' ? <><p className="why-not">Who is signed in could not be read, so a handle cannot be ended here.</p><ErrorWords problem={caller.refused} /></> : null}
    <p className="note">What is shown here does not promise that the agent’s next use of a secret will be allowed.</p>
  </section>;
}
