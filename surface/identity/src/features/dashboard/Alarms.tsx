import { useEffect, useState } from 'react';
import { request, Refused } from '../../api';
import { refusedPart, validAlarm } from './contract';
import type { DashboardAgent, PartRefused } from './contract';

export interface Alarm {
  id: string; kind: 'seat_stalled' | 'session_ungoverned'; session: string; agent: string;
  machine: string; raised_at: number; detail: string; acknowledged_by: string | null;
  acknowledged_at: number | null; cleared_at: number | null;
}
export interface AlarmSummary { open_count: number; newest: Alarm[] }

function problem(error: unknown): PartRefused {
  return error instanceof Refused ? error.refusal : { refusal: 'AlarmsUnreadable', reason: String(error) };
}

export function Alarms({ summary, agents }: { summary: AlarmSummary | PartRefused; agents: DashboardAgent[] }) {
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
  const rows = refusedPart(summary) ? [] : all ?? summary.newest;
  return <section className="dash-widget" aria-label="Alarms">
    <div className="section-h"><span>Alarms</span><b className="dash-count">{refusedPart(summary) ? '?' : summary.open_count}</b>
      {!refusedPart(summary) && all === null ? <button type="button" className="btn" onClick={() => void readAll()}>Show all</button> : null}</div>
    <div className="you-scroll">
      {refusedPart(summary) ? <p className="why-not" role="alert" title={summary.reason}>{summary.refusal}</p>
        : <table className="dash-table"><tbody>{rows.map((reported) => {
          const local = acknowledged[reported.id];
          const alarm = local && !reported.acknowledged_by ? { ...reported,
            acknowledged_by: local.acknowledged_by, acknowledged_at: local.acknowledged_at,
          } : reported;
          return <tr key={alarm.id} data-alarm={alarm.id}>
            <td>{agents.find((row) => row.agent.id === alarm.agent)?.agent.display_name ?? 'Agent name unavailable'}
              <div className="sec">{alarm.kind === 'seat_stalled' ? 'Seat stopped signalling' : 'Session has no proxy usage'}</div>
              <div className="note">{alarm.detail}</div></td>
            <td className="you-act">{alarm.cleared_at !== null ? 'Cleared' : alarm.acknowledged_by ? <span title={alarm.acknowledged_by}>Acknowledged</span>
              : <button type="button" className="btn" disabled={busy !== null} onClick={() => void acknowledge(alarm)}>Acknowledge</button>}</td>
          </tr>;
        })}{rows.length ? null : <tr className="empty"><td>No open alarms.</td></tr>}</tbody></table>}
      {failure ? <p className="why-not" role="alert" title={failure.reason}>{failure.refusal}: {failure.reason}</p> : null}
    </div>
  </section>;
}
