import type { ReactNode } from 'react';
import { Dock } from './Dock';
import { Explain } from './Explain';
import { Palette } from './Palette';
import { Rail } from './Rail';
import { Upgraded } from './Upgraded';
import { useShell } from './ShellContext';
import { useShellKeys } from './keys';

export function Shell({ children }: { children: ReactNode }) {
  const shell = useShell();
  useShellKeys();
  return (
    <>
      <div className={'shell' + (shell.dockRight ? ' dock-right' : '')} id="shell">
        {shell.signedOut ? null : <Rail />}
        <main className="screen" id="screen" tabIndex={-1}>
          {children}
        </main>
        {shell.signedOut ? null : <Dock />}
      </div>
      <div className={'scrim' + (shell.paletteOpen && !shell.signedOut ? ' open' : '')} id="scrim" onClick={shell.closeAll} />
      {shell.signedOut ? null : <Palette />}
      <Upgraded />
      <div className={'toast' + (shell.toastShown ? ' show' : '')} id="toast" role="status">
        {shell.toastText}
      </div>
      {shell.explaining ? <Explain /> : null}
    </>
  );
}
