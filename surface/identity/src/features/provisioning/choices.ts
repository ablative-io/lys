/** What a profile is chosen from, each read from the service; a list the service does not answer yet is named, never filled in. */
import { Refused, request } from '../../api';
import type { NetworkView } from '../network/contract';
import type { HarnessDescription } from './Provisioning';

/** One program Lys can start, as GET /harnesses answers it (DIRECTORY-076 R1). */
export interface Program {
  name: string; line: string;
  models: { id: string; label: string }[];
  modes: { id: string; meaning: string }[];
  instructions_modes?: ('keep' | 'append' | 'replace')[];
  description: HarnessDescription;
  builds: { name: string; program: string; package: string; from: 'profile' | 'runner'; machine?: string }[];
}
export interface Choices {
  programs: Program[] | null; programsMissing: string;
  machines: NetworkView['machines'];
  skills: string[] | null;
  secrets: string[] | null;
}

async function optional<T>(path: string): Promise<{ value: T | null; missing: string }> {
  try { return { value: await request<T>(path), missing: '' }; } catch (error) {
    if (error instanceof Refused) return { value: null, missing: error.refusal.refusal + ': ' + error.refusal.reason };
    throw error;
  }
}

export async function readChoices(): Promise<Choices> {
  const [programs, network, skills, secrets] = await Promise.all([
    optional<{ programs: Program[] }>('/harnesses'),
    optional<NetworkView>('/network'),
    optional<{ skills: { name: string }[] }>('/skills'),
    optional<{ secrets: { name: string }[] }>('/secrets'),
  ]);
  return {
    programs: programs.value?.programs ?? null, programsMissing: programs.missing,
    machines: (network.value?.machines ?? []).filter((machine) => machine.state === 'in_use'),
    skills: skills.value ? skills.value.skills.map((skill) => skill.name) : null,
    secrets: secrets.value ? secrets.value.secrets.map((secret) => secret.name) : null,
  };
}
