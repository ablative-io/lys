import { useEffect, useRef, useState } from 'react';
import { api, Refused, request } from '../../api';
import { Act } from '../../shell/Act';

type Resource = { kind: string; id: string; relation: string; actions: string[]; seats: string[]; evidence: string };
type Plan = { version: 1; resources: Resource[]; agents: { display_name: string }[] };
type Receipt = { operation: string; grant: string; receipt: { caller: string } };

export function planOf(value: unknown): Plan {
  const plan = value as Plan;
  if (!plan || plan.version !== 1 || !Array.isArray(plan.resources) || !Array.isArray(plan.agents)) throw new Error('Expected an estate approval plan');
  const names = new Set(plan.agents.map((agent) => agent.display_name));
  if (names.size !== plan.agents.length || [...names].some((name) => typeof name !== 'string' || !name.trim())) throw new Error('Invalid agent names');
  const seen = new Set<string>();
  for (const resource of plan.resources) {
    if (!resource || ![resource.kind, resource.id, resource.relation, resource.evidence].every((x) => typeof x === 'string' && x.trim())
      || !Array.isArray(resource.actions) || !resource.actions.length || resource.actions.some((a) => typeof a !== 'string' || !a.trim())
      || !Array.isArray(resource.seats) || !resource.seats.length || resource.seats.some((name) => !names.has(name))) throw new Error('Invalid resource or seat');
    if (![resource.kind,resource.id,resource.relation,...resource.actions].every((token)=>/^[a-z0-9_.-]+$/.test(token))) throw new Error('Invalid resource token: '+resource.id+'; use lowercase letters, digits, _, - or .');
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
  const [account, setAccount] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  useEffect(() => {
    let current = true;
    void request<{plan:unknown;loader:string}>('/identity/estate-plan').then((answer)=>{
      const parsed=planOf(answer.plan);
      if(!/^op-[0-9a-f]{32}$/.test(answer.loader))throw new Error('The installed loader was not identified');
      if(current){setPlan(parsed);setAccount(answer.loader);}
    }).catch((error)=>{if(current)setStatus(error instanceof Refused ? error.refusal.refusal+': '+error.refusal.reason : String(error));});
    return ()=>{current=false;};
  },[]);
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
      setStatus('Applying the approved grants to the agents…');
      const result=await request<{by:{kind:string;id:string};completed:{entry:string;operation:string}[]}>('/identity/estate-apply',{agents:plan.agents,delegations});
      const expected=[...plan.agents.map((a)=>'agents/'+a.display_name),...delegations.map((d)=>'delegations/'+(d as {name:string}).name)];
      if(result.by?.kind!=='service_account'||result.by.id!==account||!Array.isArray(result.completed)||result.completed.length!==expected.length||result.completed.some((r,i)=>r.entry!==expected[i]||!/^op-[0-9a-f]{32}$/.test(r.operation)))throw new Error('The import answer did not confirm every planned entry');
      setStatus('Estate grants applied: '+delegations.length+' delegations recorded for '+plan.agents.length+' agents.');
    } catch (error) { setStatus((error instanceof Refused ? error.refusal.refusal+': '+error.refusal.reason : String(error)) + '. Stopped; no automatic retry. Some earlier grants may be recorded. An explicit repeat of the same plan and account uses the same operation IDs.'); }
    finally { working.current=false;setBusy(false); }
  };
  return <div className="page fill">
    <div className="head"><div><div className="eyebrow">Access</div><h1>Approve estate grants</h1></div>
      {plan ? <Act symbol="approve" name="Approve and apply estate grants" word="Approve" tone="primary" disabled={busy || !account} onClick={()=>void approve()} /> : null}</div>
    <p className="sub">Review the installed estate plan, then approve as yourself. Opening this page does not write anything. Each root is held by you; your installed loader passes only these actions to the listed agents. Apps must already be approved and agents active.</p>
    {status ? <p role="status">{status}</p> : null}
    {plan ? <div className="pane">
      <p className="note">Using your installed Lys directory loader. {plan.resources.length} resources. No expiry of their own. You can revoke the roots or loader grants later.</p>
      <table><thead><tr><th>Resource</th><th>Kind</th><th>Actions</th><th>Passed to</th><th>Why</th></tr></thead>
        <tbody>{plan.resources.map((r)=><tr key={r.kind+'/'+r.id}><td><strong>{r.id}</strong></td><td className="sec">{r.kind}</td><td className="mono">{r.actions.join(', ')}</td><td className="sec">{r.seats.join(', ')}</td><td className="dim">{r.evidence}</td></tr>)}</tbody></table>
    </div> : null}
  </div>;
}
