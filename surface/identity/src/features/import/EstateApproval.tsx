import { useRef, useState } from 'react';
import { api, request } from '../../api';

type Resource = { kind: string; id: string; relation: string; actions: string[]; seats: string[]; evidence: string };
type Plan = { version: 1; resources: Resource[]; agents: { display_name: string }[] };
type Receipt = { operation: string; grant: string; receipt: { caller: string } };
type Account = { id: string; owner: string; name: string; state: string };

function planOf(value: unknown): Plan {
  const plan = value as Plan;
  if (!plan || plan.version !== 1 || !Array.isArray(plan.resources) || !Array.isArray(plan.agents)
    || plan.resources.length > 250 || plan.agents.length > 100) throw new Error('Expected an estate approval plan');
  const names = new Set(plan.agents.map((agent) => agent.display_name));
  if (names.size !== plan.agents.length || [...names].some((name) => typeof name !== 'string' || !name.trim())) throw new Error('Invalid agent names');
  const seen = new Set<string>();
  for (const resource of plan.resources) {
    if (!resource || ![resource.kind, resource.id, resource.relation, resource.evidence].every((x) => typeof x === 'string' && x.trim())
      || !Array.isArray(resource.actions) || !resource.actions.length || resource.actions.some((a) => typeof a !== 'string' || !a.trim())
      || !Array.isArray(resource.seats) || !resource.seats.length || resource.seats.some((name) => !names.has(name))) throw new Error('Invalid resource or seat');
    const key = JSON.stringify([resource.kind, resource.id, resource.relation]);
    if (seen.has(key)) throw new Error('Repeated resource');
    seen.add(key);
  }
  return plan;
}

async function operation(caller: string, purpose: string, body: object): Promise<string> {
  const bytes = new TextEncoder().encode(JSON.stringify(['lys/estate-approval/v1', caller, purpose, body]));
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return 'op-' + [...digest.slice(0, 16)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

async function recorded(path: string, caller: string, purpose: string, body: object): Promise<Receipt> {
  const id = await operation(caller, purpose, body);
  const answer = await request<Receipt>(path, { operation: id, ...body });
  if (answer.operation !== id || answer.receipt?.caller !== caller || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
    throw new Error('The answer did not confirm this operation. Stop and reconcile ' + id);
  }
  return answer;
}

/** Loading previews only. The signed-in person explicitly approves the shown roots. */
export function EstateApproval() {
  const [plan, setPlan] = useState<Plan | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [account, setAccount] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const read = async (file: File) => {
    setPlan(null); setStatus(''); setAccount('');
    try {
      if (file.size > 2_000_000) throw new Error('Plan is too large');
      const parsed = planOf(JSON.parse(await file.text()));
      const [me, result] = await Promise.all([api.me(), request<{service_accounts: Account[]}>('/service-accounts')]);
      setAccounts(result.service_accounts.filter((a) => a.owner === me.person.id && a.state === 'active'));
      setPlan(parsed);
    } catch (error) { setStatus(String(error)); }
  };
  const approve = async () => {
    if (!plan || !account || working.current) return;
    working.current = true; setBusy(true); setStatus('Recording your approval…');
    try {
      const me = await api.me();
      const all = await api.people();
      for (const name of plan.agents.map((a) => a.display_name)) {
        const matches = all.people.flatMap((p) => p.agents.map((agent) => ({...agent, owner:p.id}))).filter((a) => a.display_name === name);
        if (matches.length !== 1 || matches[0].owner !== me.person.id || matches[0].state !== 'active') throw new Error(name + ' must be one active agent owned by you');
      }
      const delegations: object[] = [];
      for (const [index, resource] of plan.resources.entries()) {
        const shared = { route:'browser', resource:{kind:resource.kind,id:resource.id}, relation:resource.relation, window:{starts_at:0,ends_at:null} };
        const root = await recorded('/grants/roots', me.person.id, 'root', {...shared, holder:me.person.id, pass_on:{kind:'to',actions:resource.actions,recipients:['agent','service_account']}});
        const given = await recorded('/grants', me.person.id, 'loader', {...shared, source:root.grant, recipient:account, responsible:me.person.id, pass_on:{kind:'to',actions:resource.actions,recipients:['agent']}});
        for (const name of resource.seats) delegations.push({ name:resource.kind + '/' + resource.id + '/' + resource.relation + '/' + name, route:'api', source:given.grant, recipient:'@agent/' + name, responsible:'@owner', resource:shared.resource, relation:resource.relation, pass_on:{kind:'use_only'}, window:shared.window });
        setStatus('Recorded ' + (index + 1) + ' of ' + plan.resources.length + ' resources');
      }
      const blob = new Blob([JSON.stringify({agents:plan.agents,delegations},null,2)+'\n'], {type:'application/json'});
      const url = URL.createObjectURL(blob); const link=document.createElement('a');link.href=url;link.download='estate-grants.json';link.click();setTimeout(() => URL.revokeObjectURL(url), 1000);
      setStatus('Your roots and loader delegations are recorded. Downloaded estate-grants.json; import it with the selected loader’s credential.');
    } catch (error) { setStatus(String(error) + '. Stopped; no automatic retry. Some earlier roots may be recorded. An explicit repeat of the same plan and account uses the same operation IDs.'); }
    finally { working.current=false;setBusy(false); }
  };
  return <section className="page"><h1>Approve estate grants</h1>
    <p>Load a sourced plan, inspect every resource and action, then approve as yourself. Loading does not write anything. Each root is held by you; the selected service account may pass only these actions to the listed agents. Apps must already be approved and agents active.</p>
    <input type="file" accept="application/json,.json" disabled={busy} onChange={(event) => {const file=event.target.files?.[0];if(file) void read(file);}} />
    {plan ? <><label className="field">Loader service account<select value={account} disabled={busy} onChange={(event)=>setAccount(event.target.value)}><option value="">Choose your loader</option>{accounts.map((a)=><option key={a.id} value={a.id}>{a.name} · {a.id}</option>)}</select></label>
      <p>{plan.resources.length} resources. No expiry of their own. You can revoke the roots or loader grants later.</p>
      <ul>{plan.resources.map((r)=><li key={r.kind+'/'+r.id}><strong>{r.id}</strong> ({r.kind}): {r.actions.join(', ')} → {r.seats.join(', ')}. <small>{r.evidence}</small></li>)}</ul>
      <button className="btn primary" disabled={busy || !account} onClick={()=>void approve()}>Approve these roots and delegate to my loader</button></> : null}
    {status ? <p role="status">{status}</p> : null}
  </section>;
}
