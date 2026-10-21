/**
 * Getting started: what a fresh Lys still needs, in the order a person sets it up. Each step says what it is for,
 * is done where it already lives, and leaves the list once it is done. Nothing here is decided for the person:
 * whether agents run on the computer Lys runs on is asked, never assumed, because some computers Lys is installed on should
 * never run agents. "Not this one" is kept by Lys for the person (`PUT /me/getting-started`, read back on `/me`), so it
 * is not asked again from another browser; an answer a browser kept before is moved to Lys once and then removed.
 */
import { useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, operationId, request } from '../../api';
import { ownAgentIds, recordComputer, useHostName, validComputerName } from '../network/AddMachine';
import { ErrorWords } from '../people/Words';
import { Act } from '../../shell/Act';
import type { NetworkView } from '../network/contract';
import type { ProvidersView } from '../connections/SignInProviders';

interface AppsView { apps: { id: string; name?: string; state: string }[] }
/**
 * What Getting started reads. `google` is true only when Google is set up and switched on, as Settings → Services
 * and sign-in providers shows it. Only administrators read or see that step.
 */
interface Standing { machines: number;
  /** 'approved' when any app is approved, else 'pending' when any has asked to connect, else null. */ app: 'approved' | 'pending' | null;
  /** The first app that has asked to connect, by its registered name (its id when nameless), while app is pending. */ asking: string | null;
  /** Whether `/apps` answered: Connect an app is offered only then. */ apps: boolean; google: boolean; read: boolean; administrator: boolean;
  /** Whether the person said the Lys computer is not for agents, as Lys keeps it. */ declined: boolean }

/** Where a browser kept "Not this one" before Lys kept it; read once, moved to Lys, then removed. */
const declinedKey = (person: string) => 'lys.getting-started.not-this-computer.' + person;

/** Keep "Not this one" for the person with Lys. */
const keepDeclined = (): Promise<unknown> => request('/me/getting-started', { host_declined: true }, 'PUT');

/**
 * Whether the person declined the Lys computer, from `/me`. An answer this browser kept before is moved to Lys once and
 * removed; while Lys will not take it, could not read it, or an older Lys says nothing, the browser's answer is still
 * honoured and kept.
 */
async function readDeclined(): Promise<boolean> {
  const me = await api.me();
  const kept = me.getting_started && 'host_declined' in me.getting_started ? me.getting_started : undefined;
  let local = false;
  try { local = localStorage.getItem(declinedKey(me.person.id)) === 'yes'; } catch { /* A browser that keeps nothing has nothing to move. */ }
  if (!local || !kept) return kept?.host_declined === true || local;
  if (!kept.host_declined) {
    try { await keepDeclined(); } catch { return true; }
  }
  try { localStorage.removeItem(declinedKey(me.person.id)); } catch { /* Lys has it now; a key the browser will not remove asks nothing again. */ }
  return true;
}

async function readStanding(): Promise<Standing> {
  const [people, declined] = await Promise.all([api.people(), readDeclined()]);
  const administrator = people.scope === 'directory';
  const [network, apps, providers] = await Promise.all([
    request<NetworkView>('/network').catch(() => null),
    request<AppsView>('/apps').catch(() => null),
    administrator ? request<ProvidersView>('/sign-in-providers').catch(() => null) : null,
  ]);
  const machines = network ? network.machines.filter((machine) => machine.state !== 'retired'
    && (administrator || machine.named_by === people.people[0]?.id)).length : 0;
  const listed = apps?.apps ?? [];
  const pending = listed.find((app) => app.state === 'pending');
  const app = listed.some((entry) => entry.state === 'approved') ? 'approved' : pending ? 'pending' : null;
  const asking = app === 'pending' && pending ? (pending.name?.trim() ? pending.name : pending.id) : null;
  const google = providers?.providers.some((provider) => provider.provider === 'google' && provider.enabled) ?? false;
  return { machines, app, asking, apps: apps !== null, google, read: network !== null, administrator, declined };
}

