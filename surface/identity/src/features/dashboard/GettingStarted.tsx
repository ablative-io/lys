/**
 * Getting started: what a fresh Lys still needs, in the order a person sets it up. Each step says what it is for,
 * is done where it already lives, and leaves the list once it is done. Nothing here is decided for the person:
 * whether agents run on this computer is asked, never assumed, because some computers Lys is installed on should
 * never run agents.
 */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, request } from '../../api';
import { ownAgentIds, recordComputer, useHostName, validComputerName } from '../network/AddMachine';
import { ErrorWords } from '../people/Words';
import { Act } from '../../shell/Act';
import type { NetworkView } from '../network/contract';

interface AppsView { apps: { id: string; state: string }[] }
interface Standing { machines: number; cambium: string | null; read: boolean }


async function readStanding(): Promise<Standing> {
  const [network, apps] = await Promise.all([
    request<NetworkView>('/network').catch(() => null),
    request<AppsView>('/apps').catch(() => null),
  ]);
  const machines = network ? network.machines.filter((machine) => machine.state !== 'retired').length : 0;
  const cambium = apps?.apps.find((app) => app.id === 'cambium')?.state ?? null;
  return { machines, cambium, read: network !== null && apps !== null };
}

/** The question about this computer: a name, and Yes; or Not this one, which leaves agents to other computers. */
function ThisComputer({ person, done }: { person: string; done: () => void }) {
  const [busy, setBusy] = useState(false);
  const [invalid, setInvalid] = useState('');
  const [failure, setFailure] = useState<unknown>(null);
  const host = useHostName();
  const [declined, setDeclined] = useState(false);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = String(new FormData(event.currentTarget).get('name') ?? '').trim();
    if (!validComputerName(name)) { setInvalid('Give this computer a name, for example "Tom’s MacBook".'); return; }
    setBusy(true); setInvalid(''); setFailure(null);
    try {
      // Your own agents may run here: those you have now are named, and each one you add later is allowed where you run it.
      const mayRun = await ownAgentIds(person);
      await recordComputer({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: mayRun, may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null }, person, () => undefined);
      done();
    } catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  if (declined) return <p className="dim">Agents won’t run on this computer. Add another computer under <a href="#/network">Network</a> when you’re ready.</p>;
  return <form onSubmit={(event) => void submit(event)} aria-label="Run agents on this computer">
    <p>Lys is running on this computer. Should your agents run here too? Only say yes if this is a computer you want agents to work on.</p>
    <label className="field">This computer’s name<input key={host} name="name" defaultValue={host} placeholder="For example, Tom’s MacBook" autoComplete="off" disabled={busy} /></label>
    <div className="chain">
      <Act symbol="add" name="Yes, run agents here" word={busy ? 'Adding…' : 'Yes, run agents here'} tone="primary" type="submit" disabled={busy} />
      <Act symbol="close" name="Not this one" word="Not this one" disabled={busy} onClick={() => setDeclined(true)} />
    </div>
    {invalid ? <p role="alert" className="why-not">{invalid}</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
  </form>;
}

/** The list itself; it is shown on the dashboard until every step is done. */
export function GettingStarted({ person, agents, standing, reread }: { person: string; agents: number; standing: Standing; reread: () => void }) {
  if (!standing.read) return null;
  const steps = [
    { key: 'computer', done: standing.machines > 0, title: 'Choose where agents run',
      body: <ThisComputer person={person} done={reread} /> },
    { key: 'google', done: false, optional: true, title: 'Let people sign in with Google',
      body: <p>So nobody needs a Lys password: add Google under <a href="#/connections">Configuration → services and sign-in providers</a>.</p> },
    { key: 'cambium', done: standing.cambium === 'approved', title: 'Connect Cambium',
      body: <p>{standing.cambium === 'pending' ? 'Cambium has asked to connect. Review what it asks for and approve it on ' : 'Connect your team’s chat to Lys on '}<a href="#/apps">Apps</a>.</p> },
    { key: 'agent', done: agents > 0, title: 'Add your first agent',
      body: <p>Choose Claude Code or Codex, a folder and a computer: <a href="#/agents/new">Add an agent</a>.</p> },
  ];
  const open = steps.filter((step) => !step.done);
  if (open.every((step) => step.optional)) return null;
  return <section className="you-panel getting-started" aria-label="Getting started">
    <div className="section-h"><span>Getting started</span><span className="dim">{steps.length - open.length} of {steps.length} done</span></div>
    <ol>{open.map((step) => <li key={step.key}><b>{step.title}</b>{step.optional ? <span className="dim"> (optional)</span> : null}{step.body}</li>)}</ol>
  </section>;
}

export { readStanding };
export type { Standing };
