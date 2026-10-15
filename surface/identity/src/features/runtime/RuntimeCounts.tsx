/** Directory counters measure returned runtime reports; an unavailable read never becomes zero. */
import { request, useLoad } from '../../api';
import { problemWords } from '../people/Words';
import type { RuntimeSession } from './RuntimeSessions';
export function RuntimeCounts() {
  const load = useLoad(async () => {
    const answer = await request<{ sessions: RuntimeSession[] }>('/runtime/sessions');
    if (!Array.isArray(answer.sessions) || answer.sessions.some((entry) => !['unconfirmed', 'running', 'stopped'].includes(entry.shown))) throw new Error('The runtime did not answer a valid session list.');
    return { found: answer.sessions.filter((entry) => entry.agent === null).length, running: answer.sessions.filter((entry) => entry.shown === 'running').length };
  }, 'directory-runtime-counts');
  return <>{(['found', 'running'] as const).map((kind) => <div className="stat" key={kind}><div className="n">{load.status === 'ok' ? load.data[kind] : '—'}</div><div className="l">{kind === 'found' ? 'sessions with no agent' : 'sessions running'}</div>{load.status === 'refused' ? <p className="why-not" title={load.refused.refusal.refusal + ': ' + load.refused.refusal.reason}>{problemWords(load.refused)}</p> : load.status === 'loading' ? <p>Reading runtime reports…</p> : null}</div>)}</>;
}
