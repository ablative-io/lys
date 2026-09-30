/** Ghostty interprets raw PTY bytes, drawn on WebGPU when the browser offers it; any other renderer the library falls to is named on the page, never silent. */
import { useEffect, useRef, useState } from 'react';
import type { BrowserTerminal } from '@gespenst/core';
import '@gespenst/core/style.css';
import { terminalRequest, terminalStreams } from './terminal-transport';
import type { SessionEnd } from './Terminal';

type Backend = BrowserTerminal['renderer']['backend'];
const RENDERERS: Record<Backend, string> = { webgpu: 'WebGPU', webgl2: 'WebGL2', canvas2d: 'Canvas 2D' };

export function GpuTerminal({ session, onEnd, onFailure }: {
  session: string; onEnd: (end: SessionEnd) => void; onFailure: (error: unknown) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onEnd, onFailure });
  callbacks.current = { onEnd, onFailure };
  const [status, setStatus] = useState<'starting' | 'ready' | 'failed'>('starting');
  const [backend, setBackend] = useState<Backend>('webgpu');
  useEffect(() => {
    const container = host.current;
    if (!container) return;
    const controller = new AbortController();
    let terminal: BrowserTerminal | undefined;
    let resizeWork = Promise.resolve();
    const failed = (error: unknown) => {
      if (controller.signal.aborted) return;
      callbacks.current.onFailure(error);
      controller.abort();
      terminal?.blur();
      setStatus('failed');
    };
    const start = async () => {
      const { createTerminal } = await import('@gespenst/core');
      if (controller.signal.aborted) return;
      const style = getComputedStyle(container);
      const opened = await createTerminal({
        container, renderer: 'auto', worker: 'dedicated', accessibility: 'full',
        ariaLabel: 'Live agent terminal', fontFamily: '"JetBrains Mono", ui-monospace, monospace',
        theme: {
          background: style.getPropertyValue('--surface-code').trim(),
          foreground: style.getPropertyValue('--text-primary').trim(),
          cursor: style.getPropertyValue('--accent').trim(),
        },
      });
      if (controller.signal.aborted) { opened.dispose(); return; }
      terminal = opened;
      setBackend(opened.renderer.backend);
      opened.on('renderer', ({ backend: drawn }) => setBackend(drawn));
      opened.on('error', failed);
      const resize = ({ cols, rows }: { cols: number; rows: number }) => {
        resizeWork = resizeWork.then(async () => {
          if (controller.signal.aborted) return;
          await terminalRequest('/runtime/sessions/' + encodeURIComponent(session) + '/resize', { columns: cols, rows });
        });
        void resizeWork.catch(failed);
      };
      opened.on('resize', resize);
      resize(opened.geometry);
      await resizeWork;
      if (controller.signal.aborted) return;
      const connection = opened.connect(terminalStreams(session, controller, (end) => callbacks.current.onEnd(end)), { signal: controller.signal });
      connection.onStatusChange((status) => { if (status === 'error') failed(connection.error ?? new Error('Terminal transport failed.')); });
      void connection.closed.catch(failed);
      setStatus('ready');
    };
    void start().catch(failed);
    return () => { controller.abort(); terminal?.dispose(); };
  }, [session]);
  return <div className="terminal-display">
    {status !== 'ready' ? <p role="status">{status === 'failed' ? 'Terminal disconnected. See the reason below.' : 'Opening terminal…'}</p> : null}
    {status === 'ready' && backend !== 'webgpu' ? <p className="terminal-renderer">This browser is not offering WebGPU, so the terminal is drawn with {RENDERERS[backend]} instead.</p> : null}
    <div className="terminal-screen" data-renderer={status === 'ready' ? backend : status} ref={host} />
  </div>;
}
