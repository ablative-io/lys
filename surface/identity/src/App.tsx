import { HashRouter } from 'react-router';
import { Callback } from './features/signin/Gate';
import { AppRoutes } from './routes';
import { Shell } from './shell/Shell';
import { ShellProvider } from './shell/ShellContext';

export function App() {
  if (location.pathname === '/callback') return <Callback />;
  return (
    <HashRouter>
      <ShellProvider>
        <Shell>
          <AppRoutes />
        </Shell>
      </ShellProvider>
    </HashRouter>
  );
}
