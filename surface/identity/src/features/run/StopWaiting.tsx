/** Escalation is a person's act; elapsed time only tells them how long it has waited. */
import { Act } from '../../shell/Act';
import { Elapsed } from '../health/Elapsed';

export function StopWaiting({ since, ending, endNow }: { since: number; ending: boolean; endNow: () => void }) {
  return <div role="status">
    <p>Waiting for the process to exit: <Elapsed since={since} /></p>
    <Act symbol="stop" name="End now" word="End now" tone="danger" disabled={ending} onClick={endNow} />
  </div>;
}
