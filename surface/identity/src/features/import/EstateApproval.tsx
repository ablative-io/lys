import { useEffect, useRef, useState } from 'react';
import { api, Refused, request } from '../../api';
import { Act } from '../../shell/Act';
import { PageHeader } from '../../shell/PageHeader';
import { TableScroll } from '../../shell/Listing';
import { AccessTabs } from '../access/AccessTabs';
import { ErrorWords } from '../people/Words';
import { kindWords } from '../people/AddPerson';

const count = (n: number, one: string, many: string) => n.toLocaleString('en-AU') + ' ' + (n === 1 ? one : many);

type Resource = { kind: string; id: string; relation: string; actions: string[]; seats: string[]; evidence: string };
type Plan = { version: 1; resources: Resource[]; agents: { display_name: string }[] };
type Receipt = { operation: string; grant: string; receipt: { caller: string } };
/** A resource's evidence as the plan writes it: the plan's producer owns its sentences, and Lys rewrites no app's words. */
export const evidenceWords = (resource: Resource): string => resource.evidence.trim();

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
    // Kinds, relations and actions are lowercase; an id is named as its product names it, case kept (the served token rule).
    if (![resource.kind,resource.relation,...resource.actions].every((token)=>/^[a-z0-9_.-]+$/.test(token)) || !/^[A-Za-z0-9_.-]+$/.test(resource.id)) throw new Error('Invalid resource token: '+resource.id+'; use lowercase letters, digits, _, - or . (an id may also use uppercase letters)');
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
    throw new Error('EstateAnswerUnconfirmed: The answer did not confirm this operation. Stop and reconcile ' + id);
  }
  return answer;
}

/** Loading previews only. The signed-in person explicitly approves the shown roots. */
export function EstateApproval() {
  const [plan, setPlan] = useState<Plan | null>(null);
  const [account, setAccount] = useState('');
  const [status, setStatus] = useState('');
  // What stopped the plan being read or approved: said in words, with its name and the service's reason under Error details.
  const [problem, setProblem] = useState<unknown>(null);
  // No plan installed at all is not a fault: it is said as the page's one sentence.
  const [none, setNone] = useState(false);
  const [unread, setUnread] = useState(false);
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  useEffect(() => {
    let current = true;
    void request<{plan:unknown;loader:string}>('/identity/estate-plan').then((answer)=>{
      const parsed=planOf(answer.plan);
      if(!/^op-[0-9a-f]{32}$/.test(answer.loader))throw new Error('The installed loader was not identified');
      if(current){setPlan(parsed);setAccount(answer.loader);}
    }).catch((error: unknown)=>{
      if(!current)return;
      if(error instanceof Refused && error.status===404)setNone(true);
      else { setUnread(true); setProblem(error); }
    });
    return ()=>{current=false;};
  },[]);
  const approve = async () => {
    if (!plan || !account || working.current) return;
    working.current = true; setBusy(true); setProblem(null); setStatus('Recording your approval…');
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
        setStatus('Recorded ' + (index + 1) + ' of ' + count(plan.resources.length, 'resource', 'resources') + '.');
      }
      setStatus('Applying the approved grants to the agents…');
      const result=await request<{by:{kind:string;id:string};completed:{entry:string;operation:string}[]}>('/identity/estate-apply',{agents:plan.agents,delegations});
      const expected=[...plan.agents.map((a)=>'agents/'+a.display_name),...delegations.map((d)=>'delegations/'+(d as {name:string}).name)];
      if(result.by?.kind!=='service_account'||result.by.id!==account||!Array.isArray(result.completed)||result.completed.length!==expected.length||result.completed.some((r,i)=>r.entry!==expected[i]||!/^op-[0-9a-f]{32}$/.test(r.operation)))throw new Error('EstateAnswerUnconfirmed: The import answer did not confirm every planned entry');
      setStatus('Estate grants applied: '+count(delegations.length,'delegation','delegations')+' recorded for '+count(plan.agents.length,'agent','agents')+'.');
    } catch (error) {
      setStatus('Stopped, with no automatic retry. Some of the access may already be given. Approving the same plan again repeats the same steps and gives nothing twice.');
      setProblem(error);
    }
    finally { working.current=false;setBusy(false); }
  };
  return <div className="page fill">
    <PageHeader title="Approve estate grants" subtitle="Lys was installed with a plan of what your agents may use in your other apps. Review it below, then approve it as yourself. Opening this page changes nothing. You hold each access, and Lys's installed loader passes only the listed actions on to the listed agents. The apps must already be approved and the agents active." actions={plan ? <Act symbol="approve" name="Approve and apply estate grants" word="Approve" tone="primary" disabled={busy || !account} onClick={()=>void approve()} /> : null} />
    <AccessTabs on="grants" />
    {status ? <p role="status">{status}</p> : null}
    {unread ? <p className="why-not">The installed estate plan could not be read, so nothing can be approved here.</p> : null}
    {problem ? <ErrorWords problem={problem} /> : null}
    {none ? <p className="note">This Lys was installed without an estate plan, so there is nothing to approve here. Give access from <a href="#/access">Access</a> instead.</p> : null}
    {!plan && !problem && !none ? <p role="status" className="dim">Reading the installed plan…</p> : null}
    {plan ? <div className="pane">
      <p className="note">{count(plan.resources.length, 'thing', 'things')} to give access to, for {count(plan.agents.length, 'agent', 'agents')}. The access has no end date of its own; you can take it back from Access at any time.</p>
      <TableScroll label="Estate grants"><table><thead><tr><th>What</th><th>Kind</th><th>What the agents may do</th><th>Given to</th><th>Why it is in the plan</th></tr></thead>
        <tbody>{plan.resources.map((r)=><tr key={r.kind+'/'+r.id+'/'+r.relation}><td><strong style={{ overflowWrap: 'anywhere' }}>{r.id}</strong></td><td className="sec">{kindWords(r.kind)}</td><td>{r.actions.join(', ')}</td><td className="sec">{r.seats.join(', ')}</td><td className="dim" title={r.evidence}>{evidenceWords(r)}</td></tr>)}</tbody></table></TableScroll>
    </div> : null}
  </div>;
}
