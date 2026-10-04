import { actionWords, resourceFromText } from '../grants/action-words';
import { readTogether } from '../../reads';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useLoad } from '../../api';
import { AccessTabs } from './AccessTabs';
import { Listing } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held, OrgTeam } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { DirectoryGate as Gate, problemWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import { reachMap } from '../grants/check';
import { CheckBox, resourcesSeen, WhoChoice } from '../grants/CheckBox';
import { ActForm, ActPanel, ChangeButtons, grantColumns } from '../grants/GrantTable';
import type { Act } from '../grants/GrantTable';
import { IssueRoot } from '../grants/IssueRoot';
import { grantNo, nameOf, readGrantWorld, resourceLabel, resourceName, voidOf } from '../grants/model';
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
            <tr key={res}><td>{resourceName(w, resourceFromText(res))}</td><td>{actionWords(w.model, resourceFromText(res), acts)}</td></tr>
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

function Body({ w, teams, mode, arg, reload }: { w: GrantWorld; teams: Teams; mode: string; arg?: string; reload: () => void }) {
  const navigate = useNavigate();
  const admin = w.people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<'all' | 'void'>('all');
  const [acting, setActing] = useState<{ grant: Grant; act: Act; opener: HTMLElement } | null>(null);
  const issuing = mode === 'issue';
  const asking = mode !== '' && !issuing;
  const segs: [string, string][] = [['can', 'Can someone…'], ['reach', 'What can someone reach'], ['who', 'Who can reach something']];
  const seen = resourcesSeen(w);
  const resources = [...seen.keys()];
  const labels = new Map([...seen].map(([k, v]) => [k, resourceLabel(v.resource, w)]));
  let q = <CheckBox w={w} />;
  if (mode === 'reach') {
    const id = arg && w.who.has(arg) ? arg : w.me.person.id;
    q = <>
      <div className="q ask-line"><span className="sec">What can</span><WhoChoice w={w} id="rWho" value={id} change={(who) => navigate('/access/reach/' + who)} /><span className="sec">reach?</span></div>
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
  const close = () => setActing(null);
  const columns = [...grantColumns(w), { head: 'Change', cell: (g: Grant) => <ChangeButtons w={w} g={g} give open={(act, opener) => setActing({ grant: g, act, opener })} /> }];
  const panel = issuing ? <ActPanel className="issue-root" label="Issue root grant" opener={null} close={() => navigate('/access')}><IssueRoot resources={[...seen.values()].map((entry) => entry.resource)} /></ActPanel>
    : acting ? <ActPanel label={acting.act === 'revoke' ? 'Revoke' : 'Give'} opener={acting.opener} close={close}><ActForm w={w} g={acting.grant} act={acting.act} done={reload} close={close} /></ActPanel> : null;
  return <div className="page fill">
    <div className="head">
      <div><h1>Access</h1><p className="sub">{asking ? 'Ask it any way round. Every answer traces to a person, or says why not.' : 'Every grant you may see. Last used is an exercise seen where access is enforced; not seen means none was observed, never that it was never used.'}</p></div>
      {asking ? null : issuing ? <a className="btn" href="#/access">Close the form</a> : <a className="btn primary" href="#/access/issue">Issue root grant</a>}
    </div>
    <AccessTabs on={asking ? 'ask' : 'grants'} />
    {asking ? <div className="pane">
      <div className="seg">{segs.map(([k, l]) => <a key={k} href={'#/access/' + k} className={mode === k ? 'on' : ''}>{l}</a>)}</div>
      <div className="check">{q}</div>
    </div> : <>
      {teams.refused ? <p className="why-not">Teams cannot be read, so grants are listed without their team. {teams.refused}</p> : null}
      <div className="body one grant-cols" style={panel ? { gridTemplateRows: 'minmax(0, auto) minmax(0, 1fr)' } : undefined}>
        {panel ? <div className="pane">{panel}</div> : null}
        <Listing<Grant> groups={groups} columns={columns} id={(g) => g.id} href={(g) => `#/file/${g.holder}/access`}
          words={(g) => grantNo(g.id) + ' ' + nameOf(w, g.holder) + ' ' + g.relation + ' ' + resourceName(w, g.resource)} noun="grants"
          holds={(items) => items.length.toLocaleString('en-AU') + (items.length === 1 ? ' grant' : ' grants')}
          selected={null} select={() => undefined} open={(g) => navigate(`/file/${g.holder}/access`)}
          tools={<>
            <WhoseSelect whose={whose} set={setWhose} teams={teams.list} admin={admin} />
            <div className="seg">{([['all', 'All'], ['void', 'Not standing']] as ['all' | 'void', string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} onClick={() => setShow(key)}>{label}</button>)}</div>
          </>} />
      </div>
    </>}
  </div>;
}

type Teams = { list: OrgTeam[]; refused: string };

export function Access() {
  // No mode in the address is the Grants tab; a mode is one of the three questions on the Ask tab.
  const { mode = '', arg } = useParams();
  const [revision, setRevision] = useState(0);
  const load = useLoad(() => readTogether({
    w: readGrantWorld(),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  }), 'access:' + revision);
  return <Gate load={load} title="Access" ok={({ w, teams }) => <Body w={w} teams={teams} mode={mode} arg={arg} reload={() => setRevision((value) => value + 1)} />} />;
}
