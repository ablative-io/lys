import { useState } from 'react';
import { useParams } from 'react-router';
import { Refused, api, useLoad } from '../../api';
import { kindOf } from '../../generated';
import type { AgentSummary, AgentView, ReceiptAnswer } from '../../generated';
import { EmergencyStop, StopReceipt } from './EmergencyStop';
import type { StopAnswer } from './EmergencyStop';
import { DirectoryGate as Gate, ErrorWords, ACTION, STATUS } from '../people/Words';
import { fileNo } from '../people/directory';
import type { Entry } from '../people/directory';
import { Pill } from '../people/Pill';
import { TabBody } from './sections';
import { ACTIONS, TABS } from './tabs';
import { day } from './time';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { AgentOverview } from './AgentOverview';

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
    {tab === 'profile' ? <TabBody tab="record" data={answered} reload={reload} /> : null}
  </>} />;
}

function FileTabs({ data, tab }: { data: FileData; tab: string }) {
  return <nav className="tabs" aria-label="Agent sections">{TABS.map(([key, label]) => <a key={key} href={`#/file/${data.x.id}/${key}`} className={tab === key ? 'on' : ''}>
    {label}{key === 'record' && data.agent ? <span className="n">{data.agent.provenance.events.length}</span> : key === 'access' ? <span className="n">{data.grants.list.grants.filter((grant) => grant.holder === data.x.id).length}</span> : null}
  </a>)}</nav>;
}

function AgentDetails({ data, tab, reload, problems = [] }: { data: FileData; tab: string; reload: () => void; problems?: Refused[] }) {
  const [opened, setOpened] = useState(tab === 'record');
  const x = data.x;
  return <details className="agent-details" open={tab === 'record' || undefined} onToggle={(event) => { if (event.currentTarget.open) setOpened(true); }}>
    <summary>Details</summary>
    {problems.length ? <section aria-label="Read problems"><h2>What could not be read</h2>{problems.map((problem, index) => <p key={index}><code>{problem.refusal.refusal}</code>: {problem.message}</p>)}</section> : null}
    <p className="identity-id">Identity: <code>{x.id}</code></p>
    <p>Access status: <span className={'state ' + x.state} id="state">{STATUS[x.state]}</span>. This says whether its identity may be used; it does not say a process is running.</p>
    {data.agent?.provenance.registration ? <p>Registered since {day(data.agent.provenance.registration.actor.authenticated_at)}.</p> : null}
    <div className="agent-detail-acts">
      <a className="btn" href={'#/directory/manage?action=profile&identity=' + encodeURIComponent(x.id)}>Edit name</a>
      {ACTIONS[x.state].map((action) => <a key={action} className={'btn ' + (action === 'suspend' || action === 'retire' ? 'danger' : '')} data-act={action} href={'#/directory/manage?action=status&identity=' + encodeURIComponent(x.id)}>{ACTION[action]}</a>)}
      <a className="btn" data-act="canvas" href={'#/canvas/' + encodeURIComponent(x.id)}>Open in the canvas</a>
    </div>
    <FileTabs data={data} tab={tab} />
    {opened ? <AgentEvidence data={data} tab={tab === 'record' ? 'record' : 'profile'} reload={reload} /> : null}
  </details>;
}

