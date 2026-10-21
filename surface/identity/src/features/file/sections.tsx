import { useState } from 'react';
import { PersonalBudgets } from './PersonalBudgets';
import { AgentUsage } from '../usage/Usage';
import { AgentCertificates } from './AgentCertificates';
import { AgentConnect } from './AgentConnect';
import { StopHistory } from './StopHistory';
import { TeamsOf } from './TeamsOf';
import { RuntimeSessions } from '../runtime/RuntimeSessions';
import { AgentMemory } from './AgentMemory';
import { AgentRefusals } from './AgentRefusals';
import { AgentCredentials } from './AgentCredentials';
import { Provisioning } from '../provisioning/Provisioning';
import { PersonCredentials, PersonSessions } from './PersonSecurity';
import { AssignedRoles } from '../roles/AssignedRoles';
import { api, useLoad } from '../../api';
import type { MeView, ReceiptAnswer } from '../../generated';
import { IdentityName, STATUS } from '../people/Words';
import { DirectoryGate as Gate } from '../people/Words';
import { CheckBox } from '../grants/CheckBox';
import { GiveAccess } from './GiveAccess';
import { GrantTable } from '../grants/GrantTable';
import { onText } from '../grants/model';
import { changeWords } from './receipt-words';
import { Pill } from '../people/Pill';
import { calledBy, firstName } from '../people/directory';
import type { FileData } from './IdentityFile';
import { clock } from './time';
import './file.css';
import { AppsFirst, anyAppResource } from '../grants/AppsFirst';

function Profile({ data }: { data: FileData }) {
  const { x, agent, agents } = data;
  return (
    <div className="grid2">
      {!agent ? (
        <div>
          <AssignedRoles id={x.id} />
        </div>
      ) : null}
      <div>
        <div className="card">
          <dl className="facts">
            <dt>Kind</dt>
            <dd>{x.kind === 'agent' ? 'Agent' : 'Person'}</dd>
            {x.person ? (
              <>
                <dt>Answers to</dt>
                <dd>
                  <Pill x={x.person} /> <span className="note">decides when its access is renewed or retired</span>
                </dd>
              </>
            ) : null}
            <dt>Teams</dt>
            <dd>
              <TeamsOf id={x.id} />
            </dd>
            <dt>Access</dt>
            <dd>{STATUS[x.state] ?? x.state}</dd>
          </dl>
        </div>
        {x.kind === 'person' ? <AgentsOf name={x.display_name} agents={agents} /> : null}
      </div>
    </div>
  );
}

/** The agents answering to a person; retired ones are left out until asked for, with the same control the People list has. */
function AgentsOf({ name, agents }: { name: string; agents: FileData['agents'] }) {
  const [retired, setRetired] = useState(false);
  const retiredCount = agents.filter((a) => a.state === 'retired').length;
  const shown = retired ? agents : agents.filter((a) => a.state !== 'retired');
  return (
    <div className="card">
      <div className="file-section-head">
        <h2>Agents answering to {firstName(name)}</h2>
        {retiredCount ? (
          <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>
            {retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}
          </button>
        ) : null}
      </div>
      {shown.length ? (
        shown.map((a) => (
          <div className="row" key={a.id}>
            <a href={'#/file/' + a.id} style={{ textDecoration: 'none' }}>
              {a.display_name}
            </a>
            <span>
              <span className={'dot s-' + a.state} />
              <span className="dim">{STATUS[a.state] ?? a.state}</span>
            </span>
          </div>
        ))
      ) : (
        <div className="dim">{agents.length ? 'Every agent answering to ' + firstName(name) + ' is retired.' : 'None yet.'}</div>
      )}
    </div>
  );
}

