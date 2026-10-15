/**
 * A part of the Dashboard that could not be read, said in plain words where the part would stand. The service's
 * refusal name and reason are never shown on the face: pressing the words opens what it means and what to do,
 * and the code itself sits one fold further in, under Error details, for whoever has to report it.
 */
import { Refused } from '../../api';
import { ErrorWords } from '../people/Words';
import type { PartRefused } from './contract';

export function Unread({ part, words = 'Could not be read' }: { part: PartRefused | Refused; words?: string }) {
  const problem = part instanceof Refused ? part : new Refused(0, part);
  return <details className="dash-unread" data-unread-part={problem.refusal.refusal}>
    <summary className="why-not">{words}</summary>
    <ErrorWords problem={problem} />
  </details>;
}
