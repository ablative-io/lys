import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api } from '../../api';
import { Act } from '../../shell/Act';

const KEY = 'lys.sign-in.file-return';
const fileLink = (hash: string) => /^#\/file\/[^/?#]+\/access\?[^#]*$/.test(hash);

/** Keep only a file route in this tab; authority is never kept in the link. */
export function rememberFileReturn(): void {
  if (fileLink(location.hash)) sessionStorage.setItem(KEY, location.hash);
}

const LANDING = 'lys.sign-in.landing';
/** A page of this surface: the router's own address, never another origin or the sign-in page itself. */
const landingPage = (hash: string) => /^#\/[^#]*$/.test(hash) && !hash.startsWith('#/sign-in');

/**
 * Where a provider sign-in begun in this tab lands. Every sign-in lands on Home, or back on the page that asked
 * for it (sign-in in place). A provider's return is fixed at `/me` by the service (sign_in_upstream.rs
 * SIGNED_IN); the return takes this once, so a later visit to You is never sent elsewhere.
 */
export function rememberLanding(): void {
  sessionStorage.setItem(LANDING, landingPage(location.hash) ? location.hash : '#/');
}

/** A sign-in form that opens drops any landing an earlier, unfinished provider sign-in left behind. */
export function forgetLanding(): void {
  sessionStorage.removeItem(LANDING);
}

function takeLanding(): string | null {
  const hash = sessionStorage.getItem(LANDING);
  sessionStorage.removeItem(LANDING);
  return hash !== null && landingPage(hash) ? hash : null;
}

/** Sign out: the links kept for a sign-in belong to the person leaving. */
export function forgetKept(): void {
  sessionStorage.removeItem(KEY);
  sessionStorage.removeItem(LANDING);
}

const PERSON = 'lys.sign-in.person';
const DROPPED = 'lys.sign-in.dropped';
const withoutQuery = (hash: string) => hash.split('?')[0];

/** Note who is signed in on this browser (the rail and You do, from their own read), so the next sign-in can tell whether the person changed. */
export function rememberSignedIn(person: string): void {
  try { localStorage.setItem(PERSON, person); } catch { /* Unrecorded: the next sign-in then cannot read it either, and keeps nothing (below). */ }
}

/**
 * Before a sign-in opens the page it was going to: a link left open on a shared screen (a give link, a kept
 * return link, any page's query) was opened by whoever was signed in on this browser last. It may fill in an act
 * only for that same person. When someone else was signed in last, or Lys cannot say who signed in now, the
 * kept file link is dropped, the landing and this page lose their query, and the page says why once it opens.
 * A browser no one has signed in on before keeps the link: it was opened by the person signing in.
 * Returns whether what was open is kept.
 */
export async function keepAcrossSignIn(): Promise<boolean> {
  let before: string | null;
  try { before = localStorage.getItem(PERSON); } catch { before = ''; }
  let now: string | null = null;
  try { now = (await api.me()).person.id; } catch { /* Who signed in is unknown: kept only where no one was signed in before. */ }
  if (now !== null) rememberSignedIn(now);
  if (before === null || before === now) return true;
  try {
    sessionStorage.removeItem(KEY);
    const landing = sessionStorage.getItem(LANDING);
    if (landing !== null) sessionStorage.setItem(LANDING, withoutQuery(landing));
    sessionStorage.setItem(DROPPED, 'yes');
  } catch { /* A tab that keeps no storage kept no link either. */ }
  if (landingPage(location.hash) && location.hash.includes('?')) {
    history.replaceState(history.state, '', location.pathname + location.search + withoutQuery(location.hash));
  }
  return false;
}

/** The line a page shows once after a sign-in dropped what someone else left open. */
function takeDropped(): boolean {
  try {
    const dropped = sessionStorage.getItem(DROPPED) !== null;
    sessionStorage.removeItem(DROPPED);
    return dropped;
  } catch {
    return false;
  }
}

export function takeFileReturn(): string | null {
  const hash = sessionStorage.getItem(KEY);
  if (hash === null) return null;
  if (!fileLink(hash)) throw new Error('The saved file link is invalid. Open the original access link again.');
  sessionStorage.removeItem(KEY);
  return hash;
}

/**
 * Providers return to the account page; go on to where the sign-in was going, and confirm the session before
 * restoring a file link. Neither is taken for a person other than the one signed in before (keepAcrossSignIn).
 */
export function ProviderFileReturn() {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const [problem, setProblem] = useState('');
  const [dropped, setDropped] = useState(false);
  useEffect(() => {
    if (takeDropped()) setDropped(true);
  }, [pathname]);
  // Go on to where the sign-in was going. An effect, never a layout effect: the router hears an address change
  // only once its own effects have run, and before that the address would move while You stayed on screen.
  // A file link, when one was kept, is restored once the session is confirmed, and wins.
  useEffect(() => {
    if (pathname !== '/me') return;
    let live = true;
    try {
      if (sessionStorage.getItem(LANDING) === null && sessionStorage.getItem(KEY) === null) return;
    } catch (error) {
      setProblem('The browser could not read where this sign-in was going: ' + String(error) + '. Lys opened You instead; use the menu to go on.');
      return;
    }
    void keepAcrossSignIn().then(() => {
      if (!live) return;
      if (takeDropped()) setDropped(true);
      let kept: boolean;
      try {
        kept = sessionStorage.getItem(KEY) !== null;
        const landing = takeLanding();
        if (landing && landing !== '#/me' && !kept) navigate(landing.slice(1), { replace: true });
      } catch (error) {
        setProblem('The browser could not read where this sign-in was going: ' + String(error) + '. Lys opened You instead; use the menu to go on.');
        return;
      }
      if (!kept) return;
      api.me().then(
        () => {
          if (!live) return;
          try {
            const target = takeFileReturn();
            if (target) navigate(target.slice(1), { replace: true });
          } catch (error) {
            setProblem(String(error));
          }
        },
        (error: unknown) => {
          if (live) setProblem('Lys could not confirm sign-in for the saved file link: ' + String(error));
        }
      );
    });
    return () => {
      live = false;
    };
  }, [pathname, navigate]);
  return <>
    {dropped ? <div className="note sign-in-dropped" role="status">
      <p>Someone else was signed in on this browser before you, so Lys opened this page without filling in anything from the link left open on it. If that link was meant for you, open it again.</p>
      <Act symbol="close" name="Close this note" word="Close" onClick={() => setDropped(false)} />
    </div> : null}
    {problem ? <p role="alert">{problem}</p> : null}
  </>;
}
