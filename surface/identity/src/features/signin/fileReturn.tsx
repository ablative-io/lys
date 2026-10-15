import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api } from '../../api';

const KEY = 'lys.sign-in.file-return';
const fileLink = (hash: string) => /^#\/file\/[^/?#]+\/access\?[^#]*$/.test(hash);

/** Keep only a file route in this tab; authority is never kept in the link. */
export function rememberFileReturn(): void {
  if (fileLink(location.hash)) sessionStorage.setItem(KEY, location.hash);
}

export function takeFileReturn(): string | null {
  const hash = sessionStorage.getItem(KEY);
  if (hash === null) return null;
  if (!fileLink(hash)) throw new Error('The saved file link is invalid. Open the original access link again.');
  sessionStorage.removeItem(KEY);
  return hash;
}

/** Providers return to the account page; confirm the session before restoring its file link. */
export function ProviderFileReturn() {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const [problem, setProblem] = useState('');
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
