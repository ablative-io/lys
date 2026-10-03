/**
 * Why an agent cannot start: one sentence a person can act on, and the one
 * button that fixes it, in place. Nothing is folded away and no reason sends
 * a person to another page to find the act. The refusal's own name stays, at
 * the end and small, for whoever debugs.
 *
 * The caller hands over what refused the start, as the service or the screen
 * named it, and `again`, which sends the same start once more. After a fix
 * here succeeds, `again` is called; the person does not press twice.
 *
 * One word per state everywhere: an agent is Ready, Paused or Retired. An
 * agent that was added and never turned on is "not turned on yet".
 */
import { useRef, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import { allowedToRun, usable } from '../network/allowed';
import type { Machine } from '../network/contract';
import { confirmAdmission } from '../network/machine-admission';
import type { MachineAdmission } from '../network/machine-admission';
import { FolderChooser } from '../provisioning/FolderChooser';
import { saveFolder } from '../provisioning/working-folder';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import { readRoles } from '../roles/AssignedRoles';

/** What refused a start: the refusal's name and its reason, as answered. */
export interface StartRefusal { refusal: string; reason: string }

/** The one act that fixes a refusal, when there is one. */
export type Fix =
  | { kind: 'turn-on'; label: 'Turn on' | 'Turn back on'; transition: 'activate' | 'reinstate' }
  | { kind: 'allow-computer' }
  | { kind: 'choose-folder' }
  | { kind: 'try-again' }
  | { kind: 'nothing' };

/** The refusals that mean no computer is allowed to run the agent. */
const NO_COMPUTER = ['MachineNotForAgent', 'MachineUnavailable'];

/**
 * The sentence and the fix for a refusal. An agent that is not active is
 * refused as `AgentNotActive`, and the service's reason names the state it
 * is in ("the agent is suspended and ..."); that word chooses the sentence.
 */
export function whyNot(name: string, refusal: StartRefusal): { sentence: string; fix: Fix } {
  const state = refusal.refusal === 'AgentNotActive' ? /\bis (registered|suspended|retired)\b/.exec(refusal.reason)?.[1] : undefined;
  if (state === 'registered') return { sentence: name + ' is not turned on yet.', fix: { kind: 'turn-on', label: 'Turn on', transition: 'activate' } };
  if (state === 'suspended') return { sentence: name + ' is paused.', fix: { kind: 'turn-on', label: 'Turn back on', transition: 'reinstate' } };
  if (state === 'retired') return { sentence: name + ' is retired and cannot be started again.', fix: { kind: 'nothing' } };
  if (refusal.refusal === 'WorkingFolderUnnamed') return { sentence: name + ' has no folder to work in.', fix: { kind: 'choose-folder' } };
  if (NO_COMPUTER.includes(refusal.refusal)) return { sentence: 'No computer is allowed to run ' + name + '.', fix: { kind: 'allow-computer' } };
  return { sentence: asSentence(refusal.reason), fix: { kind: 'try-again' } };
}

/** A reason as one sentence: its own name taken off the front, a capital, a full stop. */
function asSentence(reason: string): string {
  const words = reason.replace(/^[A-Z][A-Za-z]+: /, '').trim();
  if (!words) return 'This agent could not be started.';
  const ended = /[.!?]$/.test(words) ? words : words + '.';
  return ended.charAt(0).toUpperCase() + ended.slice(1);
}

const refusalOf = (error: unknown): StartRefusal => error instanceof Refused
  ? { refusal: error.refusal.refusal, reason: error.message }
  : { refusal: 'Unexpected', reason: String(error) };

export function CannotStart({ agent, name, refusal, machines, again }: {
  /** The computers as the start that was refused read them, so this explanation and that start never disagree. */
  agent: string; name: string; refusal: StartRefusal; machines: Machine[]; again: () => void;
}) {
  const { sentence, fix } = whyNot(name, refusal);
  return <div role="alert" className="cannot-start">
    <p>{sentence} <small className="refusal-name">{refusal.refusal}</small></p>
    {fix.kind === 'turn-on' ? <TurnOn agent={agent} name={name} label={fix.label} transition={fix.transition} again={again} /> : null}
    {fix.kind === 'allow-computer' ? <AllowComputer agent={agent} name={name} machines={machines} again={again} /> : null}
    {fix.kind === 'choose-folder' ? <ChooseFolder agent={agent} name={name} machines={machines} again={again} /> : null}
    {fix.kind === 'try-again' ? <button type="button" className="btn primary" onClick={again}>Try again</button> : null}
  </div>;
}

/** What a fix that was itself refused says: one sentence, the name small, the button still there. */
function Failed({ refusal }: { refusal: StartRefusal | null }) {
  return refusal ? <p role="alert">That did not work. {asSentence(refusal.reason)} <small className="refusal-name">{refusal.refusal}</small></p> : null;
}

function TurnOn({ agent, name, label, transition, again }: {
  agent: string; name: string; label: string; transition: 'activate' | 'reinstate'; again: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<StartRefusal | null>(null);
  const working = useRef(false);
  const turnOn = async () => {
    if (working.current) return;
    working.current = true; setBusy(true); setFailed(null);
    try {
      await request<unknown>('/identities/' + encodeURIComponent(agent) + '/transitions', { transition, reason: label + ' from the start screen, to start ' + name });
      again();
    } catch (error) { setFailed(refusalOf(error)); }
    finally { working.current = false; setBusy(false); }
  };
  return <>
    <button type="button" className="btn primary" disabled={busy} onClick={() => { void turnOn(); }}>{label}</button>
    <Failed refusal={failed} />
  </>;
}

/**
 * The folder is a path on one computer, so it is chosen on the computer the
 * start will use: the one the agent's settings name, or else the one
 * computer that is allowed to run it, by name or by a role it holds. When neither settles it, the computer
 * is the thing to settle first, and that is said. The folder is then saved
 * to the agent's settings and the start follows.
 */
function ChooseFolder({ agent, name, machines, again }: { agent: string; name: string; machines: Machine[]; again: () => void }) {
  const load = useLoad(async () => {
    const [held, roles] = await Promise.all([request<ProvisioningAnswer>('/agents/' + encodeURIComponent(agent) + '/provisioning'), readRoles()]);
    const named = usable(machines).find((entry) => entry.id === held.profile?.runs_on);
    // Allowed by its own name or by a role it holds now: the same rule Start uses, so the two never disagree.
    const allowed = allowedToRun(machines, roles.roles, agent);
    return named ?? (allowed.length === 1 ? allowed[0] : null);
  }, 'cannot-start-folder:' + agent);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<StartRefusal | null>(null);
  const working = useRef(false);
  // One save per folder, kept while this is shown, so a second press sends the same one.
  const asked = useRef(new Map<string, string>());
  if (load.status === 'loading') return <p role="status">Looking for a computer…</p>;
  if (load.status === 'refused') return <Failed refusal={{ refusal: load.refused.refusal.refusal, reason: load.refused.message }} />;
  if (!load.data) return <p>Lys does not know yet which computer {name} runs on. Its folder is chosen on that computer, so the computer comes first.</p>;
  const save = async (folder: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setFailed(null);
    try {
      const operation = asked.current.get(folder) ?? operationId();
      asked.current.set(folder, operation);
      await saveFolder(agent, folder, operation);
      again();
    } catch (error) { setFailed(refusalOf(error)); }
    finally { working.current = false; setBusy(false); }
  };
  return <>
    <FolderChooser computers={[load.data]} chosen="" disabled={busy} choose={(folder) => { void save(folder); }} />
    {busy ? <p role="status">Saving the folder…</p> : null}
    <Failed refusal={failed} />
  </>;
}

function AllowComputer({ agent, name, machines, again }: { agent: string; name: string; machines: Machine[]; again: () => void }) {
  const load = useLoad(async () => {
    const me = await api.me();
    return { person: me.person.id, computers: usable(machines) };
  }, 'cannot-start-computers:' + agent);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<StartRefusal | null>(null);
  const working = useRef(false);
  // One request per computer, kept while this is shown, so a second press sends the same one.
  const asked = useRef(new Map<string, MachineAdmission>());
  if (load.status === 'loading') return <p role="status">Looking for a computer…</p>;
  if (load.status === 'refused') return <Failed refusal={{ refusal: load.refused.refusal.refusal, reason: load.refused.message }} />;
  const { person, computers } = load.data;
  const allow = async (machine: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setFailed(null);
    try {
      const body = asked.current.get(machine) ?? { operation: operationId(), agent, allow: true };
      asked.current.set(machine, body);
      const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/agents', body);
      confirmAdmission(answer, machine, body, person);
      again();
    } catch (error) { setFailed(refusalOf(error)); }
    finally { working.current = false; setBusy(false); }
  };
  if (!computers.length) return <>
    <p>No computer has Lys running on it yet.</p>
    <a className="btn primary" href="#/network?add=computer">Add a computer</a>
  </>;
  return <>
    {computers.map((computer) => <button key={computer.id} type="button" className="btn primary" disabled={busy} onClick={() => { void allow(computer.id); }}>
      {computers.length === 1 ? 'Allow on this computer' : 'Allow on ' + computer.name}
    </button>)}
    {computers.length === 1 ? <p className="sec">{name} will be allowed to run on {computers[0].name}.</p> : null}
    <Failed refusal={failed} />
  </>;
}
