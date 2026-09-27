import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import { api, useLoad } from '../../api';
import type { Scope } from '../../generated';
import { keyable } from '../../shell/keyable';
import { useShell } from '../../shell/ShellContext';
import type { KindFilter } from '../../shell/ShellContext';
import { Gate } from '../signin/Gate';
import { entries, needsNewPerson } from './directory';
import type { Entry } from './directory';
import { Preview } from './Preview';

const NOT_YET = (
  <span className="open-q" title="Its server does not exist yet">
    not built yet
  </span>
);

function PeopleHead() {
  const shell = useShell();
  const filters: [KindFilter, string][] = [['all', 'All'], ['person', 'People'], ['agent', 'Agents'], ['teams', 'Teams'], ['found', 'Found']];
  return (
    <div className="head">
      <div>
        <div className="eyebrow">Directory</div>
        <h1>People and agents</h1>
        <p className="sub">Everyone who works here, human or not. Every agent answers to a person.</p>
      </div>
      <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
        <div className="seg">
          {filters.map(([k, l]) => (
            <button key={k} data-kind={k} className={shell.filterKind === k ? 'on' : ''} onClick={() => { shell.setFilterKind(k); shell.setCursor(0); }}>
              {l}
            </button>
          ))}
        </div>
        <button className="btn primary" data-act="commission" onClick={() => shell.toast('Registering an agent from this screen is not built yet')}>
          Register an agent
        </button>
      </div>
    </div>
  );
}

function Stat({ n, l, warn }: { n: number | null; l: string; warn?: boolean }) {
  return (
    <div className="stat">
      <div className="n" style={warn && n ? { color: 'var(--warn)' } : undefined}>
        {n === null ? <span className="dim">—</span> : n}
      </div>
      <div className="l">
        {l}
        {n === null ? <> · {NOT_YET}</> : null}
      </div>
    </div>
  );
}

function List({ all, scope }: { all: Entry[]; scope: Scope }) {
  const shell = useShell();
  const navigate = useNavigate();
  const list = all.filter((x) => shell.filterKind === 'all' || x.kind === shell.filterKind);
  const rows = list.map((x) => '#/file/' + x.id);
  const key = rows.join(',');
  useEffect(() => {
    shell.setRows(rows);
    return () => shell.setRows([]);
  }, [key]);
  const cursor = Math.min(shell.cursor, Math.max(0, list.length - 1));
  const sel = list[cursor];
  const active = (kind: string) => all.filter((x) => x.kind === kind && x.state === 'active').length;
  return (
    <>
      <div className="stat-strip">
        <Stat n={active('person')} l="people active" />
        <Stat n={active('agent')} l="agents active" />
        <Stat n={all.filter(needsNewPerson).length} l="with no one answering" warn />
        <Stat n={null} l="found, not registered" />
        <Stat n={null} l="sessions running" />
      </div>
      <div className="split">
        <div>
          <table>
            <thead>
              <tr><th>Name</th><th>Kind</th><th>Role</th><th>State</th><th>Answers to</th><th>Reaches</th></tr>
            </thead>
            <tbody>
              {list.map((x, i) => (
                <Row key={x.id} x={x} i={i} cursor={cursor} open={() => navigate('/file/' + x.id)} />
              ))}
            </tbody>
          </table>
          {list.length ? null : <div className="dim" style={{ marginTop: 10 }}>No one here yet.</div>}
          <p className="note" style={{ marginTop: 8 }}>j and k move, Enter opens. Hover a row to preview it.</p>
          <p className="note">Role and Reaches are {NOT_YET}: the directory records no roles yet, and reach comes from grants (DIRECTORY-006).</p>
          {scope === 'personal' ? (
            <p className="note">Your own records: you and the agents that answer to you. A directory administrator sees everyone through the directory&apos;s own routes.</p>
          ) : null}
        </div>
        <div className="preview">{sel ? <Preview x={sel} /> : null}</div>
      </div>
    </>
  );
}

function Row({ x, i, cursor, open }: { x: Entry; i: number; cursor: number; open: () => void }) {
  const shell = useShell();
  return (
    <tr
      className={i === cursor ? 'cursor' : ''}
      data-href={'#/file/' + x.id}
      data-pick={i}
      onClick={open}
      onMouseOver={() => i !== cursor && shell.setCursor(i)}
      onFocus={() => i !== cursor && shell.setCursor(i)}
      {...keyable(open)}
    >
      <td>{x.display_name}</td>
      <td><span className={'kind ' + x.kind}>{x.kind}</span></td>
      <td className="sec"><span className="dim">—</span></td>
      <td><span className={'dot s-' + x.state} />{x.state}</td>
      <td className="sec">
        {x.person ? (
          <>
            {x.person.display_name}
            {needsNewPerson(x) ? <span style={{ color: 'var(--warn)' }}> ({x.person.state})</span> : null}
          </>
        ) : (
          <span className="dim">—</span>
        )}
      </td>
      <td className="mono"><span className="dim">—</span></td>
    </tr>
  );
}

function Elsewhere({ which }: { which: 'teams' | 'found' }) {
  return (
    <div className="empty-note" style={{ marginBottom: 14 }}>
      {NOT_YET}{' '}
      {which === 'teams'
        ? 'Teams have no server yet. A team can answer for an agent together, and can hold access that its members inherit.'
        : 'Found agents need runtimes and connections that report sessions with no identity; none reports here yet.'}
    </div>
  );
}

export function People() {
  const shell = useShell();
  const load = useLoad(api.people, 'people');
  if (shell.filterKind === 'teams' || shell.filterKind === 'found') {
    return (
      <div className="page">
        <PeopleHead />
        <Elsewhere which={shell.filterKind} />
      </div>
    );
  }
  return (
    <Gate
      load={load}
      title="Directory"
      ok={(d) => (
        <div className="page">
          <PeopleHead />
          <List all={entries(d)} scope={d.scope} />
        </div>
      )}
    />
  );
}
