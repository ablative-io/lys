import { createContext, useContext } from 'react';
import type { ReactNode } from 'react';
import { useLocation } from 'react-router';
import { api, request, useLive, useLoad } from '../api';
import type { Load } from '../api';
import { Gate } from '../features/signin/Gate';

async function grantsRead(): Promise<void> {
  await Promise.all([api.me(), api.people(), api.grants(), api.model()]);
}

async function graphRead(): Promise<void> {
  const [people] = await Promise.all([api.people(), api.me(), api.grants(), api.model()]);
  if (people.scope === 'directory') await request('/apps');
}

const READS = {
  canvas: { title: 'Operations', read: () => request('/runtime/live') },
  people: { title: 'People and agents', read: api.people },
  roles: { title: 'Roles', read: () => request('/roles') },
  access: { title: 'Access', read: grantsRead },
  graph: { title: 'Graph', read: graphRead },
  secrets: { title: 'Secrets', read: () => request('/secrets') },
  network: { title: 'Network', read: () => request('/network') },
  settings: { title: 'Configuration', read: () => request('/configuration') },
};
type Place = keyof typeof READS;
type Access = Record<Place, Load<void>>;
const Navigation = createContext<Access | null>(null);

function decision(place: Place): Promise<void> {
  return READS[place].read().then(() => undefined);
}

function placeOf(pathname: string): Place | null {
  const view = pathname.split('/')[1];
  if (view === 'file' || view === 'agents' || view === 'service-accounts') return 'people';
  if (['resources', 'requests', 'reviews', 'model'].includes(view)) return 'access';
  if (view === 'apps' || view === 'connections') return 'settings';
  if (view === 'vault') return 'secrets';
  if (view === 'runtime') return 'canvas';
  return Object.hasOwn(READS, view) ? view as Place : null;
}

/** Only decisions are retained. Reads share the service's change signal and in-flight requests. */
export function NavigationProvider({ children }: { children: ReactNode }) {
  const canvas = useLive(() => decision('canvas'), 'navigation-canvas');
  const people = useLive(() => decision('people'), 'navigation-people');
  const roles = useLive(() => decision('roles'), 'navigation-roles');
  const access = useLive(() => decision('access'), 'navigation-access');
  const graph = useLive(() => decision('graph'), 'navigation-graph');
  const secrets = useLive(() => decision('secrets'), 'navigation-secrets');
  const network = useLive(() => decision('network'), 'navigation-network');
  const settings = useLive(() => decision('settings'), 'navigation-settings');
  return <Navigation.Provider value={{ canvas, people, roles, access, graph, secrets, network, settings }}>{children}</Navigation.Provider>;
}

export function useNavigationAccess(): (pathname: string) => boolean {
  const access = useContext(Navigation);
  if (!access) throw new Error('NavigationAccessMissing: the shell has no API decisions.');
  return (pathname) => {
    const place = placeOf(pathname);
    return place === null || access[place].status === 'ok';
  };
}

function ReadEntry({ place, children }: { place: Place; children: ReactNode }) {
  const load = useLoad(() => decision(place), 'navigation-entry');
  return <Gate load={load} title={READS[place].title} ok={() => children} />;
}

/** Each direct entry is read afresh before its controls mount, including a previously admitted entry. */
export function NavigationGate({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const place = placeOf(pathname);
  return place ? <ReadEntry key={pathname} place={place}>{children}</ReadEntry> : <>{children}</>;
}
