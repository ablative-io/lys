/** The settings form's one button: it saves what the form shows as the next version of the agent's settings, and nothing else. Approving and starting belong to Start in the run panel. The request is kept until its answer is confirmed, and the button is never greyed without its reason beside it. On the Overview (`brief`) it reads Save and is shown only once something is changed, or a save is outstanding. */
import { useRef, useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import { answeredNo, sendKept } from '../../kept';
import type { PeopleView } from '../../generated';
import { ComputerAdmission } from '../network/ComputerAdmission';
import type { Machine } from '../network/contract';
import { entries } from '../people/directory';
import { readRoles } from '../roles/AssignedRoles';
import type { Role } from '../roles/contract';
import { changedFrom, profileRequest } from '../runtime/start-requests';
import { Gate } from '../signin/Gate';
import type { ProvisioningProfile } from './Provisioning';

export function SaveSettings({ agent, profile, settings, refusal, canSave, people, machines, saved, brief = false }: {
  agent: string; profile: ProvisioningProfile | null; settings: Record<string, unknown>; refusal: string; canSave: boolean;
  people: PeopleView; machines: Machine[];
  /** Called once a save is confirmed. */
  saved?: () => void;
  /** The Overview's saver: it reads Save and is shown only when something was changed. */
  brief?: boolean;
}) {
  const load = useLoad(async () => {
    const [me, roles] = await Promise.all([api.me(), readRoles()]);
    return { me, roles };
  }, 'save-settings:' + agent);
  return <Gate load={load} title="Settings" ok={({ me, roles }) => {
    const name = entries(people).find((entry) => entry.id === agent)?.display_name ?? agent;
    return <>
      <Save key={agent} agent={agent} name={name} person={me.person.id} profile={profile} settings={settings} refusal={refusal} canSave={canSave} then={saved} brief={brief} />
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

function Save({ agent, name, person, profile, settings, refusal, canSave, then, brief }: {
  agent: string; name: string; person: string; profile: ProvisioningProfile | null; settings: Record<string, unknown>; refusal: string; canSave: boolean; then?: () => void; brief: boolean;
}) {
  // Saving never starts the agent: Start in the run panel is the one start control.
  const label = brief ? 'Save' : 'Save these settings';
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
  // The Overview shows only the model and the computer, so only a change to one of those is a change there.
  const changed = brief && base?.harness
    ? JSON.stringify(settings.model_access) !== JSON.stringify(base.model_access) || (settings.runs_on ?? '') !== (base.runs_on ?? '')
    : changedFrom(base, settings);
  /** Why the button cannot be pressed, in the order a person would fix it; empty when it can. */
  const why = initial.error ? 'Lys kept an earlier save for ' + name + ' that it can no longer read.'
    : pending ? ''
    : starting ? 'A start of ' + name + ' has no confirmed answer yet. Press Start on the front page to finish it before changing these settings.'
    : !canSave ? 'An administrator changes these settings.'
    : refusal ? refusal
    : base && !('session' in base) ? 'ProfileReadIncomplete: update Lys before changing these settings; the saved ones were not all returned.'
    : !changed ? 'Nothing has changed.'
    : '';
  const run = async () => {
    if (working.current || why) return;
    working.current = true; setBusy(true); setFailure(''); setSaved(false);
    const version = base?.version ?? 0;
    const current: KeptSave = pending ?? { path, body: { ...settings, operation: operationId(), from_version: version }, version };
    try {
      const confirmed = await sendKept(key, current, () => {
        setPending(current);
        // The computer is no part of a save; the request helper reads only the stage, the path, the body and the version.
        return profileRequest(agent, { stage: 'profile', path: current.path, body: current.body, version: current.version, machine: '' });
      }, true);
      setPending(null); setBase(confirmed.profile); setSaved(true);
      if (then) then();
    } catch (error) {
      // The service answered no: nothing of this request is outstanding. Anything else may have been carried out, so the request stays kept.
      if (answeredNo(error)) setPending(null);
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  // On the Overview nothing is offered until something is changed, a save is outstanding, or there is something to say.
  if (brief && !changed && !pending && !failure && !saved && !initial.error) return null;
  return <form className="save-settings" onSubmit={(event) => { event.preventDefault(); void run(); }}>
    {pending ? <p role="status">The last save has no confirmed answer. Press {label} to send that same request again.</p> : null}
    {failure ? <><p role="alert" className="why-not">Lys could not confirm these settings were saved.</p><p><small className="refusal-name">{failure}</small></p></> : null}
    {saved && !failure ? <p role="status">Saved. {name} uses these settings the next time it starts. <a href={'#/file/' + encodeURIComponent(agent)}>Go to {name}</a></p> : null}
    {brief && !changed && !pending ? null : <>
    <button type="submit" className="btn primary" disabled={busy || Boolean(why)}>{busy ? 'Saving…' : label}</button>
    {why ? <p className="why-not">{why}{starting && !pending && !initial.error ? <> <a href={'#/file/' + encodeURIComponent(agent)}>Go to {name}</a></> : null}</p> : null}
    </>}
    {initial.error ? <p><small className="refusal-name">{initial.error}</small></p> : null}
  </form>;
}
