/**
 * The terminal space for an agent chosen in the network tree that holds no seat on that computer and is not running:
 * the agent's own Start, the same one its page and the dashboard offer (POST /agents/{id}/start, in team/Start), so a
 * press here starts it the way it starts everywhere else. What stops a start is said there, before the press.
 */
import { api, useLoad } from '../../api';
import type { Entry } from '../people/directory';
import { Start } from '../team/Start';
import { said } from './canvas-kept';

/** Who is asking, whether they may approve settings, and the agent as the directory has it. */
async function readStart(agent: string): Promise<{ me: string; admin: boolean; entry: Entry }> {
  const [me, people, view] = await Promise.all([api.me(), api.people(), api.agent(agent)]);
  return {
    me: me.person.id,
    admin: people.scope === 'directory',
    entry: { id: view.id, display_name: view.display_name, state: view.state, kind: 'agent', role: null, person: view.person },
  };
}

export function NetworkAgentStart({ agent, name, reason }: { agent: string; name: string; reason: string | null }) {
  const load = useLoad(() => readStart(agent), 'network-agent-start:' + agent);
  return <section className="terminal network-terminal" aria-label={'Terminal of ' + name}>
    <header className="terminal-head"><h2 title={name}>{name}</h2><span role="status">not running</span></header>
    {reason ? <p className="network-reason" role="status">{reason}</p> : null}
    {load.status === 'ok' ? <Start key={agent} entry={load.data.entry} me={load.data.me} admin={load.data.admin}
      // The tree follows the service's change feed, so a started run shows up in it without a read of its own.
      changed={() => undefined}
      settings={() => { window.location.hash = '#/file/' + encodeURIComponent(agent) + '/provisioning'; }} />
      : load.status === 'refused' ? <p className="why-not" role="alert">{said(load.refused)}</p> : null}
  </section>;
}
