/**
 * The master off switch as `GET /runtime/stop-everything` answers it, what a pull answers, and the sentences the
 * Dashboard says them in: by agent and computer names, never their identifiers.
 */
import { Refused, request } from '../../api';
import { clock } from '../file/time';

/** Who stopped everything, when and why. */
export interface CordPull { operation: string; by: string; by_name: string | null; reason: string; kill: boolean; at: number }
/** A session a pull names, with its agent's and computer's names. */
export interface CordSession { session: string; agent: string | null; agent_name: string | null; machine: string; machine_name: string | null }
/** A computer whose runner could not be asked or did not answer. */
export interface CordUnreached { machine: string; machine_name: string | null; refusal: string; reason: string }
/** What one pull did: the answer of `POST /runtime/stop-everything`. */
export interface CordResult {
  operation: string; pulled: CordPull; stopped: CordSession[]; still_running: CordSession[]; unreached: CordUnreached[];
  handles_ended: string[]; handles_refused: { agent: string; refusal: string }[];
}
/** Who let agents start again, and when. */
export interface CordRelease { operation: string; pull: string; by: string; by_name: string | null; at: number }
/** How the cord stands, and whether the reader may pull it. */
export interface CordView { pulled: CordPull | null; last: CordResult | null; released: CordRelease | null; may_pull: boolean }

export const PULL = '/runtime/stop-everything';
export const RELEASE = '/runtime/stop-everything/release';

/** How the cord stands, or the refusal that kept the page from knowing; the rest of the page reads either way. */
export const readCord = (): Promise<CordView | Refused> => request<CordView>(PULL).then(
  (view) => view,
  (problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: 'CordUnreadable', reason: String(problem) }),
);

const who = (pull: CordPull): string => pull.by_name ?? 'an administrator';

/**
 * The line across the top while the cord is pulled. It says everything is stopped only when the pull's own result
 * left nothing running and reached every computer; with no result kept for this pull yet it says so; otherwise it
 * says what has not stopped, since a page that read
 * "everything is stopped" over a running agent would be believed.
 */
export function stoppedLine(pull: CordPull, last: CordResult | null): string {
  const told = pull.reason + ', by ' + who(pull) + ', ' + clock(pull.at) + '. No agent can be started.';
  if (!last || last.operation !== pull.operation) return 'Stop everything was pulled; what it stopped is not yet known. ' + told;
  const still = last.still_running.length;
  const unreached = last.unreached.length;
  if (!still && !unreached) return 'Everything is stopped: ' + told;
  const left = [
    still ? still + (still === 1 ? ' agent is still running' : ' agents are still running') : '',
    unreached ? unreached + (unreached === 1 ? ' computer could not be reached' : ' computers could not be reached') : '',
  ].filter(Boolean).join(' and ');
  return 'Stop everything was pulled, and not everything has stopped: ' + left + '. ' + told;
}

/** Why a Start control is disabled, for its title; null while agents may start. */
export function stoppedWhy(cord: CordView | Refused): string | null {
  if (cord instanceof Refused || !cord.pulled) return null;
  return 'No agent can be started: ' + who(cord.pulled) + ' stopped everything ' + clock(cord.pulled.at) + ' because: ' + cord.pulled.reason;
}

const where = (name: string | null): string => name ?? 'a computer with no name';
const named = (session: CordSession): string =>
  (session.agent ? session.agent_name ?? 'an agent with no name' : 'a session Lys did not start') + ' on ' + where(session.machine_name);

/** `a`, `a and b`, `a, b and c`. */
function listed(items: string[]): string {
  return items.length < 2 ? items.join('') : items.slice(0, -1).join(', ') + ' and ' + items[items.length - 1];
}

/** What a pull did, in plain sentences: how many stopped, which still run, which computers could not be reached and why. */
export function pullSentences(result: CordResult): string[] {
  const { stopped, still_running: still, unreached } = result;
  if (!stopped.length && !still.length && !unreached.length) return ['Nothing was running, so nothing needed stopping.'];
  const said = [stopped.length ? 'Stopped ' + stopped.length + ': ' + listed(stopped.map(named)) + '.' : 'Nothing was confirmed stopped.'];
  said.push(still.length ? 'Still running: ' + listed(still.map(named)) + '.' : unreached.length ? '' : 'Nothing Lys started is still running.');
  for (const computer of unreached) said.push(where(computer.machine_name) + ' could not be reached: ' + computer.reason + '.');
  const agents = new Map([...stopped, ...still].flatMap((session): [string, string | null][] => session.agent ? [[session.agent, session.agent_name]] : []));
  for (const refused of result.handles_refused) {
    said.push('The credential service did not confirm that the credentials of ' + (agents.get(refused.agent) ?? 'an agent with no name') + ' ended.');
  }
  return said.filter(Boolean);
}
