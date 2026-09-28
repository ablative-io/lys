/** A mutation is confirmed only by a receipt for the operation this form sent. */
import { Refused } from '../../api';

function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function natural(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function text(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

function matches(value: unknown, operation: string, grant: boolean): boolean {
  if (!object(value) || !object(value.receipt)) return false;
  const receipt = value.receipt;
  const log = receipt.log;
  if (!object(log) || !natural(log.index) || !natural(log.tree_size) || log.tree_size <= log.index
    || !text(log.root) || !text(log.leaf_hash) || !natural(receipt.version)
    || !natural(receipt.change_kind) || !text(receipt.payload_commitment)
    || receipt.payload_commitment_hash !== 'sha-256') return false;
  if (grant) return value.operation === operation && text(value.grant)
    && value.index === log.index && text(receipt.caller) && natural(receipt.revision);
  return receipt.operation === operation && text(receipt.identity) && object(receipt.actor)
    && text(receipt.actor.issuer) && text(receipt.actor.subject) && natural(receipt.actor.authenticated_at);
}

export function confirmReceipt(value: unknown, operation: string, path: string): void {
  if (!matches(value, operation, path.startsWith('/grants/'))) {
    throw new Refused(200, {
      refusal: 'UnconfirmedReceipt',
      reason: `the identity service did not return a complete receipt for operation ${operation}; its outcome remains unknown`,
    });
  }
}