/** The question about the computer Lys runs on, named by its own name: a name, and Yes; or Not this one, which leaves agents to other computers. */
function ThisComputer({ person, done, decline }: { person: string; done: () => void; decline: () => void }) {
  const [busy, setBusy] = useState(false);
  const [invalid, setInvalid] = useState('');
  const [failure, setFailure] = useState<unknown>(null);
  const host = useHostName();
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = String(new FormData(event.currentTarget).get('name') ?? '').trim();
    if (!validComputerName(name)) { setInvalid('Give that computer a name, for example "My MacBook".'); return; }
    setBusy(true); setInvalid(''); setFailure(null);
    try {
      // Your own agents may run here: those you have now are named, and each one you add later is allowed where you run it.
      const mayRun = await ownAgentIds(person);
      await recordComputer({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: mayRun, may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null }, person, () => undefined);
      done();
    } catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  return <form onSubmit={(event) => void submit(event)} aria-label="Run agents on the Lys computer">
    <p>Lys runs on {host || 'the computer it was installed on'}. Should your agents run there too? Only say yes if that is a computer you want agents to work on.</p>
    <label className="field">That computer’s name<input key={host} name="name" defaultValue={host} placeholder="For example, My MacBook" autoComplete="off" disabled={busy} /></label>
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
  const [declinedHere, setDeclined] = useState(false);
  const declined = standing.declined || declinedHere;
  // What this view says after Not this one: kept by Lys, or not kept.
  const [answered, setAnswered] = useState<'kept' | 'unkept' | null>(null);
  if (!standing.read) return null;
  const decline = () => {
    setDeclined(true);
    keepDeclined().then(() => setAnswered('kept'), () => setAnswered('unkept'));
  };
  const steps: { key: string; done: boolean; title: string; body: ReactNode; optional?: boolean }[] = [
    { key: 'computer', done: standing.machines > 0 || (standing.administrator && declined),
      title: standing.administrator ? 'Choose where agents run' : 'Add your own computer',
      body: standing.administrator ? <ThisComputer person={person} done={reread} decline={decline} />
        : <p>Open <a href="#/network">Computers</a>, choose Another computer, then download its installer and open it on your computer.</p> },
    ...(standing.administrator ? [{ key: 'google', done: standing.google, optional: true, title: 'Let people sign in with Google',
      body: <p>So nobody needs a Lys password: add Google under <a href="#/connections">Settings → services and sign-in providers</a>.</p> }] : []),
    // Offered only when Apps could be read; an administrator connects it there, and anyone else is told who does.
    ...(standing.apps ? [{ key: 'app', done: standing.app === 'approved', title: 'Connect an app',
      body: !standing.administrator ? <p>No app is connected to Lys yet. An administrator connects one on Apps; once it is, this step is done.</p>
        : <p>{standing.asking ? standing.asking + ' has asked to connect. Review what it asks for and approve it on ' : 'Connect an app to Lys on '}<a href="#/apps">Apps</a>.</p> }] : []),
    { key: 'agent', done: agents > 0, title: 'Add your first agent',
      body: <p>Choose Claude Code or Codex, a folder and a computer: <a href="#/agents/new">Add an agent</a>.</p> },
  ];
  const open = steps.filter((step) => !step.done);
  if (open.every((step) => step.optional) && answered === null) return null;
  return <section className="you-panel getting-started" aria-label="Getting started">
    <div className="section-h"><span>Getting started</span><span className="dim">{steps.length - open.length} of {steps.length} done</span></div>
    {answered ? <p className={answered === 'kept' ? 'dim' : 'why-not'} role="status">
      Agents won’t run on the computer Lys runs on. Add another computer under <a href="#/network">Computers</a> when you’re ready.
      {answered === 'unkept' ? ' Lys could not keep the answer, so it will ask again next time.' : null}
    </p> : null}
    {open.some((step) => !step.optional) ? <ol>{open.map((step) => <li key={step.key}><b>{step.title}</b>{step.optional ? <span className="dim"> (optional)</span> : null}{step.body}</li>)}</ol> : null}
  </section>;
}

export { readStanding };
export type { Standing };
