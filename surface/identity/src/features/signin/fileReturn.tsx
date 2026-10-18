import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api } from '../../api';

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

export function takeFileReturn(): string | null {
  const hash = sessionStorage.getItem(KEY);
  if (hash === null) return null;
  if (!fileLink(hash)) throw new Error('The saved file link is invalid. Open the original access link again.');
  sessionStorage.removeItem(KEY);
  return hash;
}

/** Providers return to the account page; go on to where the sign-in was going, and confirm the session before restoring a file link. */
export function ProviderFileReturn() {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const [problem, setProblem] = useState('');
  // Go on to where the sign-in was going. An effect, never a layout effect: the router hears an address change
  // only once its own effects have run, and before that the address would move while You stayed on screen.
  // A file link, when one was kept, is restored below once the session is confirmed, and wins.
  useEffect(() => {
    if (pathname !== '/me') return;
    try {
      const landing = takeLanding();
      if (landing && landing !== '#/me' && sessionStorage.getItem(KEY) === null) navigate(landing.slice(1), { replace: true });
    } catch (error) {
      setProblem('The browser could not read where this sign-in was going: ' + String(error) + '. Lys opened You instead; use the menu to go on.');
    }
  }, [pathname, navigate]);
  useEffect(() => {
    if (pathname !== '/me') return;
    let live = true;
    try {
      if (sessionStorage.getItem(KEY) === null) return;
    } catch (error) {
      setProblem('The browser could not read the file link saved for sign-in: ' + String(error));
      return;
    }
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
    return () => {
      live = false;
    };
  }, [pathname, navigate]);
  return problem ? <p role="alert">{problem}</p> : null;
}
