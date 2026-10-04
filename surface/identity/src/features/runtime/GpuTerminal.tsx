/** Ghostty interprets raw PTY bytes, drawn on WebGPU when the browser offers it; any other renderer the library falls to is named on the page, never silent. */
import { useEffect, useRef, useState } from 'react';
import type { BrowserTerminal } from '@gespenst/core';
import '@gespenst/core/style.css';
import { terminalRequest, terminalStreams } from './terminal-transport';
import type { SessionEnd } from './Terminal';

type Backend = BrowserTerminal['renderer']['backend'];
const RENDERERS: Record<Backend, string> = { webgpu: 'WebGPU', webgl2: 'WebGL2', canvas2d: 'Canvas 2D' };

/**
 * The editing keys a Mac's own terminals give: Command and Option with Delete and the arrows. The library encodes a
 * key as the terminal protocol says; these are sent as the control bytes a line editor acts on, as Terminal and
 * Ghostty send them.
 */
export const MAC_KEYS: Record<string, string> = {
  'meta+Backspace': '\x15', // delete to the start of the line
  'meta+Delete': '\x0b', // delete to the end of the line
  'meta+ArrowLeft': '\x01', // go to the start of the line
  'meta+ArrowRight': '\x05', // go to the end of the line
  'alt+Backspace': '\x1b\x7f', // delete the word before
  'alt+Delete': '\x1bd', // delete the word after
  'alt+ArrowLeft': '\x1bb', // go back a word
  'alt+ArrowRight': '\x1bf', // go forward a word
};

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
      // One session has one size, and another window showing it may have set its own. The window a person is using
      // says its size again when they click or type into it, so what they look at always fills what they see.
      const claim = () => resize(opened.geometry);
      container.addEventListener('pointerdown', claim, { signal: controller.signal });
      container.addEventListener('focusin', claim, { signal: controller.signal });
      const macKey = (event: KeyboardEvent) => {
        if (event.ctrlKey || event.shiftKey || event.metaKey === event.altKey) return;
        const bytes = MAC_KEYS[(event.metaKey ? 'meta+' : 'alt+') + event.key];
        if (bytes === undefined) return;
        event.preventDefault();
        event.stopPropagation();
        opened.sendText(bytes);
      };
      container.addEventListener('keydown', macKey, { capture: true, signal: controller.signal });
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
