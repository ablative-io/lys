/** A small live view of one session: eighty by twenty-four drawn at full size and scaled down as one picture, so the session is never resized and its shape holds. It takes no input; clicking it opens the real terminal. Given no width it fills its box and measures itself. */
import { useLayoutEffect, useRef, useState } from 'react';
import { GpuTerminal, PEEK } from './GpuTerminal';
import './peek.css';

export function Peek({ session, name, machine, width }: { session: string; name: string; machine: string | null; width?: number }) {
  const [failed, setFailed] = useState<string | null>(null);
  const [ended, setEnded] = useState(false);
  const frame = useRef<HTMLDivElement>(null);
  const [measured, setMeasured] = useState(0);
  useLayoutEffect(() => {
    if (width !== undefined) return;
    const element = frame.current;
    if (!element) return;
    const read = () => setMeasured(element.clientWidth || PEEK.width / 2);
    read();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => observer.disconnect();
  }, [width]);
  const box = width ?? measured;
  const scale = box / PEEK.width;
  return <a className="peek" href={'#/runtime/' + encodeURIComponent(session)} style={width !== undefined ? { width } : undefined} aria-label={'Open the terminal of ' + name}>
    <div className="peek-name">{name}{machine ? <span className="peek-machine"> on {machine}</span> : null}{ended ? <span className="peek-machine"> · ended</span> : null}</div>
    <div className="peek-frame" ref={frame} style={{ height: Math.round(PEEK.height * scale) }}>
      {box > 0 ? <div className="peek-screen" style={{ width: PEEK.width, height: PEEK.height, transform: `scale(${scale})` }} inert aria-hidden="true">
        <GpuTerminal session={session} peek onEnd={() => setEnded(true)} onFailure={(error) => setFailed(String(error))} />
      </div> : null}
    </div>
    {failed ? <p className="peek-why">{failed}</p> : null}
  </a>;
}
