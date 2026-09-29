import { AgentCertificates } from './AgentCertificates';
import { StopHistory } from './StopHistory';
import { TeamsOf } from './TeamsOf';
import { RuntimeSessions } from '../runtime/RuntimeSessions';
import { AgentMemory } from './AgentMemory';
import { AgentPolicy } from './AgentPolicy';
import { AgentRefusals } from './AgentRefusals';
import { AgentCredentials } from './AgentCredentials';
import { Provisioning } from '../provisioning/Provisioning';
import { PersonCredentials, PersonSessions } from './PersonSecurity';
import { AssignedRoles } from '../roles/AssignedRoles';
import { api, useLoad } from '../../api';
import type { ReceiptAnswer } from '../../generated';
import { Gate } from '../signin/Gate';
import { useShell } from '../../shell/ShellContext';
import { CheckBox } from '../grants/CheckBox';
import { Delegate } from '../grants/Delegate';
import { GrantCard } from '../grants/GrantCard';
import { chainOf, onText, passesToAgents } from '../grants/model';
import { CHANGE_KINDS } from '../../generated';
import { Pill } from '../people/Pill';
import { firstName } from '../people/directory';
import type { FileData } from './IdentityFile';
import { clock } from './time';

function Profile({ data }: { data: FileData }) {
  const { x, agent, agents } = data;
  return (
    <div className="grid2">
      <div>
        <AssignedRoles id={x.id} />
      </div>
      <div>
        <div className="card">
          <dl className="facts">
            <dt>Kind</dt>
            <dd><span className={'kind ' + x.kind}>{x.kind}</span></dd>
            <dt>Answers to</dt>
            <dd>
              {x.person ? (
                <>
                  <Pill x={x.person} /> <span className="note">decides renewal and retirement; a transfer is its own act</span>
                </>
              ) : (
                <span className="dim">A person answers for themselves</span>
              )}
            </dd>
            <dt>Teams</dt>
            <dd><TeamsOf id={x.id} /></dd>
            <dt>State</dt>
            <dd>{x.state} <span className="note">· authority only; it does not say anything is running</span></dd>
            {agent ? (
              <>
                <dt>Registered by</dt>
                <dd><span className="mono dim">{agent.provenance.registered_by.subject}</span> <span className="note">at {agent.provenance.registered_by.provider}</span></dd>
              </>
            ) : null}
          </dl>
        </div>
        {x.kind === 'person' ? (
          <div className="card">
            <h2>Agents answering to {firstName(x.display_name)}</h2>
            {agents.length ? agents.map((a) => (
              <div className="row" key={a.id}>
                <a href={'#/file/' + a.id} style={{ textDecoration: 'none' }}>{a.display_name}</a>
                <span><span className={'dot s-' + a.state} /><span className="dim">{a.state}</span></span>
              </div>
            )) : <div className="dim">None yet.</div>}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function Access({ data, reload }: { data: FileData; reload: () => void }) {
  const shell = useShell();
  const { x, grants: w } = data;
  const name = firstName(x.display_name);
  const held = w.list.grants.filter((g) => g.holder === x.id);
  const reach = new Map<string, string[]>();
  for (const g of held) {
    if (!g.standing.stands) continue;
    const key = onText(g);
    reach.set(key, [...new Set([...(reach.get(key) ?? []), ...g.actions])]);
  }
  const passable = w.list.grants.filter((g) => g.holder === w.me.person.id && g.standing.stands && passesToAgents(g.pass_on));
  const mineToGive = x.kind === 'agent' && x.person?.id === w.me.person.id && x.state !== 'retired';
  return (
    <div className="grid2">
      <div>
        <div className="section-h" style={{ marginTop: 0 }}>
          <span>Grants</span>
        </div>
        {held.length ? held.map((g) => <GrantCard key={g.id} w={w} g={g} chain={chainOf(w, g)} done={reload} />) : <div className="dim">No grants.</div>}
        {x.kind === 'agent' && x.state !== 'retired' ? (
          <div style={{ marginTop: 12, display: 'flex', gap: 8 }}>
            <button
              className="btn"
              data-act="grant"
              onClick={() =>
                mineToGive && passable.length
                  ? shell.openDrawer(<Delegate w={w} source={passable[0]} to={x.id} done={reload} />)
                  : shell.toast(mineToGive ? 'You hold nothing you may pass on to an agent' : `Only the person ${name} answers to gives it access`)
              }
            >
              Grant access
            </button>
            <button className="btn" data-act="temporary-access" onClick={() => mineToGive && passable.length ? shell.openDrawer(<Delegate w={w} source={passable[0]} to={x.id} done={reload} />) : shell.toast(mineToGive ? 'You hold nothing you may pass on to an agent' : `Only the person ${name} answers to gives it access`)}>Temporary access…</button>
          </div>
        ) : null}
      </div>
      <div>
        <div className="section-h" style={{ marginTop: 0 }}>
          <span>What {name} can reach</span>
          <a href={'#/access/reach/' + x.id} className="note" style={{ letterSpacing: 0, textTransform: 'none' }}>open in Access</a>
        </div>
        <div className="card">
          {reach.size ? (
            <table><tbody>
              {[...reach].map(([res, acts]) => (
                <tr key={res}><td>{res}</td><td><span className="svc built-in">built in</span></td><td className="mono">{acts.join(', ')}</td><td /></tr>
              ))}
            </tbody></table>
          ) : <div className="dim">Nothing.</div>}
        </div>
        <CheckBox w={w} who={x.id} />
      </div>
    </div>
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

function RecordView({ receipts }: { receipts: ReceiptAnswer[] }) {
  return (
    <div className="grid2">
      <div>
        <div className="card">
          <h2>Lifecycle</h2>
          <div className="sec" style={{ marginBottom: 10 }}>registered → active ⇄ suspended → retired</div>
          {receipts.length ? (
            <div className="timeline">
              {receipts.map((r, i) => (
                <div className={'tl' + (i === receipts.length - 1 ? ' now' : '')} key={r.receipt.log.index}>
                  <div className="when">{clock(r.receipt.actor.authenticated_at)}</div>
                  <div>
                    {CHANGE_KINDS[r.receipt.change_kind] ?? 'change ' + r.receipt.change_kind}
                    <span className="note"> · by {r.receipt.actor.subject}</span>
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
          <h2>Evidence</h2>
          <div className="sec">Every change to this file is kept as an append-only entry, for when you need it.</div>
          {receipts.map((r) => (
            <div className="row" style={{ padding: '5px 0' }} key={r.receipt.log.index}>
              <a href={'/api/receipts/' + r.receipt.log.index} className="mono">entry #{r.receipt.log.index}</a>
              <span className="mono dim">{r.receipt.operation.slice(0, 11)}…</span>
            </div>
          ))}
          {receipts.length ? <div className="note" style={{ marginTop: 6 }}>Each entry is the signed receipt, the log checkpoint and its inclusion proof.</div> : null}
        </div>
      </div>
    </div>
  );
}

export function TabBody({ tab, data, reload }: { tab: string; data: FileData; reload: () => void }) {
  const person = data.x.kind === 'person';
  switch (tab) {
    case 'access':
      return <Access data={data} reload={reload} />;
    case 'provisioning':
      return person ? <div className="dim">A person has no provisioning profile. Their own tools are their own.</div> : <Provisioning id={data.x.id} />;
    case 'memory':
      return person ? <p>A person’s memories are not kept in an agent home.</p> : <AgentMemory id={data.x.id} />;
    case 'credentials':
      return person ? <PersonCredentials id={data.x.id} /> : <AgentCredentials id={data.x.id} />;
    case 'sessions':
      return person ? <PersonSessions id={data.x.id} /> : <RuntimeSessions agent={data.x.id} />;
    case 'certificate':
      return person ? (
        <div className="card"><div className="sec">People sign in; they are not issued certificates here.</div></div>
      ) : (
        <AgentCertificates id={data.x.id} />
      );
    case 'policy':
      return person ? (
        <div className="card"><div className="sec">A person’s own tools are not judged by an agent policy.</div></div>
      ) : (
        <><AgentPolicy key={data.x.id} id={data.x.id} /><AgentRefusals key={'r' + data.x.id} id={data.x.id} /></>
      );
    case 'record':
      return <><Record data={data} />{person ? null : <StopHistory key={data.x.id} id={data.x.id} />}</>;
    default:
      return <Profile data={data} />;
  }
}
