import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { pref, setPref } from './prefs';

export type DockMode = 'help' | null;

export type KindFilter = 'all' | 'person' | 'agent' | 'teams' | 'accounts' | 'found';

export interface Shell {
  labels: boolean;
  toggleLabels: () => void;
  dockRight: boolean;
  setDockSide: (side: 'left' | 'right') => void;
  setLabels: (on: boolean) => void;
  toggleDockSide: () => void;
  dockMode: DockMode;
  toggleDock: (mode: Exclude<DockMode, null>) => void;
  /** Open the dock on a panel, leaving it open if it already shows that panel. */
  openDock: (mode: Exclude<DockMode, null>) => void;
  closeDock: () => void;
  helpSel: string | null;
  showHelp: (id: string | null) => void;
  paletteOpen: boolean;
  openPalette: () => void;
  explaining: boolean;
  explainOn: () => void;
  explainOff: () => void;
  closeAll: () => void;
  toastText: string;
  toastShown: boolean;
  toast: (text: string) => void;
  rows: string[];
  cursor: number;
  setRows: (rows: string[]) => void;
  setCursor: (cursor: number) => void;
  filterKind: KindFilter;
  setFilterKind: (kind: KindFilter) => void;
  /** Nobody is signed in: the shell shows no rail, help or palette, none of which can work. */
  signedOut: boolean;
  /** Mark the screen signed out until the returned function is called. */
  markSignedOut: () => () => void;
}

const ShellContext = createContext<Shell | null>(null);

export function useShell(): Shell {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error('useShell outside ShellProvider');
  return shell;
}

/** While mounted, the shell shows nothing that needs a session. Outside the shell (the sign-in callback) it does nothing. */
export function useSignedOutScreen(): void {
  const mark = useContext(ShellContext)?.markSignedOut;
  useEffect(() => mark?.(), [mark]);
}

const screen = () => document.getElementById('screen');

function returnFocus(opener: Element | null) {
  const target = opener instanceof HTMLElement && document.contains(opener) ? opener : screen();
  target?.focus();
}

export function ShellProvider({ children }: { children: ReactNode }) {
  const [labels, setLabelsState] = useState(pref('labels', 'icons') === 'labels');
  const [dockRight, setDockRight] = useState(pref('dock', 'left') === 'right');
  const [dockMode, setDockMode] = useState<DockMode>(() => { const saved = pref('dock-panel', ''); return saved === 'help' ? saved : null; });
  const [helpSel, setHelpSel] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [explaining, setExplaining] = useState(false);
  const [toastText, setToastText] = useState('');
  const [toastShown, setToastShown] = useState(false);
  const [rows, setRows] = useState<string[]>([]);
  const [cursor, setCursor] = useState(0);
  const [filterKind, setFilterKind] = useState<KindFilter>('all');
  const [signedOutScreens, setSignedOutScreens] = useState(0);
  const opener = useRef<Element | null>(null);
  const explainOpener = useRef<Element | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const state = useRef({ paletteOpen, explaining });
  state.current = { paletteOpen, explaining };

  const setLabels = useCallback((on: boolean) => {
    setPref('labels', on ? 'labels' : 'icons');
    setLabelsState(on);
  }, []);
  const toggleLabels = useCallback(() => setLabels(pref('labels', 'icons') !== 'labels'), [setLabels]);
  const setDockSide = useCallback((side: 'left' | 'right') => {
    setPref('dock', side);
    setDockRight(side === 'right');
  }, []);
  const toggleDockSide = useCallback(
    () => setDockSide(pref('dock', 'left') === 'left' ? 'right' : 'left'),
    [setDockSide],
  );
  const toggleDock = useCallback((mode: Exclude<DockMode, null>) => {
    setDockMode((was) => {
      const next = was === mode ? null : mode;
      setPref('dock-panel', next || '');
      return next;
    });
    setHelpSel(null);
  }, []);
  const openDock = useCallback((mode: Exclude<DockMode, null>) => {
    setPref('dock-panel', mode);
    setDockMode(mode);
  }, []);
  const closeDock = useCallback(() => {
    setPref('dock-panel', '');
    setDockMode(null);
  }, []);
  const showHelp = useCallback((id: string | null) => {
    setPref('dock-panel', 'help');
    setDockMode('help');
    setHelpSel(id);
  }, []);
  const explainOff = useCallback(() => {
    if (!state.current.explaining) return;
    setExplaining(false);
    const back = explainOpener.current;
    explainOpener.current = null;
    queueMicrotask(() => returnFocus(back));
  }, []);
  const closeAll = useCallback(() => {
    explainOff();
    if (state.current.paletteOpen) {
      setPaletteOpen(false);
      const back = opener.current;
      opener.current = null;
      queueMicrotask(() => returnFocus(back));
    }
  }, [explainOff]);
  const explainOn = useCallback(() => {
    if (state.current.paletteOpen) {
      setPaletteOpen(false);
    }
    explainOpener.current = opener.current || document.activeElement;
    opener.current = null;
    setExplaining(true);
  }, []);
  const openPalette = useCallback(() => {
    opener.current = document.activeElement;
    setPaletteOpen(true);
  }, []);
  const toast = useCallback((text: string) => {
    setToastText(text);
    setToastShown(true);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToastShown(false), 2600);
  }, []);

  const markSignedOut = useCallback(() => {
    setSignedOutScreens((count) => count + 1);
    return () => setSignedOutScreens((count) => count - 1);
  }, []);
  const signedOut = signedOutScreens > 0;

  const value = useMemo<Shell>(
    () => ({
      labels, toggleLabels, setLabels, dockRight, setDockSide, toggleDockSide,
      dockMode, toggleDock, openDock, closeDock, helpSel, showHelp,
      paletteOpen, openPalette, explaining, explainOn, explainOff, closeAll,
      toastText, toastShown, toast, rows, cursor, setRows, setCursor, filterKind, setFilterKind,
      signedOut, markSignedOut,
    }),
    [labels, toggleLabels, setLabels, dockRight, setDockSide, toggleDockSide, dockMode, toggleDock, openDock,
      closeDock, helpSel, showHelp, paletteOpen, openPalette, explaining, explainOn, explainOff,
      closeAll, toastText, toastShown, toast, rows, cursor, filterKind, signedOut, markSignedOut],
  );
  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>;
}
