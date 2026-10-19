/**
 * Connect, under an agent's Credentials (AGENTS-016 R2): the lines network/connectLines.ts builds for this agent from
 * this page's own origin and the agent's id, each with the same copy act the Connect group of a computer uses. The
 * command text and the tools' names live in that module alone. Nothing is issued, posted or stored here: the section
 * renders strings. A person's file has no Connect section.
 */
import { kindOf } from '../../generated';
import { Copy } from '../network/JoinCode';
import { CONNECT_INTRO, KEY_FILE, connectLines } from '../network/connectLines';

/** Whether `origin` is plain http on this computer, which `lys mcp` reaches without the https bridge. */
const loopback = (origin: string): boolean => {
  try {
    const url = new URL(origin);
    return url.protocol === 'http:' && ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname);
  } catch {
    return false;
  }
};

export function AgentConnect({ id, origin = window.location.origin }: { id: string; origin?: string }) {
  if (kindOf(id) !== 'agent') return <span data-connect-ready hidden />;
  const lines = connectLines({ id, origin, keyFile: KEY_FILE });
  return <section className="card" aria-label="Connect" data-connect-ready>
    <h2>Connect</h2>
    <p>{CONNECT_INTRO}</p>
    {lines.map(({ what, line }) => <p key={what}><code data-connect-line>{line}</code> <Copy text={line} what={what} /></p>)}
    <p className="sec">The key file is the PEM or seat file holding the key certified for this agent, listed under Capability certificates above.</p>
    <p data-connect-works>{loopback(origin)
      ? 'With this address the line works now: lys mcp reaches Lys over http on this computer.'
      : 'With this address the line needs a lys that carries the https bridge of AGENTS-015 R3; with an older lys it is refused.'}</p>
  </section>;
}
