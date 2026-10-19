/**
 * What the person's agents registered and started for them (AGENTS-016 R5), under one heading, newest first, each
 * line in plain words with Stop beside it. Stop opens the agent file's own confirmation (EmergencyStop) for the
 * registered or started agent and shows its receipt; nothing here posts without that confirmation. No lines, no heading.
 */
import { useState } from 'react';
import { Act } from '../../shell/Act';
import { EmergencyStop, StopReceipt } from '../file/EmergencyStop';
import type { StopAnswer } from '../file/EmergencyStop';
import { clock } from '../file/time';
import type { ActedLine, PartRefused } from './contract';
import { refusedPart } from './contract';
import { Unread } from './Unread';

const said = (line: ActedLine, name: (id: string) => string): string =>
  `${name(line.acting_agent)} ${line.kind} ${name(line.agent)} for you, ${clock(line.at)}.`;

function Line({ line, name, changed }: { line: ActedLine; name: (id: string) => string; changed: () => void }) {
  const [asking, setAsking] = useState(false);
  const [receipt, setReceipt] = useState<StopAnswer | null>(null);
  return <li data-acted-line={line.kind}>
    <span data-acted-words>{said(line, name)}</span>{' '}
    {receipt ? null : <Act symbol="stop" name={`Stop ${name(line.agent)}`} word="Stop" tone="danger" data-act="stop" disabled={asking} onClick={() => setAsking(true)} />}
    <EmergencyStop id={line.agent} active open={asking} cancel={() => setAsking(false)}
      stopped={(answer) => { setAsking(false); setReceipt(answer); changed(); }} />
    {receipt ? <StopReceipt answer={receipt} /> : null}
  </li>;
}

export function ActedByAgents({ lines, names, changed }: { lines: ActedLine[] | PartRefused | undefined; names: Map<string, string>; changed: () => void }) {
  if (lines === undefined) return <span data-acted-ready hidden />;
  if (refusedPart(lines)) return <Unread part={lines} words="What your agents registered and started for you could not be read." />;
  if (!lines.length) return <span data-acted-ready hidden />;
  const name = (id: string) => names.get(id) ?? id;
  return <section className="you-panel dash-acted" aria-label="Acted for you by your agents" data-acted-ready>
    <h2 className="section-h">Acted for you by your agents</h2>
    <ul>{lines.map((line) => <Line key={line.kind + line.agent + line.at} line={line} name={name} changed={changed} />)}</ul>
  </section>;
}
