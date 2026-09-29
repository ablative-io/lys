/** An agent's tool-boundary policy: the deny rules its next launch is judged under, kept as numbered versions. */
import { useState } from 'react';
import { Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { KINDS } from './policyContract';
import type { Policy, PolicyView, Rule, RuleKind } from './policyContract';

const path = (id: string) => '/agents/' + encodeURIComponent(id) + '/policy';

function lifts(rule: Rule): string {
  if (rule.authority === 'hard') return 'Nobody: no grant can lift it';
  const { resource, action } = rule.authority.permission;
  return 'A grant of ' + action + ' on ' + resource.kind + ' ' + resource.id;
}

function said(failure: unknown): string {
  if (failure instanceof Refused && failure.refusal.refusal === 'PolicyVersionConflict') return 'Someone else changed this policy first. Refresh to see their version, then make your change again.';
  if (failure instanceof Refused) return failure.refusal.reason;
  return String(failure);
}

function RuleForm({ onAdd }: { onAdd: (rule: Rule) => void }) {
  const [kind, setKind] = useState<RuleKind>('tool');
  const [grantable, setGrantable] = useState(false);
  return (
    <form aria-label="Add a rule" className="card" onSubmit={(event) => {
      event.preventDefault();
      const form = new FormData(event.currentTarget);
      const value = (name: string) => String(form.get(name) ?? '').trim();
      const rule: Rule = {
        id: value('id'), tool: value('tool'), kind,
        authority: grantable ? { permission: { resource: { kind: value('resource_kind'), id: value('resource_id') }, action: value('action') } } : 'hard',
      };
      if (kind !== 'tool') rule.target = value('target');
      onAdd(rule);
      event.currentTarget.reset();
      setKind('tool');
      setGrantable(false);
    }}>
      <h3>Add a rule</h3>
      <label>Rule name <input name="id" required /></label>
      <label>Tool, exactly as the harness names it <input name="tool" required /></label>
      <label>Denies <select name="kind" value={kind} onChange={(event) => setKind(event.target.value as RuleKind)}>
        {Object.entries(KINDS).map(([k, words]) => <option key={k} value={k}>{words}</option>)}
      </select></label>
      {kind !== 'tool' ? <label>{kind === 'host' ? 'Host' : 'Absolute path'} <input name="target" required /></label> : null}
      <label><input type="checkbox" name="grantable" checked={grantable} onChange={(event) => setGrantable(event.target.checked)} /> A grant may lift it</label>
      {grantable ? <>
        <label>Resource kind <input name="resource_kind" required /></label>
        <label>Resource id <input name="resource_id" required /></label>
        <label>Action <input name="action" required /></label>
      </> : null}
      <button className="btn" type="submit">Add rule</button>
    </form>
  );
}

function Editor({ id, view, onKept }: { id: string; view: PolicyView; onKept: () => void }) {
  const [rules, setRules] = useState<Rule[]>(view.policy?.rules ?? []);
  const [failure, setFailure] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const version = view.policy?.version ?? 0;
  return (
    <>
      <section className="card">
        <h3>{view.policy ? 'Version ' + view.policy.version : 'No policy set'}</h3>
        <p className="note">A kept version {view.applies}.</p>
        {view.digest ? <p className="note mono">Digest {view.digest}</p> : null}
        {rules.length ? (
          <table><thead><tr><th>Rule</th><th>Tool</th><th>Denies</th><th>Who can lift it</th><th /></tr></thead>
            <tbody>{rules.map((rule, index) => (
              <tr key={rule.id + ':' + index}>
                <td className="mono">{rule.id}</td><td>{rule.tool}</td>
                <td>{KINDS[rule.kind]}{rule.target ? ': ' + rule.target : ''}</td><td>{lifts(rule)}</td>
                <td><button className="btn" type="button" onClick={() => setRules(rules.filter((_, at) => at !== index))}>Remove</button></td>
              </tr>
            ))}</tbody>
          </table>
        ) : <p>No rules: this agent's tool calls are not denied by a policy.</p>}
        <button className="btn" type="button" disabled={saving} onClick={async () => {
          setSaving(true);
          setFailure(null);
          try {
            await request<Policy>(path(id), { version, rules });
            onKept();
          } catch (error) {
            setFailure(said(error));
          } finally {
            setSaving(false);
          }
        }}>Keep as version {version + 1}</button>
        {failure ? <p role="alert">{failure}</p> : null}
      </section>
      <RuleForm onAdd={(rule) => setRules([...rules, rule])} />
    </>
  );
}

export function AgentPolicy({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const view = await request<PolicyView>(path(id));
    if (view.agent !== id) throw new Error('The policy answer did not name this agent.');
    return view;
  }, 'agent-policy:' + id + ':' + revision);
  return (
    <>
      <div className="head"><div><h2>Tool policy</h2><p>The tool calls this agent is refused before they run.</p></div></div>
      <Gate load={load} title="Tool policy" ok={(view) => <Editor key={view.policy?.version ?? 0} id={id} view={view} onKept={() => setRevision((value) => value + 1)} />} />
    </>
  );
}
