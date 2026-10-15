/** An agent's schedules (AGENTS-001 R5): each with its cadence, next due instant, occurrences and every delivery's state, why one stopped, pause, resume and stop, and a row that sets one on this agent. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
import { clock } from '../file/time';
import { SLOTS } from './contract';
import type { ScheduleItem } from './contract';

type Props = { agent: string; schedules: ScheduleItem[]; changed: (words: string) => void };

const COLUMNS = ['30%', '20%', '16%', '18%', '16%'];

/** Where each delivery of an occurrence stands, as a person says it. */
const SENT: Record<string, string> = { pending: 'waiting to be sent', accepted: 'accepted, not yet delivered', delivered: 'delivered', uncertain: 'delivery not confirmed', refused: 'not delivered' };

/** A datetime-local value as seconds since the epoch, or null when it is not one. */
function seconds(local: string): number | null {
  const ms = Date.parse(local);
  return Number.isFinite(ms) ? Math.floor(ms / 1000) : null;
}

function cadence(item: ScheduleItem): string {
  const schedule = item.schedule;
  const every = schedule.interval ? 'every ' + Math.round(schedule.interval / 60) + ' min' : 'once';
  const most = schedule.max_occurrences ? ', at most ' + schedule.max_occurrences : '';
  const until = schedule.until ? ', until ' + clock(schedule.until) : '';
  return 'From ' + clock(schedule.at) + ', ' + every + most + until;
}

function standing(item: ScheduleItem): string {
  if (item.stopped) return 'Stopped: ' + item.stopped.reason;
  const paused = item.changes?.slice().reverse().find((change) => 'paused' in change.change);
  if (paused && paused.change.paused === true) return 'Paused';
  return item.next_due === null ? 'Nothing due' : 'Next ' + clock(item.next_due);
}

export function Schedules({ agent, schedules, changed }: Props) {
  const mine = schedules.filter((item) => item.schedule.recipients.some((recipient) => recipient.kind === 'agent' && recipient.id === agent));
  const add = useRoleChange<ScheduleItem>('lys.pending.schedules.' + agent + '.add', '/schedules',
    (answer, body) => answer?.schedule?.id === body.operation,
    () => changed('Schedule set.'));
  const [at, setAt] = useState('');
  const [minutes, setMinutes] = useState('');
  const [most, setMost] = useState('');
  const [text, setText] = useState('');
  const when = seconds(at);
  const whole = (given: string) => given === '' || (/^\d+$/.test(given) && Number(given) >= 1);
  // Why Set cannot be pressed yet, said once anything has been typed.
  const why = !text.trim() && !at ? '' : !text.trim() ? 'Write what is sent.' : when === null ? 'Choose when it is first sent.'
    : when <= Date.now() / 1000 ? 'Choose a first time still to come.' : !whole(minutes) ? 'Every must be a whole number of minutes, or empty to send it once.'
    : !whole(most) ? 'At most must be a whole number of times, or empty for no limit.' : '';
  const valid = when !== null && when > Date.now() / 1000 && text.trim().length > 0 && whole(minutes) && whole(most);
  return <section className="card usage-schedules" aria-label="Schedules"><h3>Schedules</h3>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>What is sent</th><th>When</th><th>Where it stands</th><th>Sent so far</th><th>Change</th></tr></thead>
      <tbody>
        {mine.map((item) => <ScheduleRow key={item.schedule.id} item={item} changed={changed} />)}
        {mine.length ? null : <tr><td colSpan={5} className="dim">No schedule sends to this agent.</td></tr>}
      </tbody>
      <tfoot><tr>
        <td><input form="set-schedule" name="text" aria-label="Schedule text" placeholder="What to send" value={text} required disabled={add.blocked} onChange={(event) => setText(event.target.value)} /></td>
        <td><input form="set-schedule" type="datetime-local" name="at" aria-label="First instant" title="When it is first sent" value={at} required disabled={add.blocked} onChange={(event) => setAt(event.target.value)} /></td>
        <td><input form="set-schedule" name="interval" inputMode="numeric" aria-label="Every, in minutes" placeholder="Every … min" title="Minutes between sends; empty to send it once" value={minutes} disabled={add.blocked} onChange={(event) => setMinutes(event.target.value)} /></td>
        <td><input form="set-schedule" name="max_occurrences" inputMode="numeric" aria-label="At most" placeholder="At most … times" title="How many times at most; empty for no limit" value={most} disabled={add.blocked} onChange={(event) => setMost(event.target.value)} /></td>
        <td><Act form="set-schedule" symbol="add" type="submit" name="Set this schedule" disabled={add.blocked || !valid} /></td>
      </tr>{why ? <tr><td colSpan={5}><p className="why-not" role="alert">{why}</p></td></tr> : null}</tfoot>
    </table>
    <form id="set-schedule" aria-label="Set a schedule" onSubmit={(event) => {
      event.preventDefault();
      if (add.blocked || !valid || when === null) return;
      const body: Record<string, unknown> = {
        operation: operationId(), at: when,
        recipients: [{ kind: 'agent', id: agent }],
        source: { kind: 'text', text: text.trim() },
      };
      if (minutes !== '') body.interval = Number(minutes) * 60;
      if (most !== '') body.max_occurrences = Number(most);
      add.submit(body);
    }}>
      <ChangeStatus change={add} />
    </form>
  </section>;
}

