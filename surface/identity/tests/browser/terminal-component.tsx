/** Mount the shipped component with native GPU rendering; the harness supplies test HTTP answers. */
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GpuTerminal } from '../../src/features/runtime/GpuTerminal';
import '../../src/styles/tokens.css';
import '../../src/features/runtime/terminal.css';

function Probe() {
  const [session, setSession] = useState('fixture-one');
  const [message, setMessage] = useState('');
  return <main style={{ padding: 24 }}>
    <h1>Native terminal component check</h1>
    <p>Test transport; this page is not a live agent session.</p>
    <button onClick={() => setSession('fixture-two')}>Switch test session</button>
    <p role="status">{session}</p>
    <GpuTerminal key={session} session={session} onEnd={(end) => setMessage(end.how)} onFailure={(error) => setMessage(String(error))} />
    <p role="alert">{message}</p>
  </main>;
}

const root = document.getElementById('root');
if (!root) throw new Error('Component check root is missing');
createRoot(root).render(<Probe />);
