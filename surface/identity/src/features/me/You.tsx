import { useState } from 'react';
import { api, useLoad } from '../../api';
import type { AgentSummary, Login, MeView } from '../../generated';
import { Delegate } from '../grants/Delegate';
import { nameOf, onText, passesToAgents, readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { keyable } from '../../shell/keyable';
import { useShell } from '../../shell/ShellContext';
import { useNavigate } from 'react-router';
import { Gate } from '../signin/Gate';

interface YouData {
  me: MeView;
  agents: AgentSummary[];
  w: GrantWorld;
}

async function readYou(): Promise<YouData> {
  const [w, own] = await Promise.all([readGrantWorld(), api.ownPeople()]);
  const self = own.people.find((p) => p.id === w.me.person.id);
  return { me: w.me, agents: (self?.agents ?? []).filter((a) => a.state !== 'retired'), w };
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

function Page({ data, reload }: { data: YouData; reload: () => void }) {
  const shell = useShell();
  const navigate = useNavigate();
  const { me, agents, w } = data;
  const mine = w.list.grants.filter((g) => g.holder === me.person.id && g.standing.stands);
  const held = (id: string) =>
    w.list.grants.filter((g) => g.holder === id && g.standing.stands).map((g) => `${g.relation} of ${onText(g)}`).join('; ');
  const same = (a: Login) => a.provider === me.signed_in.provider && a.subject === me.signed_in.subject;
  return (
    <div className="page">
      <div className="head">
        <div>
          <div className="eyebrow">Signed in as</div>
          <h1>{me.person.display_name}</h1>
          <p className="sub">Your accounts, what you hold, and what you have given your agents. You can only pass on what you hold and are allowed to pass on.</p>
        </div>
      </div>
      <div className="grid2">
        <div>
          <div className="section-h" style={{ marginTop: 0 }}><span>What you hold</span></div>
          <table>
            <thead><tr><th>Relation</th><th>On</th><th>From</th><th>You may pass it on</th><th></th></tr></thead>
            <tbody>
              {mine.length ? mine.map((g) => (
                <tr key={g.id}>
                  <td className="mono" style={{ color: 'var(--accent)' }}>{g.relation}</td>
                  <td className="mono">{onText(g)}</td>
                  <td className="sec">{g.source ? nameOf(w, w.byId.get(g.source)?.holder ?? g.issuer) : 'root'}</td>
                  <td>{passesToAgents(g.pass_on) ? <span className="pass">yes, to agents</span> : <span className="dim">no</span>}</td>
                  <td>
                    {passesToAgents(g.pass_on) && agents.length ? (
                      <button className="btn" data-act="delegate" data-g={g.id} onClick={() => shell.openDrawer(<Delegate w={w} source={g} done={reload} />)}>
                        Give to an agent…
                      </button>
                    ) : null}
                  </td>
                </tr>
              )) : <tr><td colSpan={5} className="dim">Nothing yet.</td></tr>}
            </tbody>
          </table>
          <div className="section-h">
            <span>Your agents</span>
            <button className="btn primary" data-act="commission" onClick={() => navigate('/directory/manage?action=agent')}>
              Register an agent
            </button>
          </div>
          <table>
            <tbody>
              {agents.length ? agents.map((a) => {
                const open = () => navigate('/file/' + a.id);
                return (
                  <tr key={a.id} data-href={'#/file/' + a.id} onClick={open} {...keyable(open)}>
                    <td>{a.display_name}</td>
                    <td><span className={'dot s-' + a.state} />{a.state}</td>
                    <td className="sec">{held(a.id) || 'no access'}</td>
                  </tr>
                );
              }) : <tr><td className="dim">None.</td></tr>}
            </tbody>
          </table>
        </div>
        <div>
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
      </div>
    </div>
  );
}

export function You() {
  const [version, setVersion] = useState(0);
  const load = useLoad(readYou, 'me' + version);
  return <Gate load={load} title="Signed in as" ok={(data) => <Page data={data} reload={() => setVersion((v) => v + 1)} />} />;
}
