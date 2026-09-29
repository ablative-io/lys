/** Team route records describe membership, separately from grant authority. */
import type { Login } from '../../generated';
export interface Team { id: string; owner: string; name: string; description: string; members: string[]; held?: { member: string; reason: string }[]; state: 'active' | 'retired'; created_by: Login; created_at: number; retired_at: number | null }
export interface TeamChanged extends Team { recorded: { operation: string; act: 'created' | 'added' | 'removed' | 'retired' | 'confirmed'; member: string | null; by: Login; at: number } }
export interface Member { id: string; display_name: string; state: string }
export const sameLogin = (left: Login, right: Login) => left.provider === right.provider && left.subject === right.subject;
