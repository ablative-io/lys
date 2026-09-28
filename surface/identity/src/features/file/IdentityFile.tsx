import { useState } from 'react';
import { useParams } from 'react-router';
import { Refused, api, useLoad } from '../../api';
import { kindOf } from '../../generated';
import type { AgentSummary, AgentView, ReceiptAnswer } from '../../generated';
import { EmergencyStop } from './EmergencyStop';
import { Gate } from '../signin/Gate';
import { fileNo } from '../people/directory';
import type { Entry } from '../people/directory';
import { Pill } from '../people/Pill';
import { TabBody } from './sections';
import { ACTIONS, TABS } from './tabs';
import { day } from './time';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';

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
  const [receipts, grants] = await Promise.all([Promise.all(agent.provenance.events.map((index) => api.receipt(index))), readGrantWorld()]);
  const x: Entry = { id: agent.id, display_name: agent.display_name, state: agent.state, kind: 'agent', role: agent.role, person: agent.person };
  return { x, agent, agents: [], receipts, grants };
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

function File({ data, tab, reload }: { data: FileData; tab: string; reload: () => void }) {
  const { x, agent } = data;
  const kind = x.kind;
  const person = x.person;
  const since = agent?.provenance.registration?.actor.authenticated_at;
  const counts: Record<string, number | undefined> = {
    record: agent ? agent.provenance.events.length : undefined,
    access: data.grants.list.grants.filter((g) => g.holder === x.id).length,
  };
  return (
    <div className="page">
      <div className="eyebrow">
        <a href="#/people">People and agents</a> / {fileNo(x.id)}
      </div>
      <div className="file" data-tab={`${kind} file · ${fileNo(x.id)}`}>
        <div className="head">
          <div>
            <h1 style={{ fontSize: 24, marginTop: 0 }}>{x.display_name}</h1>
            <div className="sec">
              {kind}
              {person ? (
                <>
                  {' · answers to '}
                  <Pill x={person} />
                  {agent?.needs_new_person ? <span style={{ color: 'var(--warn)' }}> {person.state}: needs a new person</span> : null}
                </>
              ) : null}{' '}
              {since ? <span className="dim">· since {day(since)}</span> : null}
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <a className="btn" href={'#/directory/manage?action=profile&identity=' + encodeURIComponent(x.id)}>Edit name</a>
            <span className={'state ' + x.state} id="state">{x.state}</span>
            {ACTIONS[x.state].map((a) => (
              <button key={a} className={'btn ' + (a === 'suspend' || a === 'retire' ? 'danger' : 'primary')} data-act={a} onClick={() => { location.hash = '/directory/manage?action=status&identity=' + encodeURIComponent(x.id); }}>
                {a[0].toUpperCase() + a.slice(1)}
              </button>
            ))}
            {kind === 'agent' && x.state === 'active' ? (
              <a className="btn primary" data-act="start" href={'#/file/' + encodeURIComponent(x.id) + '/provisioning'} title="Prepare a start from the reviewed profile">Start…</a>
            ) : null}
            {kind === 'agent' && (x.state === 'active' || x.state === 'suspended') ? (
              <EmergencyStop id={x.id} active={x.state === 'active'} stopped={reload} />
            ) : null}
          </div>
        </div>
        <nav className="tabs">
          {TABS.map(([k, l]) => (
            <a key={k} href={`#/file/${x.id}/${k}`} className={tab === k ? 'on' : ''}>
              {l}
              {counts[k] !== undefined ? <span className="n">{counts[k]}</span> : null}
            </a>
          ))}
        </nav>
        <TabBody tab={tab} data={data} reload={reload} />
      </div>
    </div>
  );
}

export function IdentityFile() {
  const { id = '', tab = 'profile' } = useParams();
  const [version, setVersion] = useState(0);
  const load = useLoad(() => readFile(id), id + '#' + version);
  if (load.status === 'refused' && load.refused.status === 404 && load.refused.refusal.refusal !== 'Unanswered') {
    return (
      <div className="page">
        <h1>Not found</h1>
        <p className="sub">{id} is not in the directory, or is not one you may see.</p>
        <div className="why-not">
          <b>{load.refused.refusal.refusal}</b> <span className="sec">{load.refused.refusal.reason}</span>
        </div>
      </div>
    );
  }
  return <Gate load={load} title="People and agents" ok={(data) => <File data={data} tab={tab} reload={() => setVersion((v) => v + 1)} />} />;
}
