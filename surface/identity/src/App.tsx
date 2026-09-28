import { HashRouter } from 'react-router';
import { SetupBoundary } from './features/setup/SetupBoundary';
import { Callback } from './features/signin/Gate';
import { AppRoutes } from './routes';
import { Shell } from './shell/Shell';
import { ShellProvider } from './shell/ShellContext';

export function App() {
  if (location.pathname === '/callback' || location.pathname === '/auth/callback') return <Callback />;
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