function Access({ data, reload, giveQuery }: { data: FileData; reload: () => void; giveQuery: string }) {
  const { x, grants: w } = data;
  const name = calledBy(x.id, x.display_name);
  const held = w.list.grants.filter((g) => g.holder === x.id);
  const reach = new Map<string, string[]>();
  for (const g of held) {
    if (!g.standing.stands) continue;
    const key = onText(g);
    reach.set(key, [...new Set([...(reach.get(key) ?? []), ...g.actions])]);
  }
  return (
    <>
      <GiveAccess key={x.id + giveQuery} x={x} w={w} query={giveQuery} reload={reload} />
      <GrantTable w={w} grants={held} done={reload} give={false} />
      {x.kind === 'agent' && !anyAppResource(w.list.grants.map((g) => g.resource)) ? <AppsFirst /> : null}
      {/* After the grant table, the two panels share the width as two halves. */}
      <div className="access-halves">
        <div>
          <div className="section-h">
            <span>What {name} can reach</span>
            <a href={'#/access/reach/' + x.id} className="note" style={{ letterSpacing: 0, textTransform: 'none' }}>
              open in Access
            </a>
          </div>
          <div className="card">
            {reach.size ? (
              <table>
                <tbody>
                  {[...reach].map(([res, acts]) => (
                    <tr key={res}>
                      <td>{res}</td>
                      <td>
                        <span className="svc built-in">built in</span>
                      </td>
                      <td className="mono">{acts.join(', ')}</td>
                      <td />
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="dim">Nothing.</div>
            )}
          </div>
        </div>
        <div>
          <CheckBox w={w} who={x.id} />
        </div>
      </div>
    </>
  );
}

function Record({ data }: { data: FileData }) {
  if (!data.agent) return <PersonRecord id={data.x.id} />;
  return <RecordView receipts={data.receipts} />;
}

function PersonRecord({ id }: { id: string }) {
  const load = useLoad(async () => {
    const record = await api.identity(id);
    return Promise.all(record.events.map((index) => api.receipt(index)));
  }, 'person-record:' + id);
  return <Gate load={load} title="Directory record" ok={(receipts) => <RecordView receipts={receipts} />} />;
}

/**
 * Whether a record's actor is the signed-in person. The same issuer and subject is one of their sign-ins. A line
 * written before the install moved to a new issuer names the earlier issuer (identity.json `issuer_moved_from`),
 * which the front end is never told; so a subject matches when exactly one of the person's sign-in identities has
 * that subject, whatever its issuer. Two sign-ins sharing the subject leave the line unnamed rather than guessed.
 */
export function signedInActor(actor: { issuer: string; subject: string }, me: MeView): boolean {
  if (me.sign_in_identities.some((login) => login.provider === actor.issuer && login.subject === actor.subject)) return true;
  return me.sign_in_identities.filter((login) => login.subject === actor.subject).length === 1;
}

/** Who a record line was signed for: an agent or the signed-in person by name; any other sign-in by the account name its provider gave, which is all the record holds. */
function Actor({ actor, me }: { actor: ReceiptAnswer['receipt']['actor']; me: MeView | null }) {
  if (actor.agent) return <IdentityName id={actor.agent} />;
  if (me && signedInActor(actor, me)) return <IdentityName id={me.person.id} />;
  return <>{actor.subject}</>;
}

/** What a record line changed: from and to when the signed event carries them, else the change's kind. */
function Change({ answer }: { answer: ReceiptAnswer }) {
  const said = changeWords(answer);
  return (
    <>
      {said.words}
      {said.names.to ? (
        <>
          {' '}
          <IdentityName id={said.names.to} />
          {said.names.from ? (
            <span className="note">
              {' '}
              (before: <IdentityName id={said.names.from} />)
            </span>
          ) : null}
        </>
      ) : null}
      {said.reason ? <span className="note"> · “{said.reason}”</span> : null}
    </>
  );
}

function RecordView({ receipts }: { receipts: ReceiptAnswer[] }) {
  const me = useLoad(api.me, 'record-me');
  return (
    <div className="grid2">
      <div>
        <div className="card">
          <h2>What changed</h2>
          <div className="sec" style={{ marginBottom: 10 }}>
            Every change to this record, oldest first, and who made it.
          </div>
          {receipts.length ? (
            <div className="timeline">
              {receipts.map((r, i) => (
                <div className={'tl' + (i === receipts.length - 1 ? ' now' : '')} key={r.receipt.log.index}>
                  <div className="when">{clock(r.receipt.actor.authenticated_at)}</div>
                  <div>
                    <Change answer={r} />
                    <span className="note">
                      {' '}
                      · by <Actor actor={r.receipt.actor} me={me.status === 'ok' ? me.data : null} />
                      {r.receipt.actor.authentication === 'operator' ? ', with the operator token' : ''}
                      {r.receipt.actor.through_app ? `, through ${r.receipt.actor.through_app}` : ''}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="note">No recorded changes were returned for this identity.</p>
          )}
        </div>
      </div>
      <div>
        <div className="card">
          <h2>Proof of each change</h2>
          <div className="sec">Lys signs every change and keeps it for good, so it can be checked later. You only need these if someone asks for proof.</div>
          {receipts.map((r) => (
            <div className="row" style={{ padding: '5px 0' }} key={r.receipt.log.index}>
              <a href={'/api/receipts/' + r.receipt.log.index} download={'lys-change-' + r.receipt.log.index + '.json'}>
                Download proof of change {r.receipt.log.index}
              </a>
              <span className="dim">{clock(r.receipt.actor.authenticated_at)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function TabBody({ tab, data, reload, giveQuery = '' }: { tab: string; data: FileData; reload: () => void; giveQuery?: string }) {
  const person = data.x.kind === 'person';
  switch (tab) {
    case 'budgets':
      return person ? (
        <PersonalBudgets key={data.x.id} id={data.x.id} name={data.x.display_name} />
      ) : (
        <section className="usage">
          <AgentUsage key={data.x.id} agent={data.x.id} name={data.x.display_name} />
        </section>
      );
    case 'access':
      return <Access data={data} reload={reload} giveQuery={giveQuery} />;
    case 'provisioning':
      return person ? (
        <Profile data={data} />
      ) : (
        <>
          <Provisioning id={data.x.id} />
          <AgentRefusals key={'r' + data.x.id} id={data.x.id} />
        </>
      );
    case 'credentials':
      return person ? (
        <PersonCredentials id={data.x.id} />
      ) : (
        <>
          <AgentCredentials id={data.x.id} />
          <AgentCertificates id={data.x.id} />
          <AgentConnect id={data.x.id} />
        </>
      );
    case 'sessions':
      return person ? (
        <PersonSessions id={data.x.id} />
      ) : (
        <>
          <RuntimeSessions agent={data.x.id} />
          <AgentMemory id={data.x.id} />
        </>
      );
    case 'record':
      return (
        <>
          <Record data={data} />
          {person ? null : <StopHistory key={data.x.id} id={data.x.id} />}
        </>
      );
    default:
      return <Profile data={data} />;
  }
}
