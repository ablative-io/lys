/**
 * The one set of words for deciding (CONSENSUS): whatever waits on a person (a request, an agent's draft, a product's
 * draft, access due a review) is approved or declined, and access is given and taken back. No page says Keep,
 * Revoke or Refuse. Every decide surface takes its verbs and its state words from here, so they cannot drift apart.
 */
export const APPROVE = 'Approve';
export const DECLINE = 'Decline';
export const TAKE_BACK = 'Take back';

/** What became of a draft, the same words for an agent's draft and a product's, by the state the service answers. */
export const DRAFT_STATE: Record<string, string> = {
  waiting: 'Waiting on a decision',
  approved: 'Approved, not yet carried out',
  refused: 'Declined',
  executed: 'Approved and carried out',
  refused_on_execution: 'Approved, but stopped when carried out',
  corrected: 'Corrected',
};
