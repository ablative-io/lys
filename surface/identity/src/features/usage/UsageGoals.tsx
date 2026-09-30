/** An agent's goals, expectations and deliverables, and the form that sets one. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { GoalItem, GoalKind } from './contract';

type Props = { agent: string; goals: GoalItem[]; changed: (words: string) => void };

const STANDING = { open: 'Open', met: 'Met', missed: 'Missed', dropped: 'Dropped' };

export function UsageGoals({ agent, goals, changed }: Props) {
  return <section className="card usage-goals" aria-label="Goals"><h3>Goals</h3>
    {goals.length ? <table className="usage-list"><thead><tr><th>Kind</th><th>What</th><th>Deadline</th><th>Where it stands</th></tr></thead>
      <tbody>{goals.map((item) => <tr key={item.goal.id}><td>{item.goal.kind}</td><td>{item.goal.words}</td>
        <td>{item.goal.deadline === null ? 'No deadline' : new Date(item.goal.deadline * 1000).toLocaleString()}</td><td>{STANDING[item.standing]}</td></tr>)}</tbody></table> : <p>No goal is set for this agent.</p>}
    <SetGoal agent={agent} changed={changed} />
  </section>;
}

function SetGoal({ agent, changed }: { agent: string; changed: (words: string) => void }) {
  const [kind, setKind] = useState<GoalKind>('goal');
  const [words, setWords] = useState('');
  const [deadline, setDeadline] = useState('');
  const [evidence, setEvidence] = useState<'commit' | 'document' | 'check'>('commit');
  const path = '/agents/' + encodeURIComponent(agent) + '/goals';
  const change = useRoleChange<GoalItem>('lys.pending.goal.' + agent, path, (answer, body) => answer.goal.id === body.operation && answer.goal.words === body.words, () => changed('Goal kept.'));
  const submit = () => {
    const seconds = Math.floor(Date.parse(deadline) / 1000);
    if (!words.trim() || !Number.isFinite(seconds)) return;
    change.submit({ operation: operationId(), kind, words: words.trim(), deadline: seconds, ...(kind === 'deliverable' ? { evidence } : {}) });
  };
  return <form aria-label="Set a goal" onSubmit={(event) => { event.preventDefault(); submit(); }}><h4>Set a goal</h4>
    <label className="field">Kind<select value={kind} disabled={change.blocked} onChange={(event) => setKind(event.target.value as GoalKind)}><option value="goal">Goal</option><option value="expectation">Expectation</option><option value="deliverable">Deliverable</option></select></label>
    <label className="field usage-what">What<textarea name="words" rows={4} value={words} required maxLength={500} disabled={change.blocked} onChange={(event) => setWords(event.target.value)} /></label>
    <label className="field">Deadline<input name="deadline" type="datetime-local" required value={deadline} disabled={change.blocked} onChange={(event) => setDeadline(event.target.value)} /></label>
    {kind === 'deliverable' ? <label className="field">Proved by<select value={evidence} disabled={change.blocked} onChange={(event) => setEvidence(event.target.value as 'commit' | 'document' | 'check')}><option value="commit">A landed commit</option><option value="document">A document</option><option value="check">A passing check</option></select></label> : null}
    <button className="btn primary" type="submit" disabled={change.blocked || !words.trim() || !deadline}>Set goal</button><ChangeStatus change={change} />
  </form>;
}
