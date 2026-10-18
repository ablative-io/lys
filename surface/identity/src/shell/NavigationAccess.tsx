import { createContext, useCallback, useContext, useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { useLocation } from 'react-router';
import { Refused, api, request, useLive, useLoad } from '../api';
import type { Load } from '../api';
import { Gate } from '../features/signin/Gate';
import { ErrorWords } from '../features/people/Words';

async function grantsRead(): Promise<void> {
  await Promise.all([api.me(), api.people(), api.grants(), api.model()]);
}

async function graphRead(): Promise<void> {
  const [people] = await Promise.all([api.people(), api.me(), api.grants(), api.model()]);
  if (people.scope === 'directory') await request('/apps');
}

const READS = {
  canvas: { title: 'Running agents', read: () => request('/runtime/live') },
  people: { title: 'People and agents', read: api.people },
  roles: { title: 'Roles', read: () => request('/roles') },
  access: { title: 'Access', read: grantsRead },
  graph: { title: 'Access map', read: graphRead },
  secrets: { title: 'Secrets', read: () => request('/secrets') },
  network: { title: 'Computers', read: () => request('/network') },
  settings: { title: 'Settings', read: () => request('/configuration') },
  // Each way to ask or decide is its own read, so a person who may not read grants still reaches what waits on them (domain 03's ask).
  requests: { title: 'Requests', read: () => request('/requests') },
  reviews: { title: 'Reviews', read: () => request('/reviews') },
  drafts: { title: 'Drafts', read: () => request('/drafts?state=waiting') },
};
type Place = keyof typeof READS;
type Access = Record<Place, Load<void>>;
const Navigation = createContext<Access | null>(null);

/** The places only a person who may act can open: a registered, suspended or retired person is not asked about them. */
const ACTING: readonly Place[] = ['people', 'access', 'graph'];

function decision(place: Place): Promise<void> {
  return READS[place].read().then(() => undefined);
}

/** The rail's decision for `place`, asked of the service only for a person who may act there. */
async function railDecision(place: Place): Promise<void> {
  if (ACTING.includes(place) && (await api.me()).person.state !== 'active') {
    throw new Refused(403, { refusal: 'NotActive', reason: 'your account is not active yet, so this is not open to you until an administrator activates it' });
  }
  return decision(place);
}

function placeOf(pathname: string): Place | null {
  if (pathname === '/settings/account') return null;
  if (pathname === '/access/drafts') return 'drafts';
  const view = pathname.split('/')[1];
  if (view === 'file' || view === 'agents' || view === 'service-accounts') return 'people';
  if (['resources', 'model'].includes(view)) return 'access';
  if (view === 'requests' || view === 'reviews') return view;
  if (view === 'apps' || view === 'connections') return 'settings';
  if (view === 'vault') return 'secrets';
  if (view === 'runtime') return 'canvas';
  return Object.hasOwn(READS, view) ? view as Place : null;
}

const Entry = createContext<(() => void) | null>(null);

/** One decision, asked once the entry page has made its own reads, so they share theirs rather than ask twice. */
function useDecision(place: Place, entered: boolean): Load<void> {
  return useLive(() => entered ? railDecision(place) : new Promise<void>(() => undefined), 'navigation-' + place + (entered ? '' : '-waiting'));
}

/** Only decisions are retained. Reads share the service's change signal and in-flight requests. */
export function NavigationProvider({ children }: { children: ReactNode }) {
  const [entered, setEntered] = useState(false);
  const enter = useCallback(() => setEntered(true), []);
  const canvas = useDecision('canvas', entered);
  const people = useDecision('people', entered);
  const roles = useDecision('roles', entered);
  const access = useDecision('access', entered);
  const graph = useDecision('graph', entered);
  const secrets = useDecision('secrets', entered);
  const network = useDecision('network', entered);
  const settings = useDecision('settings', entered);
  const requests = useDecision('requests', entered);
  const reviews = useDecision('reviews', entered);
  const drafts = useDecision('drafts', entered);
  return (
    <Entry.Provider value={enter}>
      <Navigation.Provider value={{ canvas, people, roles, access, graph, secrets, network, settings, requests, reviews, drafts }}>{children}</Navigation.Provider>
    </Entry.Provider>
  );
}

/** Marks the entry page mounted, so the rail's decisions may be asked. */
function Entered({ children }: { children: ReactNode }) {
  const enter = useContext(Entry);
  useEffect(() => { enter?.(); }, [enter]);
  return <>{children}</>;
}

/**
 * What the rail says of a place: open (or still being checked, which is drawn the same, so the rail never moves),
 * not open to this person (the service said so: 403, or a person who is not active), or not checked because the
 * read failed for another reason. Both of the last two carry the words a person reads.
 */
export type Verdict = { open: true } | { open: false; definite: boolean; words: string };

/** The service's reason without its leading refusal name ("NotAdmitted: …"), which a person does not need. */
const reasonOf = (refused: Refused): string => refused.refusal.reason.replace(/^[A-Z][A-Za-z]+: /, '');

function verdictOf(load: Load<void>): Verdict {
  if (load.status !== 'refused') return { open: true };
  const definite = load.refused.status === 401 || load.refused.status === 403;
  return definite
    ? { open: false, definite, words: 'not open to you: ' + reasonOf(load.refused) + '. Open it to see what to do.' }
    : { open: false, definite, words: 'could not be checked: ' + reasonOf(load.refused) + '. Open it to try again.' };
}

/** The rail's verdict for each address. Every entry is drawn whatever the verdict; only the marking differs. */
export function useNavigationVerdict(): (pathname: string) => Verdict {
  const access = useContext(Navigation);
  if (!access) throw new Error('NavigationAccessMissing: the shell has no API decisions.');
  return (pathname) => {
    const place = placeOf(pathname);
    return place === null ? { open: true } : verdictOf(access[place]);
  };
}

/** Whether Search and the g keys offer an address: everything except what the service said is not open to this person. */
export function useNavigationAccess(): (pathname: string) => boolean {
  const verdict = useNavigationVerdict();
  return (pathname) => {
    const said = verdict(pathname);
    return said.open || !said.definite;
  };
}

function ReadEntry({ place, children }: { place: Place; children: ReactNode }) {
  const load = useLoad(() => decision(place), 'navigation-entry');
  // People and agents are refused in the directory's own words, as its pages refuse them.
  const words = place === 'people' ? (problem: Refused) => <ErrorWords problem={problem} /> : undefined;
  const gate = <Gate load={load} title={READS[place].title} ok={() => children} renderError={words} />;
  return load.status === 'loading' ? gate : <Entered>{gate}</Entered>;
}

/** Each direct entry is read afresh before its controls mount, including a previously admitted entry. */
export function NavigationGate({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const place = placeOf(pathname);
  return place ? <ReadEntry key={pathname} place={place}>{children}</ReadEntry> : <Entered>{children}</Entered>;
}
