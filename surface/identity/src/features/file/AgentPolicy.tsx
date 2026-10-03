/** An agent's tool-boundary policy: the deny rules its next launch is judged under, kept as numbered versions. */
import { useState } from 'react';
import { Refused, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import { KINDS } from './policyContract';
import type { PolicyView, Rule, RuleKind } from './policyContract';

const path = (id: string) => '/agents/' + encodeURIComponent(id) + '/policy';

function lifts(rule: Rule): string {
  if (rule.authority === 'hard') return 'Nobody: no grant can lift it';
  const { resource, action } = rule.authority.permission;
  return 'A grant of ' + action + ' on ' + resource.kind + ' ' + resource.id;
}

function said(failure: unknown): string {
  if (failure instanceof Refused && failure.refusal.refusal === 'PolicyVersionConflict') return 'PolicyVersionConflict: Someone else changed these rules.';
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
      <label>Tool name, exactly as the agent’s program names it <input name="tool" required /></label>
      <label>Denies <select name="kind" value={kind} onChange={(event) => setKind(event.target.value as RuleKind)}>
        {Object.entries(KINDS).map(([k, words]) => <option key={k} value={k}>{words}</option>)}
      </select></label>
      {kind !== 'tool' ? <label>{kind === 'host' ? 'Host' : 'Absolute path'} <input name="target" required /></label> : null}
      <label><input type="checkbox" name="grantable" checked={grantable} onChange={(event) => setGrantable(event.target.checked)} /> An access permission may allow this call</label>
      {grantable ? <>
        <label>Type of thing the permission covers <input name="resource_kind" required /></label>
        <label>Name of the thing the permission covers <input name="resource_id" required /></label>
        <label>Action <input name="action" required /></label>
      </> : null}
      <button className="btn" type="submit">Add rule</button>
    </form>
  );
}

function Editor({ id, view, onKept }: { id: string; view: PolicyView; onKept: (answer: PolicyView) => void }) {
  const [rules, setRules] = useState<Rule[]>(view.policy?.rules ?? []);
  const [failure, setFailure] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const version = view.policy?.version ?? 0;
  return (
    <>
      <section className="card">
        <h3>{view.policy ? 'Version ' + view.policy.version : 'No policy set'}</h3>
        <p className="note">A kept version {view.applies}.</p>
        {view.digest ? <p className="note">Saved rules <span className="mono">{view.digest}</span></p> : null}
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
            const answer = await request<PolicyView>(path(id), { version, rules });
            if (answer.agent !== id || answer.policy?.agent !== id || answer.policy.version !== version + 1) throw new Error('The answer did not confirm the saved rules. Open this tab again to check the outcome.');
            onKept(answer);
          } catch (error) {
            setFailure(said(error));
          } finally {
            setSaving(false);
          }
        }}>Save rules for the next start</button>
        {failure ? <ErrorWords problem={failure} /> : null}
      </section>
      <RuleForm onAdd={(rule) => setRules([...rules, rule])} />
    </>
  );
}

export function AgentPolicy({ id }: { id: string }) {
  const [saved, setSaved] = useState<PolicyView | null>(null);
  const load = useLoad(async () => {
    const view = await request<PolicyView>(path(id));
    if (view.agent !== id) throw new Error('The policy answer did not name this agent.');
    return view;
  }, 'agent-policy:' + id);
  return (
    <>
      <p>Set rules that block tool calls when this agent next starts. Sessions already running keep their original rules.</p>
      <Gate load={load} title="Tool policy" ok={(view) => <Editor key={(saved ?? view).policy?.version ?? 0} id={id} view={saved ?? view} onKept={setSaved} />} />
    </>
  );
}
