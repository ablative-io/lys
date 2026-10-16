/** A changed seat is reviewed from the runner's exact found digests, never from a guessed file. */
import { useRef, useState } from 'react';
import { operationId, request } from '../../api';
import { answeredNo } from '../../kept';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';
import type { SeatProgramEvidence } from '../runtime/RuntimeSessions';
import type { ProvisioningAnswer } from './Provisioning';

export function SeatProgramChange({ id, evidence, canReview, changed }: { id: string; evidence: SeatProgramEvidence; canReview: boolean; changed: () => void }) {
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<unknown>(null);
  const operation = useRef('');
  const valid = evidence.contract === 'seat-program/v1' && Number.isSafeInteger(evidence.provisioning_version) && evidence.provisioning_version > 0
    && typeof evidence.executable === 'string' && typeof evidence.seat_file === 'string'
    && [evidence.reviewed_executable_sha256, evidence.reviewed_seat_file_sha256, evidence.found_executable_sha256, evidence.found_seat_file_sha256]
      .every((digest) => typeof digest === 'string' && /^[0-9a-f]{64}$/.test(digest));
  if (!valid) return <ErrorWords problem={new Error('SeatProgramEvidenceUnreadable: the runner did not supply the reviewed and found seat digests.')} />;
  const review = async () => {
    if (busy) return;
    setBusy(true); setProblem(null);
    operation.current ||= operationId();
    try {
      const answer = await request<ProvisioningAnswer>('/agents/' + encodeURIComponent(id) + '/provisioning/' + evidence.provisioning_version + '/seat-program-review', {
        operation: operation.current, executable_sha256: evidence.found_executable_sha256, seat_file_sha256: evidence.found_seat_file_sha256,
      });
      const seat = answer.profile?.harness?.description.seat_program;
      if (answer.agent !== id || answer.recorded?.operation !== operation.current || answer.recorded.version !== evidence.provisioning_version + 1
        || answer.profile?.version !== answer.recorded.version || !answer.profile.reviewed_by
        || seat?.executable_sha256 !== evidence.found_executable_sha256 || seat.seat_file_sha256 !== evidence.found_seat_file_sha256) {
        throw new Error('SeatProgramReviewUnconfirmed: Lys did not confirm the found digests as a reviewed settings version.');
      }
      operation.current = '';
      changed();
    } catch (error) {
      if (answeredNo(error)) operation.current = '';
      setProblem(error);
    } finally { setBusy(false); }
  };
  return <section aria-label="Changed seat program"><h3>The seat program changed</h3>
    <dl><dt>Executable</dt><dd>{evidence.executable}</dd>
      <dt>Reviewed executable SHA-256</dt><dd>{evidence.reviewed_executable_sha256}</dd>
      <dt>Found executable SHA-256</dt><dd>{evidence.found_executable_sha256}</dd>
      <dt>Seat file</dt><dd>{evidence.seat_file}</dd>
      <dt>Reviewed seat file SHA-256</dt><dd>{evidence.reviewed_seat_file_sha256}</dd>
      <dt>Found seat file SHA-256</dt><dd>{evidence.found_seat_file_sha256}</dd></dl>
    {canReview ? <Act symbol="approve" name="Re-review" word="Re-review" disabled={busy} onClick={() => void review()} /> : null}
    {problem ? <ErrorWords problem={problem} /> : null}
  </section>;
}
