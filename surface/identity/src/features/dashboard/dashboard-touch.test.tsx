/**
 * On a touch screen nothing that explains is left in a tooltip: who acknowledged an alarm and when is said in the
 * cell, and no panel of the dashboard carries a `title`.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Alarms } from '../dashboard/Alarms';
import type { Alarm } from '../dashboard/Alarms';
import type { DashboardAgent } from '../dashboard/contract';
import { Exceptions } from '../dashboard/Exceptions';
import { Spend } from '../dashboard/Spend';
import { WaitingOnYou } from '../dashboard/WaitingOnYou';
import { clock } from '../file/time';

const alarm = (acknowledged: boolean): Alarm => ({ id: 'alarm-1', kind: 'seat_stalled', session: 's', agent: 'agent-1', machine: 'm', raised_at: 1_790_000_000,
  detail: 'no report for a while', acknowledged_by: acknowledged ? 'person-ada' : null, acknowledged_at: acknowledged ? 1_790_000_600 : null, cleared_at: null });
const agent = { agent: { id: 'agent-1', display_name: 'Helper', state: 'active' }, teams: [], sessions: [],
  usage: { agent: 'agent-1', used: [], receipts: [], last_reported_ms: null },
  budget: { holder: { kind: 'agent', id: 'agent-1' }, limits: [{ unit: 'tokens', amount: 10, period: 'day', act: 'tell' }], warn_at: null, zone: 'UTC', version: 1, by: null, at: null,
    used: [{ unit: 'tokens', period: 'day', since_ms: 0, figure: 10, unavailable: null }], unavailable: [], within: [], unconfirmed: [] },
  goals: { goals: [] } } as unknown as DashboardAgent;

describe('Nothing explained only in a tooltip', () => {
  it('says when an alarm was acknowledged in the cell, and never shows who by identifier', () => {
    const markup = renderToStaticMarkup(<Alarms summary={{ open_count: 1, newest: [alarm(true)] }} agents={[agent]} />);
    expect(markup).toContain('Acknowledged ' + clock(1_790_000_600));
    expect(markup).not.toContain('person-ada');
  });

  it('draws the dashboard’s side panels with no title attribute', () => {
    const markup = renderToStaticMarkup(<>
      <WaitingOnYou waiting={[{ kind: 'review', id: 'grant-1', asked_by: { id: 'agent-1', name: 'Helper' }, about: 'Check the access Helper holds: viewer on project logo', asked_at: '2026-10-18T01:00:00Z', can_decide: true }]} me="person-ada" />
      <Exceptions rows={[agent]} /><Spend rows={[agent]} /></>);
    expect(markup).toContain('Needs a look');
    expect(markup).not.toMatch(/\btitle="/);
  });
});
