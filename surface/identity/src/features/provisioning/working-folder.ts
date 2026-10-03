/**
 * Saving the folder an agent works in, by itself: the agent's settings are
 * read, kept as they are, and saved again as the next version with the
 * folder named. The answer must confirm that version and that folder.
 *
 * Every member the server's save takes is sent (SetBody,
 * crates/lys-identity-server/src/provisioning_api.rs:149-176): operation,
 * from_version, model_access, tools, skills, mcp_servers, instructions,
 * instructions_mode, note, session, harness, permissions, runs_on, writable,
 * working_folder. The note is this act's own. Two things a read and a write
 * cannot carry, because the server sets them itself: the new version is not
 * reviewed, and its skills are pinned again at that review
 * (provisioning_api.rs:254, :470). A route that sets the folder alone would
 * carry both; that is a design row, not built here.
 */
import { Refused, request } from '../../api';
import type { ProvisioningAnswer } from './Provisioning';

export async function saveFolder(agent: string, folder: string, operation: string): Promise<void> {
  const path = '/agents/' + encodeURIComponent(agent) + '/provisioning';
  const held = await request<ProvisioningAnswer>(path);
  const profile = held.profile;
  if (held.agent !== agent || !profile) throw new Refused(200, { refusal: 'ProfileMissing', reason: 'This agent has no saved settings yet. Save its settings, then choose its folder.' });
  const saved = await request<ProvisioningAnswer>(path, {
    operation, from_version: profile.version, model_access: profile.model_access, tools: profile.tools, skills: profile.skills, mcp_servers: profile.mcp_servers,
    instructions: profile.instructions, note: 'Choose the folder this agent works in', working_folder: folder,
    ...(profile.instructions_mode ? { instructions_mode: profile.instructions_mode } : {}), ...(profile.harness ? { harness: profile.harness } : {}),
    // Permissions are sent whole, exactly as read: every rule list, the mode and the extra folders.
    ...(profile.permissions ? { permissions: profile.permissions } : {}), ...(profile.session ? { session: profile.session } : {}),
    ...(profile.runs_on ? { runs_on: profile.runs_on } : {}), ...(profile.writable ? { writable: profile.writable } : {}),
  });
  if (saved.agent !== agent || saved.recorded?.operation !== operation || saved.profile?.working_folder !== folder) {
    throw new Refused(200, { refusal: 'ProfileReceiptMismatch', reason: 'The answer did not confirm the folder was saved.' });
  }
}
