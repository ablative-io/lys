/** The front page fills the screen and never scrolls as a whole: what waits for you on the top line, your agents as a tree under their teams on the left, the running ones as small live pictures on the right, and your account under a second tab. */
import { useState } from 'react';
import type { CSSProperties } from 'react';
import { api, request, useLoad } from '../../api';
import type { AgentSummary, Login, MeView } from '../../generated';
import { Delegate } from '../grants/Delegate';
import { mayText, nameOf, onText, passesToAgents, readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { keyable } from '../../shell/keyable';
import { pref, setPref } from '../../shell/prefs';
import { useShell } from '../../shell/ShellContext';
import { useLocation, useNavigate } from 'react-router';
import { Gate } from '../signin/Gate';
import { OwnAccount } from '../people/Account';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import { buildTree } from '../people/tree';
import type { Branch, Node } from '../people/tree';
import type { AccessRequest } from '../requests/contract';
import { Peek } from '../runtime/Peek';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { SessionList } from '../sessions/Sessions';
import type { Team } from '../teams/contract';
import './you.css';

interface ActiveData {
  kind: 'active';
  me: MeView;
  agents: AgentSummary[];
  w: GrantWorld;
  tree: Node;
  sessions: RuntimeSession[];
  waiting: { requests: number; reviews: number };
}

type YouData = ActiveData | { kind: 'registered'; me: MeView };

async function readYou(): Promise<YouData> {
  const me = await api.me();
  if (me.person.state === 'registered') return { kind: 'registered', me };
  const [w, own, teams, live, requests, reviews] = await Promise.all([
    readGrantWorld(me), api.ownPeople(), request<{ teams: Team[] }>('/teams'), request<{ sessions: RuntimeSession[] }>('/runtime/live'),
    request<{ requests: AccessRequest[] }>('/requests'), request<{ due: unknown[] }>('/reviews'),
  ]);
  const self = own.people.find((p) => p.id === w.me.person.id);
  const all = entries(own);
  const root = all.find((entry) => entry.id === w.me.person.id);
  if (!root) throw new Error('The directory did not list you.');
  const tree = buildTree(root, teams.teams.filter((team) => team.state === 'active'), all.filter((entry) => entry.state !== 'retired'));
  const mine = requests.requests.filter((ask) => ask.state === 'waiting' && (ask.approvers.some((who) => who.id === me.person.id) || ask.responsible.id === me.person.id));
  return {
    kind: 'active', me: w.me, agents: (self?.agents ?? []).filter((a) => a.state !== 'retired'), w, tree, sessions: live.sessions,
    waiting: { requests: mine.length, reviews: reviews.due.length },
  };
}

/** An issuer URL named by its host, as a provider is shown. */
const providerName = (provider: string): string => {
  try {
    return new URL(provider).host;
  } catch {
    return provider;
  }
};

function SignInIdentity({ login, current }: { login: Login; current: boolean }) {
  return (
    <div className="row">
      <span>
        {providerName(login.provider)} <span className="note">{current ? 'signs you in · this session' : 'signs you in'}</span>
      </span>
      <details><summary>Account details</summary><span className="mono dim">{login.subject}</span></details>
    </div>
  );
}

/** How many grants an agent holds, in a word; the list itself is on the agent's own page. */
const holdsWord = (held: string): string => {
  const count = held ? held.split('; ').length : 0;
  return count === 0 ? 'no access' : count === 1 ? '1 grant' : count + ' grants';
};
const liveOf = (sessions: RuntimeSession[], id: string) => sessions.find((entry) => entry.agent === id && entry.shown !== 'stopped');
const FOLDED = 'you-folded';
const readFolded = () => new Set(pref(FOLDED, '').split(',').filter(Boolean));
const depth = (level: number) => ({ '--depth': level } as CSSProperties);

/** Table rows cannot be wrapped in an element, so a keyed fragment stands in. */
function FragmentRows({ children }: { children: React.ReactNode }) { return <>{children}</>; }

function AgentRows({ branches, level, sessions, held, folded, fold, open, watching, toggleWatch }: {
  branches: Branch[]; level: number; sessions: RuntimeSession[]; held: (id: string) => string; folded: Set<string>; fold: (team: string) => void; open: (id: string) => void;
  watching: (session: string) => boolean; toggleWatch: (session: RuntimeSession, name: string) => void;
}) {
  return <>{branches.map((branch, index) => {
    const away = branch.team ? folded.has(branch.team.id) : false;
    const running = branch.members.filter((member) => liveOf(sessions, member.entry.id)).length;
    return <FragmentRows key={branch.team?.id ?? 'loose-' + index}>
      {branch.team ? <tr className="you-team">
        <td colSpan={4} style={depth(level)}>
          <button type="button" className="you-fold" aria-expanded={!away} onClick={() => fold(branch.team!.id)}>
            <span className="you-chevron" aria-hidden="true" />{branch.team.name}
            <span className="you-count">{running ? running + ' running of ' : ''}{branch.members.length}</span>
          </button>
        </td>
      </tr> : null}
      {away ? null : branch.members.map((member) => {
        const session = liveOf(sessions, member.entry.id);
        return <FragmentRows key={member.entry.id}>
          <tr data-href={'#/file/' + member.entry.id} {...keyable(() => open(member.entry.id))}>
            <td style={depth(level + (branch.team ? 1 : 0))} className="you-agent"><span className={'dot ' + (session ? 's-active' : 's-retired')} aria-label={session ? 'running' : 'not running'} />{member.entry.display_name}</td>
            <td className="sec you-where">{session ? 'on ' + (session.machine_name ?? session.machine) : member.entry.state === 'active' ? 'not running' : member.entry.state}</td>
            <td className="sec you-holds" title={held(member.entry.id) || 'no access'}>{holdsWord(held(member.entry.id))}</td>
            <td className="you-act">{session ? <button type="button" className="you-watch" aria-pressed={watching(session.session)} onClick={(event) => { event.stopPropagation(); toggleWatch(session, member.entry.display_name); }}>{watching(session.session) ? 'Watching' : 'Watch'}</button> : null}</td>
          </tr>
          <AgentRows branches={member.branches} level={level + 1} sessions={sessions} held={held} folded={folded} fold={fold} open={open} watching={watching} toggleWatch={toggleWatch} />
        </FragmentRows>;
      })}
    </FragmentRows>;
  })}</>;
}

function Waiting({ waiting }: { waiting: ActiveData['waiting'] }) {
  if (!waiting.requests && !waiting.reviews) return <p className="you-line you-quiet">Nothing waiting.</p>;
  return <p className="you-line you-waiting">
    {waiting.requests ? <a href="#/requests">{waiting.requests === 1 ? '1 request to decide' : waiting.requests + ' requests to decide'}</a> : null}
    {waiting.requests && waiting.reviews ? ' · ' : null}
    {waiting.reviews ? <a href="#/reviews">{waiting.reviews === 1 ? '1 review due' : waiting.reviews + ' reviews due'}</a> : null}
  </p>;
}

function Account({ data, reload }: { data: ActiveData; reload: () => void }) {
  const shell = useShell();
  const { me, agents, w } = data;
  const mine = w.list.grants.filter((g) => g.holder === me.person.id && g.standing.stands);
  const same = (a: Login) => a.provider === me.signed_in.provider && a.subject === me.signed_in.subject;
  return <div className="you-account grid2">
    <div>
      <div className="section-h" style={{ marginTop: 0 }}><span>What you hold</span></div>
      <table>
        <thead><tr><th>What it lets you do</th><th>Given by</th><th>You can give it to your agents</th><th></th></tr></thead>
        <tbody>
          {mine.length ? mine.map((g) => (
            <tr key={g.id}>
              <td>{mayText(w, g)}<details><summary>Exactly</summary><span className="mono">{g.relation}</span> on <span className="mono">{onText(g)}</span></details></td>
              <td className="sec">{g.source ? nameOf(w, w.byId.get(g.source)?.holder ?? g.issuer) : 'root'}</td>
              <td>{passesToAgents(g.pass_on) ? <span className="pass">yes</span> : <span className="dim">no</span>}</td>
              <td>
                {passesToAgents(g.pass_on) && agents.length ? (
                  <button className="btn" data-act="delegate" data-g={g.id} onClick={() => shell.openDrawer(<Delegate w={w} source={g} done={reload} />)}>
                    Give to an agent…
                  </button>
                ) : null}
              </td>
            </tr>
          )) : <tr><td colSpan={4} className="dim">Nothing yet.</td></tr>}
        </tbody>
      </table>
    </div>
    <div>
      <OwnAccount />
      <div className="card" id="signin-identities">
        <h2>Sign-in identities</h2>
        <div className="note" style={{ margin: '2px 0 6px' }}>Accounts that prove you are you. Never lent to an agent.</div>
        {me.sign_in_identities.map((login) => (
          <SignInIdentity key={login.provider + ' ' + login.subject} login={login} current={same(login)} />
        ))}
        <div className="note" style={{ marginTop: 6 }}>
          These are the sign-in accounts linked to your identity.
        </div>
      </div>
      <div className="card" id="service-accounts">
        <h2>Your service-account records</h2><a className="btn" href="#/service-accounts">Manage service accounts</a>
        <div className="note" style={{ margin: '2px 0 6px' }}>These records name accounts. Permission to use or lend their credentials is checked separately.</div>
        {me.service_accounts.length ? (
          me.service_accounts.map((s) => (
            <div className="row" style={{ alignItems: 'flex-start' }} key={s.id}>
              <span>{s.name}<p className="note">{s.description}</p><details><summary>Account record</summary><p className="mono">{s.id}</p><p>Owner: {s.owner}</p></details></span>
              <span>{s.state === 'retired' ? 'Retired' : s.state === 'active' ? 'Registered' : s.state}</span>
            </div>
          ))
        ) : (
          <p className="note">No service-account records were returned for you.</p>
        )}
      </div>
      <div className="card">
        <h2>Secrets available to you</h2>
        <div className="note" style={{ marginTop: 6 }}>View the secrets you can access, their permissions and their activity. Secret values are never displayed here.</div>
        <a className="btn" style={{ marginTop: 8 }} href="#/secrets">Open secrets</a>
      </div>
    </div>
  </div>;
}

function Agents({ data }: { data: ActiveData }) {
  const shell = useShell();
  const navigate = useNavigate();
  const { agents, tree, sessions, w } = data;
  const held = (id: string) =>
    w.list.grants.filter((g) => g.holder === id && g.standing.stands).map((g) => `${g.relation} of ${onText(g)}`).join('; ');
  const [folded, setFolded] = useState(readFolded);
  const fold = (team: string) => {
    const next = new Set(folded);
    if (next.has(team)) next.delete(team); else next.add(team);
    setPref(FOLDED, [...next].join(','));
    setFolded(next);
  };
  const byId = new Map<string, Entry>();
  const walk = (node: Node) => { byId.set(node.entry.id, node.entry); node.branches.forEach((branch) => branch.members.forEach(walk)); };
  walk(tree);
  const running = sessions.filter((session) => session.shown !== 'stopped' && session.agent && byId.has(session.agent));
  const watching = (session: string) => shell.watched.some((entry) => entry.session === session);
  const toggleWatch = (session: RuntimeSession, name: string) => watching(session.session)
    ? shell.unwatch(session.session)
    : shell.watch({ session: session.session, agent: session.agent, name, machine: session.machine_name ?? session.machine });
  return <div className="you-body">
    <div className="you-panel">
      <div className="section-h">
        <span>Your agents</span>
        <button className="btn" data-act="commission" onClick={() => navigate('/directory/manage?action=agent')}>Register an agent</button>
      </div>
      <div className="you-scroll">
        <table className="you-tree">
          <tbody>
            {agents.length
              ? <AgentRows branches={tree.branches} level={0} sessions={sessions} held={held} folded={folded} fold={fold} open={(id) => navigate('/file/' + id)} watching={watching} toggleWatch={toggleWatch} />
              : <tr><td className="dim">None.</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
    <div className="you-panel">
      <div className="section-h"><span>Running now</span><a href="#/runtime">Open the terminals</a></div>
      <div className="you-scroll">
        {running.length
          ? <div className="you-grid" aria-label="Running agents">{running.map((session) => {
            const name = byId.get(session.agent!)?.display_name ?? session.agent!;
            const machine = session.machine_name ?? session.machine;
            return <div className="you-peek" key={session.session}>
              <Peek session={session.session} name={name} machine={machine} />
              <button type="button" className="you-watch" aria-pressed={watching(session.session)} onClick={() => toggleWatch(session, name)}>{watching(session.session) ? 'Watching' : 'Watch'}</button>
            </div>;
          })}</div>
          : <p className="note you-quiet">Nothing running.</p>}
      </div>
    </div>
  </div>;
}

function Page({ data, reload }: { data: ActiveData; reload: () => void }) {
  const { search } = useLocation();
  const tab = new URLSearchParams(search).get('tab') === 'account' ? 'account' : 'agents';
  return (
    <div className="page fill you-page">
      <div className="head">
        <div>
          <div className="eyebrow">Signed in as</div>
          <h1>{data.me.person.display_name}</h1>
        </div>
      </div>
      <Waiting waiting={data.waiting} />
      <div className="tabs">
        <a href="#/me" className={tab === 'agents' ? 'on' : undefined}>Agents</a>
        <a href="#/me?tab=account" className={tab === 'account' ? 'on' : undefined}>Account</a>
      </div>
      {tab === 'account' ? <div className="pane"><Account data={data} reload={reload} /></div> : <Agents data={data} />}
    </div>
  );
}

function Registered({ me }: { me: MeView }) {
  return <div className="page fill">
    <div className="head"><div><div className="eyebrow">Signed in as</div>
      <h1>{me.person.display_name}</h1>
      <p>Your account is waiting for activation by an administrator.</p>
    </div></div>
    <div className="pane">
    <OwnAccount readOnly />
    <section className="card" aria-label="Your signed-in sessions">
      <h2>Your signed-in sessions</h2>
      <SessionList person="" />
    </section>
    </div>
  </div>;
}

export function You() {
  const [version, setVersion] = useState(0);
  const load = useLoad(readYou, 'me' + version);
  return <Gate load={load} title="Signed in as" ok={(data) => data.kind === 'registered' ? <Registered me={data.me} /> : <Page data={data} reload={() => setVersion((v) => v + 1)} />} />;
}
