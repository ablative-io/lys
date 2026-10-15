/**
 * Getting started: what a fresh Lys still needs, in the order a person sets it up. Each step says what it is for,
 * is done where it already lives, and leaves the list once it is done. Nothing here is decided for the person:
 * whether agents run on this computer is asked, never assumed, because some computers Lys is installed on should
 * never run agents.
 */
import { useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, operationId, request } from '../../api';
import { ownAgentIds, recordComputer, useHostName, validComputerName } from '../network/AddMachine';
import { ErrorWords } from '../people/Words';
import { Act } from '../../shell/Act';
import type { NetworkView } from '../network/contract';
import type { ProvidersView } from '../connections/SignInProviders';

interface AppsView { apps: { id: string; state: string }[] }
/**
 * What Getting started reads. `google` is true only when Google is set up and switched on, as Settings → Services
 * and sign-in providers shows it. Only administrators read or see that step.
 */
interface Standing { machines: number; cambium: string | null; google: boolean; read: boolean; administrator: boolean }

async function readStanding(): Promise<Standing> {
  const people = await api.people();
  const administrator = people.scope === 'directory';
  const [network, apps, providers] = await Promise.all([
    request<NetworkView>('/network').catch(() => null),
    request<AppsView>('/apps').catch(() => null),
    administrator ? request<ProvidersView>('/sign-in-providers').catch(() => null) : null,
  ]);
  const machines = network ? network.machines.filter((machine) => machine.state !== 'retired'
    && (administrator || machine.named_by === people.people[0]?.id)).length : 0;
  const cambium = apps?.apps.find((app) => app.id === 'cambium')?.state ?? null;
  const google = providers?.providers.some((provider) => provider.provider === 'google' && provider.enabled) ?? false;
  return { machines, cambium, google, read: network !== null && apps !== null, administrator };
}

/** "Not this one" is kept for the person on this browser, so Getting started can finish; the key names the person. */
const declinedKey = (person: string) => 'lys.getting-started.not-this-computer.' + person;

function readDeclined(person: string): boolean {
  try { return localStorage.getItem(declinedKey(person)) === 'yes'; }
  catch { return false; }
}

/** Keeps the answer; false when this browser refuses to keep it (storage off or full), so the page can say so. */
function keepDeclined(person: string): boolean {
  try { localStorage.setItem(declinedKey(person), 'yes'); return true; }
  catch { return false; }
}

/** The question about this computer: a name, and Yes; or Not this one, which leaves agents to other computers. */
function ThisComputer({ person, done, decline }: { person: string; done: () => void; decline: () => void }) {
  const [busy, setBusy] = useState(false);
  const [invalid, setInvalid] = useState('');
  const [failure, setFailure] = useState<unknown>(null);
  const host = useHostName();
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = String(new FormData(event.currentTarget).get('name') ?? '').trim();
    if (!validComputerName(name)) { setInvalid('Give this computer a name, for example "My MacBook".'); return; }
    setBusy(true); setInvalid(''); setFailure(null);
    try {
      // Your own agents may run here: those you have now are named, and each one you add later is allowed where you run it.
      const mayRun = await ownAgentIds(person);
      await recordComputer({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: mayRun, may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null }, person, () => undefined);
      done();
    } catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  return <form onSubmit={(event) => void submit(event)} aria-label="Run agents on this computer">
    <p>Lys is running on this computer. Should your agents run here too? Only say yes if this is a computer you want agents to work on.</p>
    <label className="field">This computer’s name<input key={host} name="name" defaultValue={host} placeholder="For example, My MacBook" autoComplete="off" disabled={busy} /></label>
    <div className="chain">
      <Act symbol="add" name="Yes, run agents here" word={busy ? 'Adding…' : 'Yes, run agents here'} tone="primary" type="submit" disabled={busy} />
      <Act symbol="close" name="Not this one" word="Not this one" disabled={busy} onClick={decline} />
    </div>
    {invalid ? <p role="alert" className="why-not">{invalid}</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
  </form>;
}

/** The list itself; it is shown on the dashboard until every step is done. */
export function GettingStarted({ person, agents, standing, reread }: { person: string; agents: number; standing: Standing; reread: () => void }) {
  const [declined, setDeclined] = useState(() => readDeclined(person));
  // What this view says after Not this one: kept, or not kept by this browser.
  const [answered, setAnswered] = useState<'kept' | 'unkept' | null>(null);
  if (!standing.read) return null;
  const decline = () => { setAnswered(keepDeclined(person) ? 'kept' : 'unkept'); setDeclined(true); };
  const steps: { key: string; done: boolean; title: string; body: ReactNode; optional?: boolean }[] = [
    { key: 'computer', done: standing.machines > 0 || (standing.administrator && declined),
      title: standing.administrator ? 'Choose where agents run' : 'Add your own computer',
      body: standing.administrator ? <ThisComputer person={person} done={reread} decline={decline} />
        : <p>Open <a href="#/network">Network</a>, choose Another computer, then download its installer and open it on your computer.</p> },
    ...(standing.administrator ? [{ key: 'google', done: standing.google, optional: true, title: 'Let people sign in with Google',
      body: <p>So nobody needs a Lys password: add Google under <a href="#/connections">Settings → services and sign-in providers</a>.</p> }] : []),
    { key: 'cambium', done: standing.cambium === 'approved', title: 'Connect Cambium',
      body: <p>{standing.cambium === 'pending' ? 'Cambium has asked to connect. Review what it asks for and approve it on ' : 'Connect your team’s chat to Lys on '}<a href="#/apps">Apps</a>.</p> },
    { key: 'agent', done: agents > 0, title: 'Add your first agent',
      body: <p>Choose Claude Code or Codex, a folder and a computer: <a href="#/agents/new">Add an agent</a>.</p> },
  ];
  const open = steps.filter((step) => !step.done);
  if (open.every((step) => step.optional) && answered === null) return null;
  return <section className="you-panel getting-started" aria-label="Getting started">
    <div className="section-h"><span>Getting started</span><span className="dim">{steps.length - open.length} of {steps.length} done</span></div>
    {answered ? <p className={answered === 'kept' ? 'dim' : 'why-not'} role="status">
      Agents won’t run on this computer. Add another computer under <a href="#/network">Network</a> when you’re ready.
      {answered === 'unkept' ? ' This browser would not keep the answer, so Lys will ask again next time.' : null}
    </p> : null}
    {open.some((step) => !step.optional) ? <ol>{open.map((step) => <li key={step.key}><b>{step.title}</b>{step.optional ? <span className="dim"> (optional)</span> : null}{step.body}</li>)}</ol> : null}
  </section>;
}

export { readStanding };
export type { Standing };
