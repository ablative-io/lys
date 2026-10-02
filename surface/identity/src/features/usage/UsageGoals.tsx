/** An agent's goals, expectations and deliverables, and the form that sets one. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { GoalItem } from './contract';

type Props = { agent: string; goals: GoalItem[]; changed: (words: string) => void };

const STANDING = { open: 'Open', met: 'Met', missed: 'Missed', dropped: 'Dropped' };

export function UsageGoals({ agent, goals, changed }: Props) {
  return <section className="card usage-goals" aria-label="Goals"><h3>Goals</h3>
    {goals.length ? <table className="usage-list"><thead><tr><th>Kind</th><th>What</th><th>Deadline</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>{goals.map((item) => <tr key={item.goal.id}><td>{item.goal.kind}</td><td>{item.goal.words}</td>
        <td>{item.goal.deadline === null ? 'No deadline' : new Date(item.goal.deadline * 1000).toLocaleString()}</td><td>{STANDING[item.standing]}</td><td><GoalEdits agent={agent} item={item} changed={changed} /></td></tr>)}</tbody></table> : <p>No goal is set for this agent.</p>}
    <SetGoal agent={agent} changed={changed} />
  </section>;
}

function GoalEdits({ agent, item, changed }: { agent: string; item: GoalItem; changed: (words: string) => void }) {
  const goal = item.goal;
  const [words, setWords] = useState(goal.words);
  const key = 'lys.pending.goal.' + agent + '.' + goal.id;
  const path = '/goals/' + encodeURIComponent(goal.id);
  const reword = useRoleChange<GoalItem>(key + '.words', path + '/words',
    (answer, body) => answer?.goal?.id === goal.id && answer.goal.words === body.words,
    () => changed('Goal reworded.'));
  const activity = useRoleChange<GoalItem>(key + '.active', path + '/active',
    (answer, body) => answer?.goal?.id === goal.id && answer.goal.active === body.active,
    (answer) => changed(answer.goal.active ? 'Goal switched on.' : 'Goal switched off.'));
  const blocked = reword.blocked || activity.blocked || item.standing !== 'open';
  const next = words.trim();
  const valid = next.length > 0 && next.length <= 500 && !/\p{Cc}/u.test(next);
  return <div>
    <label>Active<input type="checkbox" aria-label={'Goal active ' + goal.id} checked={goal.active} disabled={blocked}
      onChange={(event) => { if (!blocked) activity.submit({ operation: operationId(), active: event.target.checked }); }} /></label>
    <ChangeStatus change={activity} />
    <form aria-label={'Reword goal ' + goal.id} onSubmit={(event) => {
      event.preventDefault(); if (!blocked && valid && next !== goal.words) reword.submit({ operation: operationId(), words: next });
    }}>
      <label className="field">Words<input name="words" value={words} required maxLength={500} disabled={blocked} onChange={(event) => setWords(event.target.value)} /></label>
      <button className="btn" type="submit" disabled={blocked || !valid || next === goal.words}>Reword goal</button>
      <ChangeStatus change={reword} />
    </form>
    {item.standing !== 'open' ? <p>Only open goals can be changed.</p> : null}
  </div>;
}

function SetGoal({ agent, changed }: { agent: string; changed: (words: string) => void }) {
  const [words, setWords] = useState('');
  const [deadline, setDeadline] = useState('');
  const [dated, setDated] = useState(false);
  const path = '/agents/' + encodeURIComponent(agent) + '/goals';
  const change = useRoleChange<GoalItem>('lys.pending.goal.' + agent, path, (answer, body) => answer.goal.id === body.operation && answer.goal.words === body.words, () => { setWords(''); setDeadline(''); setDated(false); changed('Goal kept.'); });
  const seconds = dated ? Math.floor(Date.parse(deadline) / 1000) : null;
  const ready = !!words.trim() && (seconds === null || Number.isFinite(seconds));
  const submit = () => {
    if (!ready || change.blocked) return;
    change.submit({ operation: operationId(), kind: 'goal', words: words.trim(), deadline: seconds });
  };
  return <form aria-label="Set a goal" onSubmit={(event) => { event.preventDefault(); submit(); }}><h4>Set a goal</h4>
    <label className="field usage-what">What the agent is reminded of<textarea name="words" rows={4} value={words} required maxLength={500} disabled={change.blocked} placeholder="Type a goal and press Enter" onChange={(event) => setWords(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); submit(); } }} /></label>
    {dated
      ? <label className="field">Deadline<input name="deadline" type="datetime-local" required value={deadline} disabled={change.blocked} onChange={(event) => setDeadline(event.target.value)} /></label>
      : <button className="btn" type="button" disabled={change.blocked} onClick={() => setDated(true)}>Add a deadline</button>}
    <button className="btn primary" type="submit" disabled={change.blocked || !ready}>Set goal</button><ChangeStatus change={change} />
  </form>;
}
