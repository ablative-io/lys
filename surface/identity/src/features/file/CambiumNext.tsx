/**
 * The step after an agent is added: using it in Cambium. Cambium's own Invite panel lists the person's Lys agents
 * (it reads `/apps/me/agents`), so Lys's part is to say where that is and what is still to do here. Which step is
 * missing is read from the apps list: before Cambium is approved, the one step is connecting it on Apps.
 *
 * Cambium's address is its sign-in return address, which only the administrator's view of the app carries; a person
 * is served the app's name and standing alone, so for them Cambium is named without a link.
 */
import { api, request, useLoad } from '../../api';
import { ErrorWords } from '../people/Words';

const CAMBIUM = 'cambium';

interface CambiumRead { state: string; address: string | null }

const strings = (value: unknown): string[] => Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];

/** The web address an app's first sign-in return address belongs to, or null when none is served or it is not a web address. */
function addressOf(app: Record<string, unknown>): string | null {
  const signIn = app.sign_in && typeof app.sign_in === 'object' ? app.sign_in as Record<string, unknown> : null;
  const first = [...strings(signIn?.redirects), ...strings(app.redirects)][0];
  if (!first) return null;
  try {
    const url = new URL(first);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.origin : null;
  } catch { return null; }
}

/** Cambium as the apps list answers it: its standing and, when served, its address. Absent means not connected. */
export async function readCambium(): Promise<CambiumRead | null> {
  const { apps } = await request<{ apps: unknown }>('/apps');
  const app = Array.isArray(apps) ? apps.find((entry): entry is Record<string, unknown> => Boolean(entry) && typeof entry === 'object' && (entry as Record<string, unknown>).id === CAMBIUM) : undefined;
  return app ? { state: typeof app.state === 'string' ? app.state : '', address: addressOf(app) } : null;
}

/** Whether `agent` holds standing access to anything in Cambium: then the steps are behind it, and one line is said. */
async function holdsCambium(agent: string): Promise<boolean> {
  const { grants } = await api.grants();
  return grants.some((grant) => grant.holder === agent && grant.standing.stands && grant.resource.kind.startsWith(CAMBIUM + '.'));
}

export function CambiumNext({ agent, name, administrator }: { agent: string; name: string; administrator: boolean }) {
  const load = useLoad(readCambium, 'agent-cambium-next');
  const held = useLoad(() => holdsCambium(agent), 'agent-cambium-held:' + agent);
  const access = '#/file/' + encodeURIComponent(agent) + '/access';
  // Nothing is said until the apps list answers, so the wrong step is never shown first.
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <section className="cambium-next" aria-label={'Use ' + name + ' in Cambium'}><h2>Use {name} in Cambium</h2><ErrorWords problem={load.refused} /></section>;
  const cambium = load.data;
  if (!cambium || cambium.state !== 'approved') {
    return <section className="cambium-next" aria-label={'Use ' + name + ' in Cambium'}><h2>Use {name} in Cambium</h2>
      {administrator
        ? <p>Cambium is not connected to this Lys yet. <a href="#/apps">Connect Cambium on Apps</a>, then come back to this page for the next step.</p>
        : <p>Cambium is not connected to this Lys yet, so {name} cannot be used there. The Lys administrator connects it on Apps; once they have, the steps appear here.</p>}
    </section>;
  }
  if (held.status === 'loading') return null;
  if (held.status === 'ok' && held.data) {
    return <p className="cambium-next done">{name} has access in Cambium. To bring it into another channel, press Invite in that channel{cambium.address ? <> (<a href={cambium.address} target="_blank" rel="noreferrer">open Cambium</a>)</> : null}; <a href={access}>change what it may use</a> on its Access tab.</p>;
  }
  return <section className="cambium-next" aria-label={'Use ' + name + ' in Cambium'}><h2>Use {name} in Cambium</h2>
    <ol>
      <li>{cambium.address ? <a href={cambium.address} target="_blank" rel="noreferrer">Open Cambium</a> : 'Open Cambium'}, go to a channel and press Invite.</li>
      <li>Under “Invite an agent from Lys”, choose {name}. Cambium shows what to do on the computer {name} runs on.</li>
      <li><a href={access}>Give {name} the Cambium channels it may use</a>, on its Access tab here.</li>
    </ol>
  </section>;
}
