/** Browser-only native GPU/VT check; no mocked renderer and no invented running-agent evidence. */
import { createTerminal } from '@gespenst/core';
import '@gespenst/core/style.css';
import '../../src/styles/tokens.css';

async function run() {
  const container = document.getElementById('terminal');
  const result = document.getElementById('result');
  if (!container || !result) throw new Error('Renderer check elements are missing');
  const terminal = await createTerminal({ container, renderer: 'webgpu', worker: 'dedicated', accessibility: 'full',
    theme: { foreground: '#f4f4f5', background: '#0d0d12', cursor: '#D4975A' },
  });
  terminal.on('error', (error) => { result.textContent = 'Renderer failed: ' + error.message; });
  await terminal.writeAsync(new TextEncoder().encode('\x1b[32mLys terminal\x1b[0m\r\nANSI colour and cursor movement\r\nold text\r\x1b[2KUpdated in place\r\n'));
  await terminal.writeAsync(new Uint8Array([0xe2]));
  await terminal.writeAsync(new Uint8Array([0x82, 0xac, 13, 10]));
  result.textContent = 'Renderer: ' + terminal.renderer.backend + '. Ghostty parsed the byte stream.';
  terminal.focus();
  window.addEventListener('pagehide', () => terminal.dispose(), { once: true });
}

void run().catch((error: unknown) => {
  const result = document.getElementById('result');
  if (result) result.textContent = 'Renderer failed: ' + String(error);
});