function File({ data, tab, reload, stop, stopped }: { data: FileData; tab: string; reload: () => void; stop: StopAnswer | null; stopped: (answer: StopAnswer) => void }) {
  const { agent } = data;
  const x = stop?.agent === data.x.id && stop.state === 'suspended' ? { ...data.x, state: 'suspended' as const } : data.x;
  const kind = x.kind;
  const person = x.person;
  const since = agent?.provenance.registration?.actor.authenticated_at;
  const counts: Record<string, number | undefined> = {
    record: agent ? agent.provenance.events.length : undefined,
    access: data.grants.list.grants.filter((g) => g.holder === x.id).length,
  };
  return (
    <div className="page fill">
      <div className="eyebrow">
        <a href="#/people">People and agents</a> / {x.display_name}
      </div>
      <div className={'file' + (agent ? ' agent-page' : '')} data-tab={`${kind} file · ${fileNo(x.id)}`}>
        <div className="head">
          <div>
            <h1 style={{ fontSize: 24, marginTop: 0 }}>{x.display_name}</h1>
            {agent ? <p className="sec">{x.state === 'active' ? x.display_name + ' is added. Choose Start to finish setting it up.' : x.state === 'registered' ? 'This agent still needs to be switched on before it can start.' : x.state === 'suspended' ? 'This agent’s access is suspended. A new start needs it to be reinstated.' : 'This agent is retired.'}</p> : <div className="sec">
              {kind}
              {person ? (
                <>
                  {' · answers to '}
                  <Pill x={person} />
                </>
              ) : null}{' '}
              {since ? <span className="dim">· since {day(since)}</span> : null}
            </div>}
          </div>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            {agent ? null : <><a className="btn" href={'#/directory/manage?action=profile&identity=' + encodeURIComponent(x.id)}>Edit name</a>
            <span className={'state ' + x.state} id="state">{stop?.agent === x.id ? 'Suspended in this stop answer' : STATUS[x.state]}</span>
            {ACTIONS[x.state].map((a) => (
              <button key={a} className={'btn ' + (a === 'suspend' || a === 'retire' ? 'danger' : 'primary')} data-act={a} onClick={() => { location.hash = '/directory/manage?action=status&identity=' + encodeURIComponent(x.id); }}>
                {ACTION[a]}
              </button>
            ))}</>}
            {kind === 'agent' && (x.state === 'active' || x.state === 'suspended') ? (
              <EmergencyStop id={x.id} active={x.state === 'active'} stopped={stopped} />
            ) : null}
          </div>
        </div>
        {stop && stop.agent === x.id ? <StopReceipt answer={stop} /> : null}
        {agent ? <div className="pane">
          {tab === 'profile' ? <AgentOverview key={x.id} agent={agent} details={(problems) => <AgentDetails data={{ ...data, x }} tab={tab} reload={reload} problems={problems} />} /> : <><p><a href={'#/file/' + encodeURIComponent(x.id)}>← About {x.display_name}</a></p>{tab !== 'record' ? <div className="agent-settings"><TabBody tab={tab} data={{ ...data, x }} reload={reload} /></div> : null}<AgentDetails key={x.id + ':' + tab} data={{ ...data, x }} tab={tab} reload={reload} /></>}
        </div> : <><nav className="tabs">
          {TABS.map(([k, l]) => (
            <a key={k} href={`#/file/${x.id}/${k}`} className={tab === k ? 'on' : ''}>
              {l}
              {counts[k] !== undefined ? <span className="n">{counts[k]}</span> : null}
            </a>
          ))}
        </nav>
        <div className="pane"><TabBody tab={tab} data={data} reload={reload} /></div></>}
      </div>
    </div>
  );
}

export function IdentityFile() {
  const { id = '', tab = 'profile' } = useParams();
  const [version, setVersion] = useState(0);
  const [stop, setStop] = useState<StopAnswer | null>(null);
  const load = useLoad(() => readFile(id), id + '#' + version);
  if (load.status === 'refused' && load.refused.status === 404 && load.refused.refusal.refusal !== 'Unanswered') {
    return (
      <div className="page">
        <h1>Not found</h1>
        <p className="sub">This person or agent is not in the directory, or is not among the records you can see. Ask the administrator to check your access.</p>
        <ErrorWords problem={load.refused} /><details><summary>Requested file details</summary><code>{id}</code></details>
      </div>
    );
  }
  return <Gate load={load} title="this person or agent’s file" ok={(data) => <File data={data} tab={tab} reload={() => setVersion((v) => v + 1)} stop={stop} stopped={(answer) => { setStop(answer); }} />} />;
}
