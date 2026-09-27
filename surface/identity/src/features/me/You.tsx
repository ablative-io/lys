import { api, useLoad } from '../../api';
import type { AgentSummary, Login, MeView } from '../../generated';
import { keyable } from '../../shell/keyable';
import { useShell } from '../../shell/ShellContext';
import { useNavigate } from 'react-router';
import { Gate } from '../signin/Gate';
import { NotBuilt } from '../file/sections';

interface YouData {
  me: MeView;
  agents: AgentSummary[];
}

async function readYou(): Promise<YouData> {
  const [me, own] = await Promise.all([api.me(), api.ownPeople()]);
  const self = own.people.find((p) => p.id === me.person.id);
  return { me, agents: (self?.agents ?? []).filter((a) => a.state !== 'retired') };
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
      <span className="mono dim">{login.subject}</span>
    </div>
  );
}

function Page({ data }: { data: YouData }) {
  const shell = useShell();
  const navigate = useNavigate();
  const { me, agents } = data;
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
              <tr><td colSpan={5} className="dim"><span className="open-q">not built yet</span> Grants arrive with DIRECTORY-006 R1 to R5.</td></tr>
            </tbody>
          </table>
          <div className="section-h">
            <span>Your agents</span>
            <button className="btn primary" data-act="commission" onClick={() => shell.toast('Registering an agent from this screen is not built yet')}>
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
                    <td className="sec"><span className="dim">access not built yet</span></td>
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
              Several sign-in providers per person needs our fork of the sign-in service. <span className="open-q">not decided</span>
            </div>
          </div>
          <div className="card" id="service-accounts">
            <h2>Service accounts you may use</h2>
            <div className="note" style={{ margin: '2px 0 6px' }}>Accounts in other systems you are authorised to act through. Using one is not the same as being allowed to pass it on.</div>
            {me.service_accounts.length ? (
              me.service_accounts.map((s) => (
                <div className="row" style={{ alignItems: 'flex-start' }} key={s.system + s.account}>
                  <span>{s.system} <span className="mono dim">{s.account}</span></span>
                  <span className={s.may_pass_on ? 'pass' : 'dim'} style={{ textAlign: 'right', maxWidth: '46%', fontSize: 12 }}>
                    {s.may_pass_on ? 'passable to agents, as a virtual credential' : 'no: use only'}
                  </span>
                </div>
              ))
            ) : (
              <NotBuilt>The directory records no service accounts yet (conformance 1.3), so none is listed.</NotBuilt>
            )}
          </div>
          <div className="card">
            <h2>Your personal secrets</h2>
            <NotBuilt>The secrets store is SECRETS-002.</NotBuilt>
            <div className="note" style={{ marginTop: 6 }}>Yours alone. An agent uses one only through a virtual credential you issue.</div>
            <a className="btn" style={{ marginTop: 8 }} href="#/secrets">Open secrets</a>
          </div>
        </div>
      </div>
    </div>
  );
}

export function You() {
  const load = useLoad(readYou, 'me');
  return <Gate load={load} title="Signed in as" ok={(data) => <Page data={data} />} />;
}
