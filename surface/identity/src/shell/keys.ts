import { useEffect, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { useShell } from './ShellContext';
import { tabsFor } from '../features/file/tabs';
import { useNavigationAccess } from './NavigationAccess';

/** g then a letter goes to a screen, as the mock-up's key registry has it. */
export const GO: Record<string, string> = {
  p: '/people', o: '/roles', r: '/resources', a: '/access', q: '/?section=waiting-on-you', w: '/reviews', v: '/secrets',
  n: '/apps', m: '/model', s: '/settings/account', h: '/graph', t: '/network', u: '/me', c: '/canvas', d: '/dashboard',
};

const typing = (target: EventTarget | null) =>
  target instanceof HTMLElement && /input|select|textarea/i.test(target.tagName);

/** The rows the mock-up's KEYABLE rule names as rows (index.v5.html:1559). */
const ROW = 'tr[data-href], tr[data-act]';

/**
 * The screen's row elements in cursor order: the rows whose address is in the
 * cursor's list. The cursor moves focus to them where they stand, so the screen
 * keeps its elements (conformance 9.3).
 */
const rowElements = (rows: string[]): HTMLElement[] =>
  [...document.querySelectorAll<HTMLElement>('#screen tr[data-href]')].filter((row) => rows.includes(row.dataset.href ?? ''));

/** One registry of keys for the whole shell, Command as the primary modifier. */
export function useShellKeys(): void {
  const shell = useShell();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const mayOpen = useNavigationAccess();
  const live = useRef({ shell, navigate, pathname, mayOpen });
  live.current = { shell, navigate, pathname, mayOpen };

  useEffect(() => {
    // g waits for its letter until the next key, a pointer press or the window losing focus: an event, never a clock.
    let pendingG = false;
    const forgetG = () => { pendingG = false; };

    // Help first, in the capture phase, so the page help owns Escape, ? and ⌘/ while it shows.
    const help = (e: KeyboardEvent) => {
      const { shell: s } = live.current;
      if (s.explaining && e.key === 'Escape') {
        e.stopImmediatePropagation();
        s.explainOff();
        return;
      }
      if (s.signedOut) return;
      // ⌘/ opens help from anywhere, a field included; ? is a character in a field, so it opens help only outside one.
      const commandSlash = (e.metaKey || e.ctrlKey) && e.key === '/';
      if (commandSlash || (e.key === '?' && !typing(e.target))) {
        e.preventDefault();
        if (s.explaining) s.explainOff();
        else s.explainOn();
      }
    };

    const keys = (e: KeyboardEvent) => {
      const { shell: s, navigate: go, pathname: path, mayOpen: permitted } = live.current;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        if (s.paletteOpen) s.closeAll();
        else s.openPalette();
        return;
      }
      if (e.key === 'Escape') return s.closeAll();
      if (s.paletteOpen || s.explaining) return;
      if (e.target instanceof HTMLElement && e.target.closest('.act-panel')) return;
      if (typing(e.target)) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === 'Enter' && e.target instanceof HTMLElement && e.target.closest('button,a')) return;
      if (pendingG) {
        pendingG = false;
        const to = GO[e.key];
        if (to && permitted(to)) return go(to);
      }
      if (e.key === 'g') {
        pendingG = true;
        return;
      }
      if (e.key === '[') return s.toggleLabels();
      if (e.key === '\\') return s.toggleDockSide();
      if (e.key === 'c') {
        document.querySelector<HTMLElement>('[data-act="check"]')?.click();
        return;
      }
      const [, view, id] = path.split('/');
      if (view === 'file' && id && /^[1-7]$/.test(e.key)) {
        const to = tabsFor(id)[Number(e.key) - 1];
        if (to) go(`/file/${id}/${to[0]}`);
        return;
      }
      if (s.rows.length && (e.key === 'j' || e.key === 'k')) {
        const next = Math.max(0, Math.min(s.rows.length - 1, s.cursor + (e.key === 'j' ? 1 : -1)));
        s.setCursor(next);
        rowElements(s.rows)[next]?.focus();
        return;
      }
      // A focused row opens itself; the cursor row opens only when no row has focus.
      if (e.key === 'Enter' && e.target instanceof HTMLElement && e.target.closest(ROW)) return;
      if (s.rows.length && e.key === 'Enter') go(s.rows[s.cursor].slice(1));
    };

    document.addEventListener('keydown', help, true);
    document.addEventListener('keydown', keys);
    document.addEventListener('pointerdown', forgetG);
    window.addEventListener('blur', forgetG);
    return () => {
      document.removeEventListener('keydown', help, true);
      document.removeEventListener('keydown', keys);
      document.removeEventListener('pointerdown', forgetG);
      window.removeEventListener('blur', forgetG);
    };
  }, []);
}
