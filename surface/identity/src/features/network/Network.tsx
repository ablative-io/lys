/** The served machine registry and explicit retirement, without inventing runtime liveness. */
import { useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import { AddMachine } from './AddMachine';
import type { Machine, NetworkView } from './contract';

function Retire({ machine, changed }: { machine: Machine; changed: () => void }) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const retire = async () => {
    if (busy) return; setBusy(true); setFailure('');
    try {
      const answer = await request<Machine>('/network/machines/' + encodeURIComponent(machine.id) + '/retire', {});
      if (answer.id !== machine.id || answer.state !== 'retired') throw new Error('Retirement is not confirmed. Refresh the machine before acting further.');
      changed();
    } catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { setBusy(false); }
  };
  return <div>{confirm ? <><p>Retire {machine.name}? Its record stays. This does not stop any running process.</p><button className="btn danger" disabled={busy} onClick={() => void retire()}>Confirm retirement</button>{' '}<button className="btn" disabled={busy} onClick={() => setConfirm(false)}>Cancel</button></>
    : <button className="btn" onClick={() => setConfirm(true)}>Retire machine</button>}{failure ? <p role="alert">{failure}</p> : null}</div>;
}

export function Network() {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const refresh = (message?: string) => { if (message) setNotice(message); setRevision((value) => value + 1); };
  const load = useLoad(() => request<NetworkView>('/network'), 'network:' + revision);
  const authority = useLoad(async () => ({ people: await api.people(), me: await api.me() }), 'network-authority');
  const admin = authority.status === 'ok' && authority.data.people.scope === 'directory';
  return <div className="page"><div className="head"><div><div className="eyebrow">Runtime</div><h1>Network</h1><p className="sub">Machines, their declared runtimes and the access recorded for them.</p></div><button className="btn" onClick={() => refresh()}>Refresh network</button></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Network" ok={(data) => <>
      {!data.reports_served ? <p className="note">Runtime reporting is not connected to this service. These records do not show whether a machine is online or an agent is running.</p> : null}
      {!data.machines.length ? <p>No machines have been registered.</p> : data.machines.map((machine) => <section className="card" key={machine.id}>
        <h2>{machine.name} <span className="note">{machine.kind} · {machine.state === 'retired' ? 'Retired' : 'Registered'}</span></h2>
        <p>Runtime: {machine.runtime ?? 'none recorded'} · Agent slots: {machine.slots}</p>
        <h3>Permitted agents</h3>{machine.may_run.length ? <ul>{machine.may_run.map((agent) => <li key={agent.id}><a href={'#/file/' + agent.id}>{agent.display_name}</a> · {agent.state}</li>)}</ul> : <p>None recorded.</p>}
        <h3>Permitted hosts</h3><p>{machine.may_reach.join(', ') || 'None recorded.'}</p>
        <p className="note">Last runtime report: {machine.last_report_at === null ? 'none received' : clock(machine.last_report_at)}.</p>
        <details><summary>Record details</summary><p>{machine.id} · Recorded {clock(machine.named_at)} by {machine.named_by}.</p></details>
        {admin && machine.state !== 'retired' ? <Retire machine={machine} changed={refresh} /> : null}
      </section>)}
    </>} />
    {admin && authority.status === 'ok' && load.status === 'ok' ? <AddMachine person={authority.data.me.person.id} agents={authority.data.people.people.flatMap((person) => person.agents)} changed={refresh} /> : null}
    {authority.status === 'refused' ? <p className="why-not">{authority.refused.refusal.refusal}: {authority.refused.message}</p> : null}
  </div>;
}
