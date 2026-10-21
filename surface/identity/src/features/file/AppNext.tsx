/**
 * The step after an agent is added: using it in the apps connected to this Lys. Each app is read from the apps list,
 * named by its own registration, so Lys says where the app is and what is still to do here, and never what the app's
 * own screens call things. Before any app is approved, the one step is connecting one on Apps.
 *
 * An app's address is its sign-in return address, which only the administrator's view of the app carries; a person is
 * served the app's name and standing alone, so for them the app is named without a link.
 */
import { api, request, useLoad } from '../../api';
import { ErrorWords } from '../people/Words';

interface AppRead { id: string; name: string; address: string | null }

const strings = (value: unknown): string[] => Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];

/**
 * The app's web address: `address`, which every person's view of an approved app carries, else the origin of its first
 * sign-in return address as the administrator's view serves it; null when none is served or it is not a web address.
 */
function addressOf(app: Record<string, unknown>): string | null {
  const signIn = app.sign_in && typeof app.sign_in === 'object' ? app.sign_in as Record<string, unknown> : null;
  const first = [...strings([app.address]), ...strings(signIn?.redirects), ...strings(app.redirects)][0];
  if (!first) return null;
  try {
    const url = new URL(first);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.origin : null;
  } catch { return null; }
}

/** Every approved app as the apps list answers it, in its order: its id, its registered name (else its id) and its address. */
export async function readApproved(): Promise<AppRead[]> {
  const { apps } = await request<{ apps: unknown }>('/apps');
  const entries = Array.isArray(apps) ? apps.filter((entry): entry is Record<string, unknown> => Boolean(entry) && typeof entry === 'object') : [];
  return entries.filter((app) => typeof app.id === 'string' && app.state === 'approved').map((app) => {
    const id = app.id as string;
    return { id, name: typeof app.name === 'string' && app.name.trim() ? app.name : id, address: addressOf(app) };
  });
}

/** The apps under whose kinds `agent` holds standing access, by id: there the steps are behind it, and one line is said. */
async function heldApps(agent: string): Promise<Set<string>> {
  const { grants } = await api.grants();
  return new Set(grants.filter((grant) => grant.holder === agent && grant.standing.stands && grant.resource.kind.includes('.'))
    .map((grant) => grant.resource.kind.slice(0, grant.resource.kind.indexOf('.'))));
}

const opened = (app: AppRead) => app.address ? <a href={app.address} target="_blank" rel="noreferrer">Open {app.name}</a> : 'Open ' + app.name;

export function AppNext({ agent, name, administrator }: { agent: string; name: string; administrator: boolean }) {
  const load = useLoad(readApproved, 'agent-app-next');
  const held = useLoad(() => heldApps(agent), 'agent-app-held:' + agent);
  const access = '#/file/' + encodeURIComponent(agent) + '/access';
  const none = 'Use ' + name + ' in an app';
  // Nothing is said until the apps list answers, so the wrong step is never shown first.
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <section className="app-next" aria-label={none}><h2>{none}</h2><ErrorWords problem={load.refused} /></section>;
  if (!load.data.length) {
    return <section className="app-next" aria-label={none}><h2>{none}</h2>
      {administrator
        ? <p>No app is connected to this Lys yet. <a href="#/apps">Connect one on Apps</a>, then come back to this page for the next step.</p>
        : <p>No app is connected to this Lys yet, so {name} cannot be used in one. The Lys administrator connects apps on Apps; once they have, the steps appear here.</p>}
    </section>;
  }
  if (held.status === 'loading') return null;
  const holds = held.status === 'ok' ? held.data : new Set<string>();
  return <>{load.data.map((app) => {
    const title = 'Use ' + name + ' in ' + app.name;
    if (holds.has(app.id)) {
      return <p key={app.id} className="app-next done">{name} has access in {app.name}.{app.address ? <> {opened(app)};</> : null} <a href={access}>change what it may use</a> on its Access tab.</p>;
    }
    return <section key={app.id} className="app-next" aria-label={title}><h2>{title}</h2>
      <ol>
        <li>{opened(app)}.</li>
        <li><a href={access}>Give {name} what it may use in {app.name}</a>, on its Access tab here.</li>
      </ol>
    </section>;
  })}</>;
}
