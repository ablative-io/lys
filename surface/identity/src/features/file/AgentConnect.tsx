/**
 * Connect, under an agent's Credentials (AGENTS-016 R2): the two complete lines that connect Claude Code and Codex to
 * this agent through `lys mcp`, built from this page's own origin and the agent's id, each with the same copy act the
 * Connect group of a computer uses. Nothing is issued, posted or stored here: the section renders strings. A person's
 * file has no Connect section.
 */
import { kindOf } from '../../generated';
import { Copy } from '../network/JoinCode';

/** The key file, shown as written: the person puts in the path on the computer the line is run on. */
const KEY_FILE = '<path to this agent’s key file on that computer>';

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
  const bridge = `lys mcp --url ${origin}/api/mcp --agent ${id} --key-file ${KEY_FILE}`;
  const lines: [string, string][] = [
    ['The Claude Code line', `claude mcp add --transport stdio lys -- ${bridge}`],
    ['The Codex line', `codex mcp add lys -- ${bridge}`],
  ];
  return <section className="card" aria-label="Connect" data-connect-ready>
    <h2>Connect</h2>
    <p>Run one of these on the computer where Claude Code or Codex runs, to reach this agent through Lys.</p>
    {lines.map(([what, line]) => <p key={what}><code data-connect-line>{line}</code> <Copy text={line} what={what} /></p>)}
    <p className="sec">The key file is the PEM or seat file holding the key certified for this agent, listed under Capability certificates above.</p>
    <p data-connect-works>{loopback(origin)
      ? 'With this address the line works now: lys mcp reaches Lys over http on this computer.'
      : 'With this address the line needs a lys that carries the https bridge of AGENTS-015 R3; with an older lys it is refused.'}</p>
  </section>;
}
