/**
 * Connect, under an agent's Credentials (AGENTS-016 R2, AGENTS-017 R1): the lines network/connectLines.ts builds for
 * this agent from this page's own origin, the agent's id and the seat file its profile names, each with the same copy
 * act the Connect group of a computer uses. The enrol line comes first: it makes the key where the agent runs and
 * writes the seat file at that path, which the two mcp lines then read. The seat file's path is read from the agent's
 * profile (its seat program's `seat_file`); a profile that names none, or one that cannot be read, gives the default
 * path, never a placeholder. Nothing is issued, posted or stored here. A person's file has no Connect section.
 */
import { useEffect, useState } from 'react';
import { request } from '../../api';
import { kindOf } from '../../generated';
import { Copy } from '../network/JoinCode';
import { CONNECT_INTRO, connectLines, defaultSeatFile } from '../network/connectLines';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';

/** Whether `origin` is plain http on this computer, which `lys mcp` reaches without the https bridge. */
const loopback = (origin: string): boolean => {
  try {
    const url = new URL(origin);
    return url.protocol === 'http:' && ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname);
  } catch {
    return false;
  }
};

/** The seat file the agent's profile names, null when it names none or cannot be read; undefined while reading. */
function useSeatFile(id: string): string | null | undefined {
  const [seatFile, setSeatFile] = useState<{ id: string; path: string | null } | null>(null);
  useEffect(() => {
    if (kindOf(id) !== 'agent') return undefined;
    let live = true;
    request<ProvisioningAnswer>('/agents/' + encodeURIComponent(id) + '/provisioning').then(
      (answer) => { if (live) setSeatFile({ id, path: answer.profile?.harness?.description.seat_program?.seat_file ?? null }); },
      () => { if (live) setSeatFile({ id, path: null }); },
    );
    return () => { live = false; };
  }, [id]);
  return seatFile?.id === id ? seatFile.path : undefined;
}

export function AgentConnect({ id, origin = window.location.origin }: { id: string; origin?: string }) {
  const seatFile = useSeatFile(id);
  if (kindOf(id) !== 'agent') return <span data-connect-ready hidden />;
  if (seatFile === undefined) return <span data-connect-reading hidden />;
  const keyFile = seatFile ?? defaultSeatFile(id);
  const lines = connectLines({ id, origin, keyFile });
  return <section className="card" aria-label="Connect" data-connect-ready>
    <h2>Connect</h2>
    <p>{CONNECT_INTRO}</p>
    {lines.map(({ what, line }) => <p key={what}><code data-connect-line>{line}</code> <Copy text={line} what={what} /></p>)}
    <p className="sec" data-connect-seat-file>{seatFile
      ? `The seat file is written at ${seatFile}, the path this agent’s profile names; it is never replaced once written.`
      : `This agent’s profile names no seat file, so the lines use ${keyFile}; it is never replaced once written.`}</p>
    <p data-connect-works>{loopback(origin)
      ? 'With this address the lines work now: lys reaches Lys over http on this computer.'
      : 'With this address the lines need a lys that carries the https bridge of AGENTS-015 R3; with an older lys they are refused.'}</p>
  </section>;
}
