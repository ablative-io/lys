/** What runtimes have reported for an agent, counted; nothing is inferred about a process no runtime reported. */
import { request, useLoad } from '../../api';
import type { RuntimeSession } from '../runtime/RuntimeSessions';

export function NowSummary({ id }: { id: string }) {
  const load = useLoad(() => request<{ sessions: RuntimeSession[] }>('/agents/' + encodeURIComponent(id) + '/runtime/sessions'), 'now:' + id);
  if (load.status === 'loading') return <span className="dim">…</span>;
  if (load.status === 'refused') return <span className="why-not">{load.refused.refusal.refusal}: {load.refused.message}</span>;
  const open = load.data.sessions.filter((session) => session.shown !== 'stopped');
  if (!open.length) return <>No runtime has reported a session. This does not say nothing is running.</>;
  const running = open.filter((session) => session.shown === 'running').length;
  return <>{running} reported running, {open.length - running} unconfirmed. <a href={'#/file/' + id + '/sessions'}>Sessions</a></>;
}
