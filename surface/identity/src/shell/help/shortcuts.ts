import { GO } from '../keys';
import { PAGES as PALETTE_PAGES } from '../Palette';

/**
 * The keyboard table, built from the shell's own key registry (shell/keys.ts GO) and the palette's names for each
 * screen, so it can never list a key that does nothing or miss one that was added.
 */
export type Shortcut = readonly [keys: string, what: string];

const nameOf = (path: string): string | undefined => PALETTE_PAGES.find(([, hash]) => hash === '#' + path)?.[0];

/** g then a letter: every screen it reaches, by the palette's name for it. Two letters for one screen are joined. */
export function goShortcuts(): Shortcut[] {
  const byPath = new Map<string, string[]>();
  for (const [letter, path] of Object.entries(GO)) byPath.set(path, [...(byPath.get(path) ?? []), letter]);
  return [...byPath].flatMap(([path, letters]) => {
    const name = nameOf(path);
    return name ? [[letters.map((letter) => 'g ' + letter).join(' or '), name] as const] : [];
  });
}

/** The keys that work everywhere, as keys.ts handles them. */
export const SHELL_SHORTCUTS: readonly Shortcut[] = [
  ['? (outside a field) or ⌘/', 'this help; Esc closes'],
  ['⌘K', 'command palette: go anywhere, find anyone'],
  ['j / k, Enter', 'move through a list, open the row'],
  ['1 to 7', 'sections of a person’s or agent’s page'],
  ['c', 'ask the check on this page, where there is one'],
  ['[', 'show or hide the menu labels'],
  ['\\', 'move the menu and Help to the other side'],
  ['Esc', 'close what is open'],
];
