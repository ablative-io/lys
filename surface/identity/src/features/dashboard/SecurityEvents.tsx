import { useLive, request } from '../../api';
import { DirectoryGate } from '../people/Words';
import { isRecord } from '../network/contract';

interface Event { id: string; kind: string; at: number; detail: string }
interface Events { administrator: boolean; events: Event[] }

async function read(): Promise<Events> {
  const value = await request<unknown>('/security-events');
  const kinds = ['first_run_code_issued', 'password_code_issued', 'password_reset_started', 'password_reset_completed'];
  if (!isRecord(value) || typeof value.administrator !== 'boolean' || !Array.isArray(value.events)
    || !value.events.every((event) => isRecord(event) && typeof event.id === 'string' && Boolean(event.id)
      && typeof event.kind === 'string' && kinds.includes(event.kind)
      && typeof event.at === 'number' && Number.isSafeInteger(event.at) && event.at >= 0 && event.at <= 8_640_000_000_000
      && typeof event.detail === 'string' && Boolean(event.detail.trim()))) {
    throw new Error('RecoveryEventsUnreadable: the service did not answer complete security events.');
  }
  return value as unknown as Events;
}

export function SecurityEvents() {
  const load = useLive(read, 'security-events');
  return <DirectoryGate load={load} title="Security events" ok={(answer) => answer.administrator && answer.events.length
    ? <section className="card" aria-label="Security events"><h2>Security events</h2>
      <ul>{answer.events.map((event) => <li key={event.id}>
        <time dateTime={new Date(event.at * 1000).toISOString()}>{new Date(event.at * 1000).toLocaleString()}</time>
        <p>{event.detail}</p>
      </li>)}</ul>
    </section> : null} />;
}
