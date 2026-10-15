import { useEffect } from 'react';
import { useShell } from './ShellContext';
import { HelpDock } from './help/HelpDock';

export function Dock() {
  const shell = useShell();
  useEffect(() => {
    if (shell.dockMode !== 'help') return;
    const leave = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented || shell.explaining || shell.paletteOpen) return;
      shell.closeDock();
      document.querySelector<HTMLButtonElement>('#rail [data-dockbtn="help"]')?.focus();
    };
    window.addEventListener('keydown', leave);
    return () => window.removeEventListener('keydown', leave);
  }, [shell.dockMode, shell.explaining, shell.paletteOpen, shell.closeDock]);
  return (
    <aside className={'dock' + (shell.dockMode === 'help' ? ' open' : '')} id="dock" aria-label="Help">
      {shell.dockMode === 'help' ? <HelpDock /> : null}
    </aside>
  );
}
