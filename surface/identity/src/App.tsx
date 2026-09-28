import { HashRouter } from 'react-router';
import { SetupBoundary } from './features/setup/SetupBoundary';
import { InstallProgress } from './features/install/InstallProgress';
import { Callback } from './features/signin/Gate';
import { AppRoutes } from './routes';
import { Shell } from './shell/Shell';
import { ShellProvider } from './shell/ShellContext';

export function App() {
  if (location.pathname === '/callback' || location.pathname === '/auth/callback') return <Callback />;
  // Lys.app serves this package on its own loopback port while it installs; the page needs no sign-in.
  if (location.pathname === '/install') return <InstallProgress />;
  return (
    <HashRouter>
      <ShellProvider>
        <Shell>
          <SetupBoundary><AppRoutes /></SetupBoundary>
        </Shell>
      </ShellProvider>
    </HashRouter>
  );
}
