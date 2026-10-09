import { HashRouter, Route, Routes, useNavigate, useParams, useSearchParams } from 'react-router';
import { FirstRunSetup, setupCode } from './features/setup/Setup';
import { SetupBoundary } from './features/setup/SetupBoundary';
import { SignIn } from './features/sign-in/SignIn';
import { Callback } from './features/signin/Gate';
import { Terminal } from './features/runtime/Terminal';
import './features/runtime/terminal.css';
import './features/runtime/session-canvas.css';
import { AppRoutes } from './routes';
import { Shell } from './shell/Shell';
import { ShellProvider } from './shell/ShellContext';
import { NavigationGate, NavigationProvider } from './shell/NavigationAccess';

/** Lys's own sign-in page, outside the signed-in shell. */
function SignInScreen() {
  const navigate = useNavigate();
  return <SignIn signedIn={() => navigate('/')} />;
}

/** One session's terminal alone in its own browser window, outside the shell: the screen takes the whole window. */
function TerminalWindow() {
  const { session = '' } = useParams();
  const [query] = useSearchParams();
  return <div className="terminal-window"><Terminal session={session} agent={query.get('agent')} /></div>;
}

export function App() {
  if (location.pathname === '/callback' || location.pathname === '/auth/callback') return <Callback />;
  if (location.pathname === '/setup') return <FirstRunSetup code={setupCode()} />;
  return (
    <HashRouter>
      <Routes>
        <Route path="/sign-in" element={<SignInScreen />} />
        <Route path="/window/:session" element={<TerminalWindow />} />
        <Route path="*" element={
          <ShellProvider>
            <NavigationProvider>
              <Shell>
                <SetupBoundary><NavigationGate><AppRoutes /></NavigationGate></SetupBoundary>
              </Shell>
            </NavigationProvider>
          </ShellProvider>
        } />
      </Routes>
    </HashRouter>
  );
}
