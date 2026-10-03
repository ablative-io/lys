/** The settings form's one button: it saves what the form shows as the next version of the agent's settings, and nothing else. Approving and starting belong to Start on the front page. The request is kept until its answer is confirmed, and the button is never greyed without its reason beside it. */
import { useRef, useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { ComputerAdmission } from '../network/ComputerAdmission';
import type { Machine } from '../network/contract';
import { entries } from '../people/directory';
import { readRoles } from '../roles/AssignedRoles';
import type { Role } from '../roles/contract';
import { changedFrom, profileRequest } from '../runtime/start-requests';
import { Gate } from '../signin/Gate';
import type { ProvisioningProfile } from './Provisioning';

export function SaveSettings({ agent, profile, settings, refusal, canSave, people, machines, saved }: {
  agent: string; profile: ProvisioningProfile | null; settings: Record<string, unknown>; refusal: string; canSave: boolean;
  people: PeopleView; machines: Machine[];
  /** When given, the button saves and then hands on to the start; with nothing changed it hands on at once. */
  saved?: () => void;
}) {
  const load = useLoad(async () => {
    const [me, roles] = await Promise.all([api.me(), readRoles()]);
    return { me, roles };
  }, 'save-settings:' + agent);
  return <Gate load={load} title="Settings" ok={({ me, roles }) => {
    const name = entries(people).find((entry) => entry.id === agent)?.display_name ?? agent;
    return <>
      <Save key={agent} agent={agent} name={name} person={me.person.id} profile={profile} settings={settings} refusal={refusal} canSave={canSave} then={saved} />
      <Admission agent={agent} name={name} person={me.person.id} admin={people.scope === 'directory'} machines={machines} roles={roles.roles} />
    </>;
  }} />;
}

/** Which computers this agent may run on: the one place on the screens where that is changed. */
function Admission({ agent, name, person, admin, machines: initial, roles }: {
  agent: string; name: string; person: string; admin: boolean; machines: Machine[]; roles: Role[];
}) {
  const [machines, setMachines] = useState(initial);
  return <ComputerAdmission agent={agent} name={name} person={person} admin={admin} machines={machines} roles={roles} changed={(machine) => {
    setMachines((current) => current.some((entry) => entry.id === machine.id) ? current.map((entry) => entry.id === machine.id ? machine : entry) : [...current, machine]);
  }} />;
}

/** A save this tab sent and never saw answered: the request itself, and the version it was made from. */
interface KeptSave { path: string; body: Record<string, unknown>; version: number }

function record(value: unknown): value is Record<string, unknown> { return Boolean(value) && typeof value === 'object' && !Array.isArray(value); }

function keptSave(raw: string, path: string): KeptSave {
  const value: unknown = JSON.parse(raw);
  if (!record(value) || value.path !== path || !record(value.body)) throw new Error('PendingSaveUnreadable: The kept save does not name this agent’s settings.');
  const body = value.body;
  if (typeof body.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(body.operation) || typeof body.from_version !== 'number' || !Number.isInteger(body.from_version) || body.from_version < 0) throw new Error('PendingSaveUnreadable: The kept save has no operation and version.');
  return { path, body, version: body.from_version };
}

function Save({ agent, name, person, profile, settings, refusal, canSave, then }: {
  agent: string; name: string; person: string; profile: ProvisioningProfile | null; settings: Record<string, unknown>; refusal: string; canSave: boolean; then?: () => void;
}) {
  const label = then ? 'Save and start ' + name : 'Save these settings';
  const path = '/agents/' + encodeURIComponent(agent) + '/provisioning';
  const key = 'lys.pending.settings-save.' + person + '.' + agent;
  // Start on the front page keeps its own unanswered request here; while one is kept, a new version would be saved under it.
  const starting = sessionStorage.getItem('lys.pending.agent-start.' + person + '.' + agent) !== null;
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(key);
      return { pending: raw === null ? null : keptSave(raw, path), error: '' };
    } catch (error) { return { pending: null, error: error instanceof Error ? error.message : String(error) }; }
  });
  const [pending, setPending] = useState<KeptSave | null>(initial.pending);
  // What is saved now: the profile the page read, then each version this form saved after it.
  const [base, setBase] = useState(profile);
  const [failure, setFailure] = useState('');
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const changed = changedFrom(base, settings);
  /** Why the button cannot be pressed, in the order a person would fix it; empty when it can. */
  const why = initial.error ? 'Lys kept an earlier save for ' + name + ' that it can no longer read.'
    : pending ? ''
    : starting ? 'A start of ' + name + ' has no confirmed answer yet. Press Start on the front page to finish it before changing these settings.'
    : !canSave ? 'An administrator changes these settings.'
    : refusal ? refusal
    : base && !('session' in base) ? 'ProfileReadIncomplete: update Lys before changing these settings; the saved ones were not all returned.'
    : !changed && !then ? 'Nothing has changed.'
    : '';
  const run = async () => {
    if (working.current || why) return;
    // Nothing to save and no save outstanding: the settings stand as they are, so the start goes ahead.
    if (then && !changed && !pending) { then(); return; }
    working.current = true; setBusy(true); setFailure(''); setSaved(false);
    const version = base?.version ?? 0;
    const current: KeptSave = pending ?? { path, body: { ...settings, operation: operationId(), from_version: version }, version };
    try {
      sessionStorage.setItem(key, JSON.stringify(current)); setPending(current);
      // The computer is no part of a save; the request helper reads only the stage, the path, the body and the version.
      const confirmed = await profileRequest(agent, { stage: 'profile', path: current.path, body: current.body, version: current.version, machine: '' });
      sessionStorage.removeItem(key); setPending(null); setBase(confirmed.profile);
      if (then) then(); else setSaved(true);
    } catch (error) {
      // The service answered no: nothing of this request is outstanding. Anything else may have been carried out, so the request stays kept.
      if (error instanceof Refused && error.status >= 400 && error.status < 500) { sessionStorage.removeItem(key); setPending(null); }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  return <form className="save-settings" onSubmit={(event) => { event.preventDefault(); void run(); }}>
    {pending ? <p role="status">The last save has no confirmed answer. Press {label} to send that same request again.</p> : null}
    {failure ? <><p role="alert" className="why-not">Lys could not confirm these settings were saved.</p><details><summary>Details</summary><p>{failure}</p></details></> : null}
    {saved && !failure ? <p role="status">Saved. {name} uses these settings the next time it starts. <a href={'#/team/' + encodeURIComponent(agent)}>Go to {name}</a></p> : null}
    <button type="submit" className="btn primary" disabled={busy || Boolean(why)}>{busy ? 'Saving…' : label}</button>
    {why ? <p className="why-not">{why}{starting && !pending && !initial.error ? <> <a href={'#/team/' + encodeURIComponent(agent)}>Go to {name}</a></> : null}</p> : null}
    {initial.error ? <details><summary>Details</summary><p>{initial.error}</p></details> : null}
  </form>;
}
