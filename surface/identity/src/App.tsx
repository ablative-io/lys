import { HashRouter, Route, Routes, useNavigate } from 'react-router';
import { FirstRunSetup, setupCode } from './features/setup/Setup';
import { SetupBoundary } from './features/setup/SetupBoundary';
import { SignIn } from './features/sign-in/SignIn';
import { Callback } from './features/signin/Gate';
import { AppRoutes } from './routes';
import { Shell } from './shell/Shell';
import { ShellProvider } from './shell/ShellContext';

/** Lys's own sign-in page, outside the signed-in shell. */
function SignInScreen() {
  const navigate = useNavigate();
  return <SignIn signedIn={() => navigate('/')} />;
}

export function App() {
  if (location.pathname === '/callback' || location.pathname === '/auth/callback') return <Callback />;
  if (location.pathname === '/setup') return <FirstRunSetup code={setupCode()} />;
  return (
    <HashRouter>
      <Routes>
        <Route path="/sign-in" element={<SignInScreen />} />
        <Route path="*" element={
          <ShellProvider>
            <Shell>
              <SetupBoundary><AppRoutes /></SetupBoundary>
            </Shell>
          </ShellProvider>
        } />
      </Routes>
    </HashRouter>
  );
}
