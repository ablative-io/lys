import type { ReactNode } from 'react';
import { CHANGE_KINDS } from '../../generated';
import { Pill } from '../people/Pill';
import { firstName } from '../people/directory';
import type { FileData } from './IdentityFile';
import { clock } from './time';

/** The mock-up's own empty state, marked as not built, with why. */
export function NotBuilt({ children }: { children: ReactNode }) {
  return (
    <div className="empty-note">
      <span className="open-q">not built yet</span> {children}
    </div>
  );
}

function Profile({ data }: { data: FileData }) {
  const { x, agent, agents } = data;
  return (
    <div className="grid2">
      <div>
        <div className="card">
          <h2>Responsibilities</h2>
          <NotBuilt>The directory records no role for {x.display_name}. Responsibilities, goals and professional practice come from the role, which waits on its ADR (conformance 4.1).</NotBuilt>
        </div>
      </div>
      <div>
        <div className="card">
          <dl className="facts">
            <dt>Kind</dt>
            <dd><span className={'kind ' + x.kind}>{x.kind}</span></dd>
            <dt>Role</dt>
            <dd>
              {agent?.role ?? <span className="dim">—</span>}
              {agent?.version != null ? <span className="ver">v{agent.version}</span> : null}{' '}
              {agent?.role ? null : <span className="note">not recorded yet</span>}
            </dd>
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
            <dd><span className="dim">—</span> <span className="note">not built yet</span></dd>
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

function Access({ data }: { data: FileData }) {
  const name = firstName(data.x.display_name);
  return (
    <div className="grid2">
      <div>
        <div className="section-h" style={{ marginTop: 0 }}>
          <span>Grants</span>
          <span className="open-q" title="The grant representation is not yet decided">draft form</span>
        </div>
        <NotBuilt>Grants arrive with DIRECTORY-006 R1 to R5, which are being built now. Nothing is shown until the server answers them.</NotBuilt>
      </div>
      <div>
        <div className="section-h" style={{ marginTop: 0 }}><span>What {name} can reach</span></div>
        <NotBuilt>Reach is answered from grants.</NotBuilt>
        <div className="check">
          <h2>Can {name} do this?</h2>
          <div className="answer-box">
            <span className="note">The answer shows the path to a person, or the reason it is refused, and which version of the model it used.</span>
          </div>
          <NotBuilt>The check is DIRECTORY-006 R5 (conformance 8.1).</NotBuilt>
        </div>
      </div>
    </div>
  );
}

function Table({ heads, note, title, why }: { heads: string[]; note: string; title: string; why: string }) {
  return (
    <div className="card">
      <div className="row" style={{ padding: '0 0 6px' }}>
        <h2>{title}</h2>
        <span className="note">{note}</span>
      </div>
      <table>
        <thead><tr>{heads.map((h) => <th key={h}>{h}</th>)}</tr></thead>
        <tbody><tr><td colSpan={heads.length} className="dim"><span className="open-q">not built yet</span> {why}</td></tr></tbody>
      </table>
    </div>
  );
}

function Record({ data }: { data: FileData }) {
  const { agent, receipts } = data;
  return (
    <div className="grid2">
      <div>
        <div className="card">
          <h2>Lifecycle</h2>
          <div className="sec" style={{ marginBottom: 10 }}>registered → active ⇄ suspended → retired</div>
          {agent ? (
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
            <NotBuilt>The directory&apos;s view of a person does not carry its events yet.</NotBuilt>
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

export function TabBody({ tab, data }: { tab: string; data: FileData }) {
  const person = data.x.kind === 'person';
  switch (tab) {
    case 'access':
      return <Access data={data} />;
    case 'provisioning':
      return person ? <div className="dim">A person has no provisioning profile. Their own tools are their own.</div> : <NotBuilt>Provisioning profiles (model access, tools, skills, MCP servers, instructions) have no server yet.</NotBuilt>;
    case 'memory':
      return <NotBuilt>Memories and the home have no server here yet: what each memory came from, who can see it, and the last context given.</NotBuilt>;
    case 'credentials':
      return <Table title="Handles" note="An agent holds a handle, never the credential. Dropping the handle ends its use." heads={['Handle', 'Behind it', 'Value', 'Lease', 'Use', '']} why="Handles come from the secrets broker (SECRETS-002)." />;
    case 'sessions':
      return <Table title="Sessions" note="As the runtimes report them. Stopped only when the runtime confirms it." heads={['Where', 'Runtime', 'Acting for', 'Started', 'Status', 'Context given']} why="Sessions come from runtime reports naming a launch record (conformance 5.4 to 5.6)." />;
    case 'certificate':
      return person ? (
        <div className="card"><div className="sec">People sign in; they are not issued certificates here.</div></div>
      ) : (
        <NotBuilt>The capability certificate and its log entry are not issued from this service yet (conformance 6.1 to 6.5).</NotBuilt>
      );
    case 'record':
      return <Record data={data} />;
    default:
      return <Profile data={data} />;
  }
}
