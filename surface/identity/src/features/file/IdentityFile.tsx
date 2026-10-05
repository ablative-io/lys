import { useState } from 'react';
import { useParams } from 'react-router';
import { Refused, api, useLoad } from '../../api';
import { kindOf } from '../../generated';
import type { AgentSummary, AgentView, ReceiptAnswer } from '../../generated';
import { EmergencyStop, StopReceipt } from './EmergencyStop';
import type { StopAnswer } from './EmergencyStop';
import { DirectoryGate as Gate, ErrorWords, ACTION, ACTION_SHORT, STATUS } from '../people/Words';
import type { Entry } from '../people/directory';
import { Pill } from '../people/Pill';
import { TabBody } from './sections';
import { ACTIONS, MERGED, tabsFor } from './tabs';
import { RecordedForm } from '../people/RecordedForm';
import { day } from './time';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { AgentOverview } from './AgentOverview';
import { HeadMenu } from './HeadMenu';
import { AssignedRoles } from '../roles/AssignedRoles';
import './agent-overview.css';
import { Act } from '../../shell/Act';

/** Everything a file shows, all of it read from the service. */
export interface FileData {
  x: Entry;
  /** The agent's own view; null for a person, whom no single route answers. */
  agent: AgentView | null;
  /** A person's agents, from the people view. */
  agents: AgentSummary[];
  /** The receipt of every event about an agent, oldest first. */
  receipts: ReceiptAnswer[];
  /** The grants the caller can see, and whom they name. */
  grants: GrantWorld;
}

async function readAgent(id: string): Promise<FileData> {
  const agent = await api.agent(id);
  const grants = await readGrantWorld();
  const x: Entry = { id: agent.id, display_name: agent.display_name, state: agent.state, kind: 'agent', role: agent.role, person: agent.person };
  return { x, agent, agents: [], receipts: [], grants };
}

async function readPerson(id: string): Promise<FileData> {
  const [view, grants] = await Promise.all([api.people(), readGrantWorld()]);
  const person = view.people.find((p) => p.id === id);
  if (!person) {
    throw new Refused(404, { refusal: 'PersonNotVisible', reason: 'this person is not among the records you may see' });
  }
  const x: Entry = { id: person.id, display_name: person.display_name, state: person.state, kind: 'person', role: null, person: null };
  return { x, agent: null, agents: person.agents, receipts: [], grants };
}

const readFile = (id: string): Promise<FileData> => (kindOf(id) === 'agent' ? readAgent(id) : readPerson(id));

function AgentEvidence({ data, tab, reload }: { data: FileData; tab: string; reload: () => void }) {
  const load = useLoad(async () => {
    const receipts = await Promise.all((data.agent?.provenance.events ?? []).map((index) => api.receipt(index)));
    return { ...data, receipts };
  }, 'agent-evidence:' + data.x.id + ':' + (data.agent?.provenance.events.join(',') ?? ''));
  return <Gate load={load} title="recorded changes" ok={(answered) => <>
    <TabBody tab={tab} data={answered} reload={reload} />
  </>} />;
}

/** Why parts of an agent's page could not be read, said in place. */
function ReadProblems({ problems }: { problems: Refused[] }) {
  return problems.length ? <section aria-label="Read problems"><h2>What could not be read</h2>{problems.map((problem, index) => <p key={index}><code>{problem.refusal.refusal}</code>: {problem.message}</p>)}</section> : null;
}

