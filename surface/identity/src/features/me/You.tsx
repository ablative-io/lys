/** You: who is signed in, their account's own pieces in one row, and what they hold across the page's width under it. The page fills the screen; each part scrolls inside itself. Their agents are on the Dashboard. */
import { useRef, useState } from 'react';
import { api, useLoad } from '../../api';
import type { AgentSummary, Login, MeView } from '../../generated';
import { GrantTable } from '../grants/GrantTable';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { Gate } from '../signin/Gate';
import { OwnAccount } from '../people/Account';
import { SessionList } from '../sessions/Sessions';
import './you.css';

interface ActiveData {
  kind: 'active';
  me: MeView;
  agents: AgentSummary[];
  w: GrantWorld;
}

type YouData = ActiveData | { kind: 'registered'; me: MeView };

async function readYou(): Promise<YouData> {
  const me = await api.me();
  if (me.person.state === 'registered') return { kind: 'registered', me };
  const [w, own] = await Promise.all([readGrantWorld(me), api.ownPeople()]);
  const self = own.people.find((p) => p.id === w.me.person.id);
  return { kind: 'active', me: w.me, agents: (self?.agents ?? []).filter((a) => a.state !== 'retired'), w };
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

/**
 * Your account first, as one row of equal height: the account's changes as rows, the signed-in sessions, and the two
 * short records one over the other; each scrolls inside itself. Under it, what you hold across the page's full width,
 * so a grant is one line. Every row of What you hold is yours and stands, so neither is said again on each row.
 * Secrets have their own place on the rail, so they are not repeated here; service accounts are linked from here
 * because nothing else on the page reaches them.
 */
function Account({ data, reload }: { data: ActiveData; reload: () => void }) {
  const { me, agents, w } = data;
  const mine = w.list.grants.filter((g) => g.holder === me.person.id && g.standing.stands);
  const same = (a: Login) => a.provider === me.signed_in.provider && a.subject === me.signed_in.subject;
  return <div className="you-account">
    <div className="you-account-row">
      <OwnAccount />
      <section className="card you-sessions" aria-label="Your signed-in sessions">
        <h2>Your signed-in sessions</h2>
        <div className="you-sessions-scroll"><SessionList person="" /></div>
      </section>
      <div className="card" id="signin-identities">
        <h2>Sign-in identities</h2>
        <div className="note" style={{ margin: '2px 0 6px' }}>Accounts that prove you are you. Never lent to an agent.</div>
        {me.sign_in_identities.map((login) => (
          <SignInIdentity key={login.provider + ' ' + login.subject} login={login} current={same(login)} />
        ))}
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
    </div>
    <section className="you-holds-pane" aria-label="What you hold">
      <div className="section-h" style={{ marginTop: 0 }}><span>What you hold</span><b className="you-count">{mine.length}</b></div>
      <div className="you-holds-scroll">
        <GrantTable w={w} grants={mine} done={reload} give={agents.length > 0} empty="Nothing yet." omit={['Holder', 'Stands']} />
      </div>
    </section>
  </div>;
}

function Page({ data, reload }: { data: ActiveData; reload: () => void }) {
  return (
    <div className="page fill you-page">
      <div className="head">
        <div>
          <div className="eyebrow">Signed in as</div>
          <h1>{data.me.person.display_name}</h1>
        </div>
      </div>
      <Account data={data} reload={reload} />
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
  // While the page reads itself again after a change, what it last showed stays; it never blanks.
  const last = useRef<typeof read | null>(null);
  if (read.status === 'ok') last.current = read;
  const load = read.status === 'loading' && last.current ? last.current : read;
  return <Gate load={load} title="Signed in as" ok={(data) => data.kind === 'registered' ? <Registered me={data.me} /> : <Page data={data} reload={() => setVersion((v) => v + 1)} />} />;
}
