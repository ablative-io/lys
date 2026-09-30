/** The served machine registry and explicit retirement, without inventing runtime liveness. */
import { useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import { AddMachine } from './AddMachine';
import type { Machine, NetworkView } from './contract';

type RunnerRecord = { kind: 'lys' } | { kind: 'socket'; path: string } | { kind: 'dialled'; key: string; runner?: string };
function starts(machine: Machine, runner: RunnerRecord | null): string {
  if (machine.state === 'retired') return 'Retired: no agent can be started on it.';
  if (runner?.kind === 'lys') return 'Lys starts agents here through its own Lys runner.';
  if (runner?.kind === 'dialled') return 'Lys starts agents here through the Lys runner that connects from it, holding key ' + runner.key.slice(0, 8) + '….';
  if (runner?.kind === 'socket') return 'Lys starts agents here through the runner listening at ' + runner.path + '.';
  if (machine.runtime !== null) return 'No Lys runner is recorded for it, so a start here gives you a command to run on it yourself.';
  return 'Lys does not start agents here.';
}

function Retire({ machine, changed }: { machine: Machine; changed: () => void }) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const retire = async () => {
    if (busy) return; setBusy(true); setFailure('');
    try {
      const answer = await request<Machine>('/network/machines/' + encodeURIComponent(machine.id) + '/retire', {});
      if (answer.id !== machine.id || answer.state !== 'retired') throw new Error('Retirement is not confirmed. Reload this page to see whether it was retired.');
      changed();
    } catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { setBusy(false); }
  };
  return <div>{confirm ? <><p>Retire {machine.name}? No agent can be started on it afterwards. Agents already running on it keep running.</p><button className="btn danger" disabled={busy} onClick={() => void retire()}>Confirm retirement</button>{' '}<button className="btn" disabled={busy} onClick={() => setConfirm(false)}>Cancel</button></>
    : <button className="btn" onClick={() => setConfirm(true)}>Retire this computer</button>}{failure ? <p role="alert">{failure}</p> : null}</div>;
}

export function Network() {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const refresh = (message?: string) => { if (message) setNotice(message); setRevision((value) => value + 1); };
  const load = useLoad(async () => {
    const view = await request<NetworkView>('/network');
    const runners = await Promise.all(view.machines.map((machine) => request<{ runner: RunnerRecord | null }>('/network/machines/' + encodeURIComponent(machine.id) + '/runner').then((answer) => answer.runner)));
    return { ...view, runners };
  }, 'network:' + revision);
  const authority = useLoad(async () => ({ people: await api.people(), me: await api.me() }), 'network-authority');
  const admin = authority.status === 'ok' && authority.data.people.scope === 'directory';
  return <div className="page"><div className="head"><div><h1>Computers</h1><p className="sub">The computers Lys can start agents on, and which agents may start on each.</p></div></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Network" ok={(data) => <>
      {!data.machines.length ? <p>No computers have been added.</p> : data.machines.map((machine, index) => <section className="card" key={machine.id}>
        <h2>{machine.name} <span className="note">{machine.kind}{machine.state === 'retired' ? ' · retired' : ''}</span></h2>
        <p>{starts(machine, data.runners[index])}</p>
        <p>{data.reports_served ? 'Last heard from: ' + (machine.last_report_at === null ? 'never' : clock(machine.last_report_at)) + '.' : 'This Lys does not collect runner reports, so it cannot say when this computer was last heard from.'}</p>
        {machine.runtime !== null ? <p>Agents that may start here: {[...machine.may_run.map((agent) => agent.display_name), ...(machine.may_run_roles ?? []).map((role) => 'anyone holding ' + role)].join(', ') || 'none yet, so every start here is refused'}.</p> : null}
        <p>Websites its agents' services may connect to: {machine.may_reach.join(', ') || 'none'}.</p>
        {admin && machine.state !== 'retired' ? <Retire machine={machine} changed={refresh} /> : null}
      </section>)}
    </>} />
    {admin && authority.status === 'ok' && load.status === 'ok' ? <AddMachine person={authority.data.me.person.id} agents={authority.data.people.people.flatMap((person) => person.agents)} changed={refresh} /> : null}
    {authority.status === 'refused' ? <p className="why-not">{authority.refused.refusal.refusal}: {authority.refused.message}</p> : null}
  </div>;
}
