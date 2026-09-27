import type { ReactNode } from 'react';
import { Dock } from './Dock';
import { Explain } from './Explain';
import { Palette } from './Palette';
import { Rail } from './Rail';
import { useShell } from './ShellContext';
import { useShellKeys } from './keys';

export function Shell({ children }: { children: ReactNode }) {
  const shell = useShell();
  useShellKeys();
  return (
    <>
      <div className={'shell' + (shell.dockRight ? ' dock-right' : '')} id="shell">
        <Rail />
        <main className="screen" id="screen" tabIndex={-1}>
          {children}
        </main>
        <Dock />
      </div>
      <aside className="drawer" id="drawer" aria-label="Detail" />
      <div className={'scrim' + (shell.paletteOpen ? ' open' : '')} id="scrim" onClick={shell.closeAll} />
      <Palette />
      <div className={'toast' + (shell.toastShown ? ' show' : '')} id="toast" role="status">
        {shell.toastText}
      </div>
      {shell.explaining ? <Explain /> : null}
    </>
  );
}
