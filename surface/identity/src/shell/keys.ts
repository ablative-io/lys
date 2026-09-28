import { useEffect, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { useShell } from './ShellContext';
import { TABS } from '../features/file/tabs';

/** g then a letter goes to a screen, as the mock-up's key registry has it. */
export const GO: Record<string, string> = {
  p: '/people', o: '/roles', r: '/resources', a: '/access', q: '/requests', w: '/reviews', v: '/secrets',
  n: '/connections', x: '/sessions', m: '/model', s: '/settings', h: '/graph', t: '/network', u: '/me',
};

const typing = (target: EventTarget | null) =>
  target instanceof HTMLElement && /input|select|textarea/i.test(target.tagName);

/** One registry of keys for the whole shell, Command as the primary modifier. */
export function useShellKeys(): void {
  const shell = useShell();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const live = useRef({ shell, navigate, pathname });
  live.current = { shell, navigate, pathname };

  useEffect(() => {
    let pendingG = false;
    let pendingTimer: ReturnType<typeof setTimeout> | undefined;

    // Help first, in the capture phase, so explain mode owns Escape and ?.
    const help = (e: KeyboardEvent) => {
      const { shell: s } = live.current;
      if (s.explaining && e.key === 'Escape') {
        e.stopImmediatePropagation();
        s.explainOff();
        return;
      }
      if (typing(e.target)) return;
      if (e.key === '?') {
        e.preventDefault();
        if (s.explaining) s.explainOff();
        else s.explainOn();
      }
    };

    const keys = (e: KeyboardEvent) => {
      const { shell: s, navigate: go, pathname: path } = live.current;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        if (s.paletteOpen) s.closeAll();
        else s.openPalette();
        return;
      }
      if (e.key === 'Escape') return s.closeAll();
      if (s.paletteOpen || s.explaining) return;
      if (s.drawer !== null && e.target instanceof HTMLElement && e.target.closest('#drawer')) return;
      if (typing(e.target)) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === 'Enter' && e.target instanceof HTMLElement && e.target.closest('button,a')) return;
      if (pendingG) {
        pendingG = false;
        const to = GO[e.key];
        if (to) return go(to);
      }
      if (e.key === 'g') {
        pendingG = true;
        clearTimeout(pendingTimer);
        pendingTimer = setTimeout(() => (pendingG = false), 900);
        return;
      }
      if (e.key === '[') return s.toggleLabels();
      if (e.key === '\\') return s.toggleDockSide();
      if (e.key === 'c') {
        document.querySelector<HTMLElement>('[data-act="check"]')?.click();
        return;
      }
      const [, view, id] = path.split('/');
      if (view === 'file' && id && /^[1-7]$/.test(e.key)) return go(`/file/${id}/${TABS[Number(e.key) - 1][0]}`);
      if (s.rows.length && (e.key === 'j' || e.key === 'k')) {
        s.setCursor(Math.max(0, Math.min(s.rows.length - 1, s.cursor + (e.key === 'j' ? 1 : -1))));
        return;
      }
      if (s.rows.length && e.key === 'Enter') go(s.rows[s.cursor].slice(1));
    };

    document.addEventListener('keydown', help, true);
    document.addEventListener('keydown', keys);
    return () => {
      clearTimeout(pendingTimer);
      document.removeEventListener('keydown', help, true);
      document.removeEventListener('keydown', keys);
    };
  }, []);
}
