import { HashRouter, Route, Routes, useNavigate, useParams, useSearchParams } from 'react-router';
import { FirstRunSetup, setupCode } from './features/setup/Setup';
import { SetupBoundary } from './features/setup/SetupBoundary';
import { SignIn } from './features/sign-in/SignIn';
import { Found } from './features/found/Found';
import { Prove } from './features/found/Prove';
import { Callback } from './features/signin/Gate';
import { ProviderFileReturn, takeFileReturn } from './features/signin/fileReturn';
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
  return <SignIn signedIn={() => navigate(takeFileReturn()?.slice(1) ?? '/')} />;
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
      <ProviderFileReturn />
      <Routes>
        <Route path="/sign-in" element={<SignInScreen />} />
        <Route path="/found" element={<Found />} />
        <Route path="/found/:proof" element={<Prove />} />
        <Route path="/added/:proof" element={<Prove admission />} />
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