function File({ data, tab, reload, stop, stopped, notice, note }: { data: FileData; tab: string; reload: () => void; stop: StopAnswer | null; stopped: (answer: StopAnswer) => void;
  /** What the last change in the head did, kept while the file is read again. */
  notice: string; note: (words: string) => void }) {
  const { agent } = data;
  const x = stop?.agent === data.x.id && stop.state === 'suspended' ? { ...data.x, state: 'suspended' as const } : data.x;
  const kind = x.kind;
  const person = x.person;
  const since = agent?.provenance.registration?.actor.authenticated_at;
  /** What is being changed in the head: the name, or one lifecycle step. Nothing is sent until its form is. */
  const [changing, setChanging] = useState<string | null>(null);
  const counts: Record<string, number | undefined> = {
    record: agent ? agent.provenance.events.length : undefined,
    access: data.grants.list.grants.filter((g) => g.holder === x.id).length,
  };
  return (
    <div className="page fill">
      {/* One flat look for a person and an agent: the name, no raw id under it, no folder tab. */}
      <div className={'file flat-file' + (agent ? ' agent-page' : ' person-page')}>
        <div className="head">
          <div>
            <h1>{x.display_name}</h1>
            {agent ? (x.state === 'active' ? null : <p className="sec">{x.state === 'registered' ? 'This agent still needs to be switched on before it can start.' : x.state === 'suspended' ? 'This agent’s access is suspended. A new start needs it to be reinstated.' : 'This agent is retired.'}</p>)
              : person ? <p className="sec">Answers to <Pill x={person} /></p> : null}
          </div>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            {/* The head holds the name, the stop, the state and one menu, and nothing else (Tom, 5 October 2026). */}
            {ACTIONS[x.state].includes('activate') ? <Act symbol="start" name={ACTION.activate} word={ACTION_SHORT.activate} tone="primary" data-act="activate" aria-pressed={changing === 'activate'} onClick={() => setChanging(changing === 'activate' ? null : 'activate')} /> : null}
            {kind === 'agent' && (x.state === 'active' || x.state === 'suspended') ? (
              <EmergencyStop id={x.id} active={x.state === 'active'} stopped={stopped} />
            ) : null}
            <span className={'state ' + x.state} id="state">{stop?.agent === x.id ? 'Suspended in this stop answer' : STATUS[x.state]}</span>
            <HeadMenu chosen={changing} choose={(act) => setChanging(changing === act ? null : act)}
              items={[...(agent ? [] : [{ act: 'name', label: 'Edit name', mark: 'rename' }]), ...ACTIONS[x.state].filter((a) => a !== 'activate').map((a) => ({ act: a, label: ACTION_SHORT[a], danger: a === 'suspend' || a === 'retire' }))]} />
          </div>
        </div>
        {changing === 'name' ? <RecordedForm name="change-profile" title="Save name" heading="Edit name" description="Change the display name while keeping the same identity and audit history." done={reload}
          success={(asked) => { const words = 'Name saved: ' + String(asked.body.display_name) + '.'; note(words); return words; }}
          change={(form) => ({ path: '/identities/' + encodeURIComponent(x.id) + '/profile', body: { display_name: String(form.get('display_name') ?? '').trim() } })}>
          <label className="field">Display name<input name="display_name" required defaultValue={x.display_name} /></label>
        </RecordedForm> : null}
        {changing && changing !== 'name' ? <RecordedForm key={changing} name="lifecycle" title="Record lifecycle change" heading={ACTION[changing]} description={changing === 'retire' ? 'Retiring stops its access. Retirement is permanent.' : changing === 'suspend' ? 'Suspending stops its access until it is restored.' : undefined} submitLabel={ACTION[changing]} drawn={{ symbol: changing === 'retire' ? 'retire' : changing === 'suspend' ? 'suspend' : 'start', word: ACTION_SHORT[changing], tone: changing === 'retire' || changing === 'suspend' ? 'danger' : 'primary' }} done={reload}
          success={(asked) => { const now: Record<string, string> = { activate: 'active', suspend: 'suspended', retire: 'retired', reinstate: 'active', resume: 'active' }; const words = x.display_name + ' is now ' + (now[String(asked.body.transition)] ?? 'updated') + '.'; note(words); return words; }}
          change={(form) => ({ path: '/identities/' + encodeURIComponent(x.id) + '/transitions', body: { transition: changing, reason: String(form.get('reason') ?? '').trim() } })}>
          <label className="field">Reason<input name="reason" required /></label>
        </RecordedForm> : null}
        {notice ? <p role="status" className="note">{notice}</p> : null}
        {stop && stop.agent === x.id ? <StopReceipt answer={stop} /> : null}
        <nav className="tabs" aria-label={agent ? 'Agent sections' : 'Sections'}>
          {tabsFor(x.id).map(([k, l]) => (
            <a key={k} href={`#/file/${x.id}/${k}`} className={tab === k ? 'on' : ''} aria-current={tab === k ? 'page' : undefined}>
              {l}
              {counts[k] !== undefined ? <span className="n">{counts[k]}</span> : null}
            </a>
          ))}
        </nav>
        <div className="pane">
          {agent && tab === 'profile' ? <AgentOverview key={x.id} agent={agent} added={since ? day(since) : undefined} rename={() => setChanging(changing === 'name' ? null : 'name')} details={(problems) => <><ReadProblems problems={problems} /><AssignedRoles id={x.id} /></>} />
            : agent && tab === 'record' ? <AgentEvidence data={{ ...data, x }} tab="record" reload={reload} />
            : <TabBody tab={tab} data={{ ...data, x }} reload={reload} />}
        </div>
      </div>
    </div>
  );
}

export function IdentityFile() {
  const { id = '', tab: asked = 'profile' } = useParams();
  // An address naming no section of this file opens its Overview, never a section of its own.
  const merged = MERGED[asked] ?? asked;
  const tab = tabsFor(id).some(([key]) => key === merged) ? merged : 'profile';
  const [version, setVersion] = useState(0);
  const [stop, setStop] = useState<StopAnswer | null>(null);
  const [notice, setNotice] = useState('');
  const load = useLoad(() => readFile(id), id + '#' + version);
  if (load.status === 'refused' && load.refused.status === 404 && load.refused.refusal.refusal !== 'Unanswered') {
    return (
      <div className="page">
        <h1>Not found</h1>
        <p className="sub">This person or agent is not in the directory, or is not among the records you can see. Ask the administrator to check your access.</p>
        <ErrorWords problem={load.refused} /><p className="note mono">{id}</p>
      </div>
    );
  }
  return <Gate load={load} title="this person or agent’s file" ok={(data) => <File notice={notice} note={setNotice} data={data} tab={tab} reload={() => setVersion((v) => v + 1)} stop={stop} stopped={(answer) => { setStop(answer); }} />} />;
}
