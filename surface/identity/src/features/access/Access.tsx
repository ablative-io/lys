import { actionWords, resourceFromText, resourceWords } from '../grants/action-words';
import { readTogether } from '../../reads';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useLoad } from '../../api';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held, OrgTeam } from '../../shell/org';
import { Picker } from '../../shell/Picker';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { DirectoryGate as Gate, problemWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import { reachMap } from '../grants/check';
import { CheckBox, resourcesSeen } from '../grants/CheckBox';
import { grantNo, lastUsedText, lastsText, nameOf, passText, readGrantWorld, resourceLabel, voidOf } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import type { Grant } from '../../generated/grants';
import { Pill } from '../people/Pill';

type ReachMap = Map<string, Map<string, string[]>>;

/** /grants/who is asked only for the segment on screen, once per world: every resource for "reach", one for "who". */
const reachOfAll = new WeakMap<GrantWorld, Promise<ReachMap>>();
const reachOfOne = new WeakMap<GrantWorld, Map<string, Promise<ReachMap>>>();

function allReach(w: GrantWorld): Promise<ReachMap> {
  let p = reachOfAll.get(w);
  if (!p) {
    p = reachMap([...resourcesSeen(w).values()]);
    reachOfAll.set(w, p);
  }
  return p;
}

function oneReach(w: GrantWorld, res: string): Promise<ReachMap> {
  let byRes = reachOfOne.get(w);
  if (!byRes) {
    byRes = new Map();
    reachOfOne.set(w, byRes);
  }
  let p = byRes.get(res);
  if (!p) {
    const seen = resourcesSeen(w).get(res);
    p = reachMap(seen ? [seen] : []);
    byRes.set(res, p);
  }
  return p;
}

const person = (w: GrantWorld, id: string) => {
  const x = w.who.get(id);
  return { id, display_name: x?.name ?? nameOf(w, id), state: x?.state ?? 'active' } as const;
};

function Reach({ w, id }: { w: GrantWorld; id: string }) {
  const load = useLoad(() => allReach(w), 'reach');
  return (
    <Gate load={load} title="Access" ok={(reach) => {
      const rows = [...reach].map(([res, byHolder]) => [res, byHolder.get(id) ?? []] as const).filter(([, acts]) => acts.length);
      return rows.length ? (
        <table><tbody>
          {rows.map(([res, acts]) => (
            <tr key={res}><td>{resourceWords(resourceFromText(res))}</td><td>{actionWords(w.model, resourceFromText(res), acts)}</td></tr>
          ))}
        </tbody></table>
      ) : <div className="dim">Nothing.</div>;
    }} />
  );
}

function WhoCan({ w, res }: { w: GrantWorld; res: string }) {
  const load = useLoad(() => oneReach(w, res), 'who:' + res);
  return (
    <Gate load={load} title="Access" ok={(reach) => {
      const holders = [...(reach.get(res) ?? new Map<string, string[]>())];
      return holders.length ? holders.map(([h, acts]) => (
        <div className="row" key={h}><span><Pill x={person(w, h)} /></span><span className="mono">{actionWords(w.model, resourceFromText(res), acts)}</span></div>
      )) : <div className="dim">Nobody.</div>;
    }} />
  );
}

function Body({ w, teams, mode, arg }: { w: GrantWorld; teams: Teams; mode: string; arg?: string }) {
  const navigate = useNavigate();
  const admin = w.people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<'all' | 'void'>('all');
  const [picked, setPicked] = useState<string | null>(null);
  const segs: [string, string][] = [['can', 'Can someone…'], ['reach', 'What can someone reach'], ['who', 'Who can reach something']];
  const seen = resourcesSeen(w);
  const resources = [...seen.keys()];
  const labels = new Map([...seen].map(([k, v]) => [k, resourceLabel(v.resource)]));
  const everyone = [...w.who].map(([id, x]) => ({ id, name: x.name, detail: x.kind === 'agent' && x.responsible ? 'agent of ' + nameOf(w, x.responsible) : undefined }));
  let q = <CheckBox w={w} />;
  if (mode === 'reach') {
    const id = arg && w.who.has(arg) ? arg : w.me.person.id;
    q = <>
      <div className="q"><span className="sec">What can <strong>{nameOf(w, id)}</strong> reach?</span></div>
      <Picker key={id} name="reach" label="Ask about someone else" options={everyone} onChange={(ids) => { if (ids[0]) navigate('/access/reach/' + ids[0]); }} />
      <div className="card"><Reach w={w} id={id} /></div>
    </>;
  }
  if (mode === 'who') {
    const res = arg && seen.has(arg) ? arg : resources[0] ?? '';
    q = <>
      <div className="q">
        <span className="sec">Who can reach</span>
        <select aria-label="Resource" value={res} onChange={(e) => navigate('/access/who/' + e.target.value)}>
          {resources.map((r) => <option key={r} value={r}>{labels.get(r) ?? r}</option>)}
        </select>
        <span className="sec">?</span>
      </div>
      <div className="card" id="whoCan"><WhoCan w={w} res={res} /></div>
    </>;
  }
  const held = (g: Grant): Held => ({ id: g.holder, person: w.who.get(g.holder)?.responsible ?? null });
  const scoped = w.list.grants.filter((g) => inWhose(whose, teams.list, w.me.person.id, held(g))).filter((g) => show === 'all' || voidOf(w, g) !== null);
  const groups = groupByTeam(scoped, held, teams.list, whose, (id) => nameOf(w, id));
  const chosen = scoped.find((g) => g.id === picked) ?? null;
  const columns: Column<Grant>[] = [
    { head: 'Grant', cell: (g) => <span className="mono">{grantNo(g.id)}</span> },
    { head: 'Holder', cell: (g) => nameOf(w, g.holder) },
    { head: 'Relation', cell: (g) => <span className="mono">{g.relation}</span> },
    { head: 'On', cell: (g) => <span className="mono">{resourceWords(g.resource)}</span> },
    { head: 'Derives from', cell: (g) => { const up = g.source ? w.byId.get(g.source) : undefined; return <span className="sec">{g.source ? `${grantNo(g.source)} · ${up ? nameOf(w, up.holder) : 'not visible'}` : <span className="dim">root</span>}</span>; } },
    { head: 'May pass on', cell: (g) => <span className="sec">{passText(g.pass_on)}</span> },
    { head: 'Lasts', cell: (g) => <span className="sec">{lastsText(g)}</span> },
    { head: 'Last used', cell: (g) => <span className="sec">{g.last_use.seen ? lastUsedText(g) : <span className="dim">{lastUsedText(g)}</span>}</span> },
    { head: 'Stands', cell: (g) => { const v = voidOf(w, g); return v === null ? <><span className="dot s-active" />yes</> : <span className="danger" title={v.why}>no</span>; } },
  ];
  return <div className="page fill">
    <div className="head">
      <div><div className="eyebrow">Access</div><h1>Access</h1><p className="sub">Ask it any way round. Every answer traces to a person, or says why not.</p></div>
      <a className="btn primary" href="#/access/issue">Issue root grant</a>
    </div>
    {teams.refused ? <p className="why-not">Teams cannot be read, so grants are listed without their team. {teams.refused}</p> : null}
    <div className="body work">
      <Listing<Grant> groups={groups} columns={columns} id={(g) => g.id} href={(g) => `#/file/${g.holder}/access`}
        words={(g) => grantNo(g.id) + ' ' + nameOf(w, g.holder) + ' ' + g.relation + ' ' + resourceWords(g.resource)} noun="grants"
        holds={(items) => items.length.toLocaleString('en-AU') + (items.length === 1 ? ' grant' : ' grants')}
        selected={chosen?.id ?? null} select={(g) => setPicked(g.id)} open={(g) => navigate(`/file/${g.holder}/access`)}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={teams.list} admin={admin} />
          <div className="seg">{([['all', 'All'], ['void', 'Not standing']] as ['all' | 'void', string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} onClick={() => setShow(key)}>{label}</button>)}</div>
        </>} />
      <div className="detail">
        <div className="seg">{segs.map(([k, l]) => <a key={k} href={'#/access/' + k} className={mode === k ? 'on' : ''}>{l}</a>)}</div>
        <div className="check">{q}</div>
        {chosen ? <GrantSummary w={w} g={chosen} /> : <p className="note">Last used is an exercise seen where access is enforced. Not seen means none was observed, never that it was never used.</p>}
      </div>
    </div>
  </div>;
}

function GrantSummary({ w, g }: { w: GrantWorld; g: Grant }) {
  const v = voidOf(w, g);
  return <article className="card" aria-label="Grant">
    <h2>{grantNo(g.id)} · {nameOf(w, g.holder)}</h2>
    <p><span className="mono">{g.relation}</span> on <span className="mono">{resourceWords(g.resource)}</span></p>
    <p className="sec">{passText(g.pass_on)} · {lastsText(g)} · last used {lastUsedText(g)}</p>
    {v === null ? <p><span className="dot s-active" />Stands.</p> : <p className="why-not">{v.why}</p>}
    <a className="btn" href={`#/file/${g.holder}/access`}>Open {nameOf(w, g.holder)}'s access</a>
  </article>;
}

type Teams = { list: OrgTeam[]; refused: string };

export function Access() {
  const { mode = 'can', arg } = useParams();
  const load = useLoad(() => readTogether({
    w: readGrantWorld(),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  }), 'access');
  return <Gate load={load} title="Access" ok={({ w, teams }) => <Body w={w} teams={teams} mode={mode} arg={arg} />} />;
}
