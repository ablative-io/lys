/**
 * Saving the folder an agent works in, by itself: the agent's settings are
 * read, kept as they are, and saved again as the next version with the
 * folder named. The answer must confirm that version and that folder.
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
    ...(profile.permissions ? { permissions: profile.permissions } : {}), ...(profile.session ? { session: profile.session } : {}),
    ...(profile.runs_on ? { runs_on: profile.runs_on } : {}), ...(profile.writable ? { writable: profile.writable } : {}),
  });
  if (saved.agent !== agent || saved.recorded?.operation !== operation || saved.profile?.working_folder !== folder) {
    throw new Refused(200, { refusal: 'ProfileReceiptMismatch', reason: 'The answer did not confirm the folder was saved.' });
  }
}
