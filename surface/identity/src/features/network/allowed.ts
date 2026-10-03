/** Which computers may run an agent: one rule, used wherever a screen has to say where an agent can start. */
import type { Role } from '../roles/contract';
import type { Machine } from './contract';

/** The computers that could run any agent: in use, with Lys running on them. */
export const usable = (machines: Machine[]): Machine[] => machines.filter((machine) => machine.state === 'in_use' && machine.runtime !== null);

/** The roles the agent holds now; a role it held and no longer holds allows nothing. */
export const heldRoles = (roles: Role[], agent: string): string[] =>
  roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id);

/** The usable computers allowed to run the agent: by its own name, or by a role it holds now. */
export function allowedToRun(machines: Machine[], roles: Role[], agent: string): Machine[] {
  const held = heldRoles(roles, agent);
  return usable(machines).filter((machine) => machine.may_run.some((holder) => holder.id === agent) || Boolean(machine.may_run_roles?.some((role) => held.includes(role))));
}