function ScheduleRow({ item, changed }: { item: ScheduleItem; changed: (words: string) => void }) {
  const id = item.schedule.id;
  const [confirming, setConfirming] = useState(false);
  const path = '/schedules/' + encodeURIComponent(id);
  const key = 'lys.pending.schedules.' + id;
  const paused = standing(item) === 'Paused';
  const pause = useRoleChange<ScheduleItem>(key + '.pause', path + '/change',
    (answer, body) => answer?.schedule?.id === id && (answer.changes ?? []).some((change) => change.operation === body.operation),
    () => changed(paused ? 'Schedule resumed.' : 'Schedule paused.'));
  const stop = useRoleChange<ScheduleItem>(key + '.stop', path + '/stop',
    (answer) => answer?.schedule?.id === id && Boolean(answer.stopped),
    () => changed('Schedule stopped.'));
  const blocked = pause.blocked || stop.blocked || Boolean(item.stopped);
  const source = item.schedule.source.kind === 'text' ? item.schedule.source.text : 'The ' + SLOTS[item.schedule.source.slot].toLowerCase() + ' words';
  return <tr>
    <td>{source}</td>
    <td>{cadence(item)}</td>
    <td>{standing(item)}</td>
    <td>{item.fired.length ? <ul className="usage-sent">{item.fired.map((fired) => <li key={fired.operation}>{fired.occurrence}: due {clock(fired.due)}{fired.coalesced > 1 ? ' (' + fired.coalesced + ' times due at once, sent once)' : ''}: {fired.sent.map((sent) => SENT[sent.state] ?? 'not known').join(', ') || (fired.refused ? 'not sent' : 'nobody to send to')}</li>)}</ul> : 'None yet'}</td>
    <td>
      {item.stopped ? <span className="dim">Stopped for good</span> : <>
      <Act symbol="again" name={(paused ? 'Resume' : 'Pause') + ' the schedule “' + source + '”'} word={paused ? 'Resume' : 'Pause'} disabled={blocked} onClick={() => pause.submit({ operation: operationId(), change: { field: 'paused', paused: !paused } })} />
      {confirming ? <span className="usage-confirm"><span>Stop it for good? It cannot be started again.</span>
        <Act symbol="decline" name={'Stop the schedule “' + source + '” for good'} word="Stop for good" tone="danger" disabled={blocked} onClick={() => { setConfirming(false); stop.submit({ words: 'stopped from the Usage screen' }); }} />
        <Act symbol="close" name="Keep the schedule" word="Keep it" onClick={() => setConfirming(false)} /></span>
        : <Act symbol="decline" name={'Stop the schedule “' + source + '”'} word="Stop" disabled={blocked} onClick={() => setConfirming(true)} />}
      </>}
      <ChangeStatus change={pause} />
      <ChangeStatus change={stop} />
    </td>
  </tr>;
}
