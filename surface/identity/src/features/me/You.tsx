/** The front page fills the screen and never scrolls as a whole: what waits for you on the top line, your agents as a tree under their teams on the left, the running ones as small live pictures on the right, and your account under a second tab. */
import { Fragment, useEffect, useRef, useState } from 'react';
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
import { Start } from '../team/Start';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import { Stop } from '../team/AgentRun';
import '../team/team.css';
import { Gate } from '../signin/Gate';
import { OwnAccount } from '../people/Account';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import { buildTree } from '../people/tree';
import type { Branch, Node } from '../people/tree';
import type { AccessRequest } from '../requests/contract';
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

/** What the person asked of one agent from its row: start it, or stop it. Shown in a row of its own under the agent. */
export interface Asked { agent: string; what: 'start' | 'stop' | 'started'; pressed: boolean }

/** Start, where the agent is listed. Whether the person may approve its settings is read here, when it is first shown. Its settings have one home, the Settings tab of its own page. */
function StartHere({ entry, me, pressed, changed, close }: { entry: Entry; me: string; pressed: boolean; changed: () => void; close: () => void }) {
  const navigate = useNavigate();
  const load = useLoad(() => api.people(), 'you-start:' + entry.id);
  if (load.status === 'loading') return null;
  return <div className="you-start-here">
    <Start entry={entry} me={me} admin={load.status === 'ok' && load.data.scope === 'directory'} changed={changed} straightAway={pressed}
      settings={() => navigate('/file/' + encodeURIComponent(entry.id) + '/provisioning')} />
    <button type="button" className="you-watch" data-act="close" onClick={close}>Close</button>
  </div>;
}

/** What a stopped agent's row offers: Start when it has a program chosen, Set up when it has none, so no button says Start that cannot start. */
function RowStart({ entry, ask }: { entry: Entry; ask: (next: Asked) => void }) {
  const load = useLoad(() => request<ProvisioningAnswer>('/agents/' + encodeURIComponent(entry.id) + '/provisioning'), 'you-row:' + entry.id);
  if (load.status === 'loading') return null;
  const unset = load.status === 'ok' && !load.data.profile?.harness;
  return unset
    ? <a className="you-watch you-start" data-act="setup" href={'#/file/' + encodeURIComponent(entry.id) + '/provisioning'} onClick={(event) => event.stopPropagation()}>Set up</a>
    : <button type="button" className="you-watch you-start" data-act="start" onClick={(event) => { event.stopPropagation(); ask({ agent: entry.id, what: 'start', pressed: true }); }}>Start</button>;
}

function AgentRows({ branches, level, sessions, held, folded, fold, open, me, asked, ask, changed }: {
  branches: Branch[]; level: number; sessions: RuntimeSession[]; held: (id: string) => string; folded: Set<string>; fold: (team: string) => void; open: (id: string) => void;
  me: string; asked: Asked | null; ask: (next: Asked | null) => void; changed: () => void;
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
            <td className="you-act">{session ? <Fragment>
              <a className="you-watch" data-act="watch" href={'#/canvas/' + encodeURIComponent(member.entry.id)} onClick={(event) => event.stopPropagation()}>Watch</a>
              <button type="button" className="you-watch" data-act="stop" onClick={(event) => { event.stopPropagation(); ask({ agent: member.entry.id, what: 'stop', pressed: true }); }}>Stop</button>
            </Fragment> : asked?.agent === member.entry.id && asked.what !== 'stop' ? null : <RowStart entry={member.entry} ask={ask} />}</td>
          </tr>
          {asked?.agent === member.entry.id && (asked.what === 'stop' ? session : !session) ? <tr className="you-asked"><td colSpan={4}>
            {asked.what === 'stop' && session
              ? <Stop entry={member.entry} session={session} changed={changed} done={() => ask(null)} />
              : asked.what === 'started'
              // The start was answered; the row offers no second Start while the page reads where it is running.
              ? <p role="status" className="team-start-line">{member.entry.display_name} has started. Reading where it is running…</p>
              : <StartHere key={member.entry.id + (asked.pressed ? ':pressed' : '')} entry={member.entry} me={me} pressed={asked.pressed} changed={() => { ask({ agent: member.entry.id, what: 'started', pressed: false }); changed(); }} close={() => ask(null)} />}
          </td></tr> : null}
          <AgentRows branches={member.branches} level={level + 1} sessions={sessions} held={held} folded={folded} fold={fold} open={open} me={me} asked={asked} ask={ask} changed={changed} />
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
              <td>{mayText(w, g)}</td>
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
        <h2>Your service-account records</h2><a className="btn" href="#/people/view/accounts">Manage service accounts</a>
        <div className="note" style={{ margin: '2px 0 6px' }}>These records name accounts. Permission to use or lend their credentials is checked separately.</div>
        {me.service_accounts.length ? (
          me.service_accounts.map((s) => (
            <div className="row" style={{ alignItems: 'flex-start' }} key={s.id}>
              <span>{s.name}<p className="note">{s.description}</p></span>
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

function Agents({ data, reload }: { data: ActiveData; reload: () => void }) {
  const navigate = useNavigate();
  const [asked, setAsked] = useState<Asked | null>(null);
  // A started row waits only for the next read of the page. When that read arrives the row says what is so: Watch and Stop if it runs, Start again if it does not.
  useEffect(() => { setAsked((held) => held?.what === 'started' ? null : held); }, [data]);
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
  return <div className="you-body">
    <div className="you-panel">
      <div className="section-h">
        <span>Your agents</span>
      </div>
      <div className="you-scroll">
        <table className="you-tree">
          <tbody>
            {agents.length
              ? <AgentRows branches={tree.branches} level={0} sessions={sessions} held={held} folded={folded} fold={fold} open={(id) => navigate('/file/' + id)} me={data.me.person.id} asked={asked} ask={setAsked} changed={reload} />
              : <tr><td className="dim">None.</td></tr>}
          </tbody>
        </table>
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
      {tab === 'account' ? <div className="pane"><Account data={data} reload={reload} /></div> : <Agents data={data} reload={reload} />}
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
  const read = useLoad(readYou, 'me' + version);
  // While the page reads itself again after a start or a stop, what it last showed stays; it never blanks.
  const last = useRef<typeof read | null>(null);
  if (read.status === 'ok') last.current = read;
  const load = read.status === 'loading' && last.current ? last.current : read;
  return <Gate load={load} title="Signed in as" ok={(data) => data.kind === 'registered' ? <Registered me={data.me} /> : <Page data={data} reload={() => setVersion((v) => v + 1)} />} />;
}
