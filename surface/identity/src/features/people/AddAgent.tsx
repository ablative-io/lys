/** Add an agent: one short form, then straight to the agent that was made. */
import { useMemo, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { useNavigate } from 'react-router';
import { api, Refused, useLoad } from '../../api';
import type { MeView, PeopleView } from '../../generated';
import { DirectoryGate as Gate } from './Words';
import { addAgent, newAgentRequest, readAgentRequest } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';

function refusalWords(problem: unknown): string {
  if (problem instanceof Refused) {
    if (problem.refusal.refusal === 'ServiceUnreachable') return 'Lys could not be reached. The outcome is not known. Your original request is saved; try the same request again.';
    if (problem.refusal.refusal === 'NotSignedIn') return 'You have been signed out. Sign in again, then add the agent.';
    return problem.refusal.refusal + ': ' + problem.refusal.reason;
  }
  return String(problem);
}

function Form({ me, people }: { me: MeView; people: PeopleView }) {
  const navigate = useNavigate();
  const key = 'lys.add-agent.' + me.person.id;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-agent') !== null) throw new Error('An earlier registration still needs its outcome checked in Manage directory before adding another agent.');
      return { pending: readAgentRequest(key), error: '' };
    } catch (error) { return { pending: null, error: refusalWords(error) }; }
  });
  const [name, setName] = useState(saved.pending?.name ?? '');
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState(saved.error);
  // A retry after an unclear answer repeats the same operations, so nothing is added twice.
  const [pending, setPending] = useState(saved.pending);
  const working = useRef(false);
  const names = useMemo(() => new Map((people.people.find((person) => person.id === me.person.id)?.agents ?? []).map((agent) => [agent.display_name.trim().toLowerCase(), agent.display_name])), [people, me.person.id]);
  const taken = pending ? undefined : names.get(name.trim().toLowerCase());
  const keep = (next: PendingAgent) => { sessionStorage.setItem(key, JSON.stringify(next)); setPending(next); };
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (working.current || saved.error) return;
    const wanted = name.trim();
    if (!wanted) { setRefusal('Give the agent a name.'); return; }
    if (taken) { setRefusal('There is already an agent called ' + taken + '. Choose another name.'); return; }
    working.current = true;
    setSending(true);
    setRefusal('');
    try {
      const agent = await addAgent(pending ?? newAgentRequest(wanted), me.person.id, keep);
      sessionStorage.removeItem(key);
      navigate('/file/' + encodeURIComponent(agent), { replace: true });
    } catch (problem) {
      setRefusal(refusalWords(problem));
    } finally { working.current = false; setSending(false); }
  };
  return <form className="add-agent" aria-label="Register an agent" onSubmit={(event) => { void submit(event); }}>
    <label className="field">Name
      <input name="display_name" value={name} autoFocus autoComplete="off" disabled={sending || pending !== null || Boolean(saved.error)} onChange={(event) => { setName(event.target.value); setRefusal(''); }} aria-invalid={taken || refusal ? true : undefined} aria-describedby="add-agent-refusal" />
    </label>
    {taken && !refusal ? <p className="why-not" id="add-agent-refusal" role="alert">There is already an agent called {taken}. Choose another name.</p> : null}
    {refusal ? <p className="why-not" id="add-agent-refusal" role="alert">{refusal}</p> : null}
    <div className="field">Answers to
      <p>You ({me.person.display_name})</p>
    </div>
    {pending ? <p role="status">{pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    <p><button className="btn primary" type="submit" disabled={sending || !name.trim() || Boolean(taken) || Boolean(saved.error)}>{sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'}</button></p>
  </form>;
}

export function AddAgent() {
  const load = useLoad(async () => {
    const [me, people] = await Promise.all([api.me(), api.people()]);
    return { me, people };
  }, 'add-agent');
  return <div className="page">
    <h1>Add an agent</h1>
    <Gate load={load} title="Add an agent" ok={(data) => data.people.scope === 'directory' ? <Form me={data.me} people={data.people} /> : <p className="why-not">Only the Lys administrator can add agents. Ask them to add it for you.</p>} />
  </div>;
}
