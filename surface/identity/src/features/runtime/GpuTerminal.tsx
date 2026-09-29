/** Ghostty interprets raw PTY bytes; strict WebGPU selection never silently becomes another renderer. */
import { useEffect, useRef, useState } from 'react';
import type { BrowserTerminal } from '@gespenst/core';
import '@gespenst/core/style.css';
import { terminalRequest, terminalStreams } from './terminal-transport';
import type { SessionEnd } from './Terminal';

export function GpuTerminal({ session, onEnd, onFailure }: {
  session: string; onEnd: (end: SessionEnd) => void; onFailure: (error: unknown) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onEnd, onFailure });
  callbacks.current = { onEnd, onFailure };
  const [status, setStatus] = useState<'starting' | 'ready' | 'failed'>('starting');
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
        container, renderer: 'webgpu', worker: 'dedicated', accessibility: 'full',
        ariaLabel: 'Live agent terminal', fontFamily: '"JetBrains Mono", ui-monospace, monospace',
        theme: {
          background: style.getPropertyValue('--surface-code').trim(),
          foreground: style.getPropertyValue('--text-primary').trim(),
          cursor: style.getPropertyValue('--accent').trim(),
        },
      });
      if (controller.signal.aborted) { opened.dispose(); return; }
      terminal = opened;
      if (opened.renderer.backend !== 'webgpu') throw new Error('The requested WebGPU renderer is unavailable.');
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
    <div className="terminal-screen" data-renderer={status === 'ready' ? 'webgpu' : status} ref={host} />
  </div>;
}
