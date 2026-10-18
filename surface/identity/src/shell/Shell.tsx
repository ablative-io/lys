import type { ReactNode } from 'react';
import { Act } from './Act';
import { Dock } from './Dock';
import { Palette } from './Palette';
import { Rail } from './Rail';
import { Upgraded } from './Upgraded';
import { useShell } from './ShellContext';
import { useShellKeys } from './keys';
import { PageHelp } from './help/PageHelp';

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
        {shell.toastShown ? <Act symbol="close" name="Dismiss this message" className="small quiet" onClick={shell.dismissToast} /> : null}
      </div>
      {shell.explaining && !shell.signedOut ? <PageHelp /> : null}
    </>
  );
}
