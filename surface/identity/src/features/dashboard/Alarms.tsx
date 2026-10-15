import { useEffect, useState } from 'react';
import { request, Refused } from '../../api';
import { Act } from '../../shell/Act';
import { SecurityEvents } from './SecurityEvents';
import { ErrorWords } from '../people/Words';
import { refusedPart, validAlarm } from './contract';
import type { DashboardAgent, PartRefused } from './contract';

export interface Alarm {
  id: string; kind: 'seat_stalled' | 'session_ungoverned'; session: string; agent: string;
  machine: string; raised_at: number; detail: string; acknowledged_by: string | null;
  acknowledged_at: number | null; cleared_at: number | null;
}
export interface AlarmSummary { open_count: number; newest: Alarm[] }

/** A failure as the service named it; one the screen found itself is named here, and shown in words, never by this name. */
function problem(error: unknown): PartRefused {
  return error instanceof Refused ? error.refusal : { refusal: 'AlarmsUnreadable', reason: error instanceof Error ? error.message : String(error) };
}

function shownAlarm(reported: Alarm, local: Alarm | undefined): Alarm {
  if (reported.cleared_at !== null || !local) return reported;
  if (local.cleared_at !== null) return local;
  return reported.acknowledged_by ? reported : { ...reported,
    acknowledged_by: local.acknowledged_by, acknowledged_at: local.acknowledged_at,
  };
}

export function Alarms(props: { summary: AlarmSummary | PartRefused; agents: DashboardAgent[] }) {
  return <><SecurityEvents /><SessionAlarms {...props} /></>;
}

function SessionAlarms({ summary, agents }: { summary: AlarmSummary | PartRefused; agents: DashboardAgent[] }) {
  const [acknowledged, setAcknowledged] = useState<Record<string, Alarm>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [failure, setFailure] = useState<PartRefused | null>(null);
  const [all, setAll] = useState<Alarm[] | null>(null);
  useEffect(() => { setAll(null); }, [summary]);
  async function acknowledge(alarm: Alarm) {
    setBusy(alarm.id); setFailure(null);
    try {
      const answer = await request<Alarm>('/alarms/' + encodeURIComponent(alarm.id) + '/acknowledge', {});
      if (!validAlarm(answer) || answer.id !== alarm.id || !answer.acknowledged_by || answer.acknowledged_at === null) throw new Error('The service did not record the acknowledgement.');
      setAcknowledged((prior) => ({ ...prior, [alarm.id]: answer }));
    } catch (error) { setFailure(problem(error)); }
    finally { setBusy(null); }
  }
  async function readAll() {
    setFailure(null);
    try {
      const answer = await request<{ alarms: Alarm[] }>('/alarms');
      if (!Array.isArray(answer.alarms) || !answer.alarms.every(validAlarm)) throw new Error('The service did not answer complete alarm records.');
      setAll(answer.alarms);
    } catch (error) { setFailure(problem(error)); }
  }
  const reported = refusedPart(summary) ? [] : all ?? summary.newest;
  const rows = reported.map((alarm) => shownAlarm(alarm, acknowledged[alarm.id]));
  const recovered = reported.filter((alarm, index) => alarm.cleared_at === null && rows[index].cleared_at !== null).length;
  const count = refusedPart(summary) ? '?' : all === null ? summary.open_count - recovered
    : rows.filter((alarm) => alarm.cleared_at === null).length;
  return <section className="dash-widget" aria-label="Alarms">
    <div className="section-h"><span>Alarms</span><b className="dash-count">{count}</b>
      {!refusedPart(summary) && all === null ? <Act symbol="more" name="Show all" word="Show all" onClick={() => void readAll()} /> : null}</div>
    <div className="you-scroll">
      {refusedPart(summary) ? <><p className="why-not">Lys could not read the alarms.</p><ErrorWords problem={new Refused(0, summary)} /></>
        : <table className="dash-table"><tbody>{rows.map((alarm) => {
          return <tr key={alarm.id} data-alarm={alarm.id}>
            <td>{agents.find((row) => row.agent.id === alarm.agent)?.agent.display_name ?? 'Agent name unavailable'}
              <div className="sec">{alarm.kind === 'seat_stalled' ? 'Stopped reporting in: it may be stuck' : 'Its model calls are not going through Lys, so its use is not counted'}</div>
              <div className="note">{alarm.detail}</div></td>
            <td className="you-act">{alarm.cleared_at !== null ? 'Cleared' : alarm.acknowledged_by ? <span title={alarm.acknowledged_by}>Acknowledged</span>
              : <Act symbol="approve" name="Acknowledge" word="Acknowledge" disabled={busy !== null} onClick={() => void acknowledge(alarm)} />}</td>
          </tr>;
        })}{rows.length ? null : <tr className="empty"><td>No open alarms.</td></tr>}</tbody></table>}
      {failure ? <ErrorWords problem={new Refused(0, failure)} /> : null}
    </div>
  </section>;
}
