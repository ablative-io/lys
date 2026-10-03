/** An agent's or a team's goals, expectations and deliverables as one table: each row is changed where it stands, and the last row adds one. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { GoalItem } from './contract';

type Props = { agent: string; kind?: 'agent' | 'team'; goals: GoalItem[]; changed: (words: string) => void };

const STANDING = { open: 'Open', met: 'Met', missed: 'Missed', dropped: 'Dropped' };

const COLUMNS = ['12%', '44%', '20%', '10%', '14%'];

export function UsageGoals({ agent, kind = 'agent', goals, changed }: Props) {
  return <section className="card usage-goals" aria-label="Goals"><h3>Goals</h3>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Kind</th><th>What the {kind} is reminded of</th><th>Deadline</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>
        {goals.map((item) => <GoalRow key={item.goal.id} agent={agent} item={item} changed={changed} />)}
        {goals.length ? null : <tr><td colSpan={5} className="dim">No goal is set for this {kind}.</td></tr>}
      </tbody>
      <tfoot><tr><td colSpan={5} className="usage-add"><SetGoal agent={agent} kind={kind} changed={changed} /></td></tr></tfoot>
    </table>
  </section>;
}

function GoalRow({ agent, item, changed }: { agent: string; item: GoalItem; changed: (words: string) => void }) {
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
  const form = 'reword-' + goal.id;
  return <tr>
    <td>{goal.kind}</td>
    <td>
      <form id={form} aria-label={'Reword goal ' + goal.id} onSubmit={(event) => {
        event.preventDefault(); if (!blocked && valid && next !== goal.words) reword.submit({ operation: operationId(), words: next });
      }}>
        <input name="words" aria-label={'Words of goal ' + goal.id} value={words} required maxLength={500} disabled={blocked} onChange={(event) => setWords(event.target.value)} />
      </form>
      <ChangeStatus change={reword} />
    </td>
    <td>{goal.deadline === null ? 'No deadline' : new Date(goal.deadline * 1000).toLocaleString()}</td>
    <td>{STANDING[item.standing]}</td>
    <td>
      <label className="tick">Active<input type="checkbox" aria-label={'Goal active ' + goal.id} checked={goal.active} disabled={blocked}
        onChange={(event) => { if (!blocked) activity.submit({ operation: operationId(), active: event.target.checked }); }} /></label>
      <button className="btn" type="submit" form={form} disabled={blocked || !valid || next === goal.words}>Reword goal</button>
      <ChangeStatus change={activity} />
      {item.standing !== 'open' ? <p className="note">Only open goals can be changed.</p> : null}
    </td>
  </tr>;
}

/** The table's last row: the same columns, filled in to add a goal. An empty deadline is no deadline. */
function SetGoal({ agent, kind, changed }: { agent: string; kind: 'agent' | 'team'; changed: (words: string) => void }) {
  const [words, setWords] = useState('');
  const [deadline, setDeadline] = useState('');
  const path = '/' + kind + 's/' + encodeURIComponent(agent) + '/goals';
  const change = useRoleChange<GoalItem>('lys.pending.goal.' + agent, path, (answer, body) => answer.goal.id === body.operation && answer.goal.words === body.words, () => { setWords(''); setDeadline(''); changed('Goal kept.'); });
  const seconds = deadline ? Math.floor(Date.parse(deadline) / 1000) : null;
  const ready = !!words.trim() && (seconds === null || Number.isFinite(seconds));
  const submit = () => {
    if (!ready || change.blocked) return;
    change.submit({ operation: operationId(), kind: 'goal', words: words.trim(), deadline: seconds });
  };
  return <form aria-label="Set a goal" className="usage-add-row" style={{ gridTemplateColumns: COLUMNS.slice(0, 3).join(' ') + ' 24%' }} onSubmit={(event) => { event.preventDefault(); submit(); }}>
    <span className="sec">goal</span>
    <textarea name="words" aria-label={'What the ' + kind + ' is reminded of'} rows={4} value={words} required maxLength={500} disabled={change.blocked} placeholder="Type a goal and press Enter" onChange={(event) => setWords(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); submit(); } }} />
    <input name="deadline" aria-label="Deadline, empty for none" type="datetime-local" value={deadline} disabled={change.blocked} onChange={(event) => setDeadline(event.target.value)} />
    <span><button className="btn primary" type="submit" disabled={change.blocked || !ready}>Set goal</button></span>
    <ChangeStatus change={change} />
  </form>;
}
