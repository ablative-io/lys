import { useEffect, useState } from 'react';
import { Refused, request } from '../../api';
import { isRecord, strings } from '../network/contract';
import type { Machine } from '../network/contract';
import { ErrorWords } from '../people/Words';
import type { Program } from './choices';
import { commandOrAbsolutePath, launchCommand, plainPath } from './harness-program';

type Computer = Pick<Machine, 'id' | 'name'>;
export interface ComputerPrograms {
  machine: Computer; programs: Program[]; problem: unknown | null; failed: boolean;
}
export const copyProgram = (copy: Program['builds'][number]): string => copy.command ?? copy.program;
export const copyValue = (copy: Program['builds'][number]): string => copyProgram(copy) + '\n' + copy.package;

export function distinctCopies(copies: Program['builds']): Program['builds'] {
  const commands = new Map<string, Program['builds'][number]>();
  for (const copy of copies) {
    const command = launchCommand(copyProgram(copy));
    const held = commands.get(command);
    // A command stays one choice when profiles keep older names or versions of it.
    if (!held || copyProgram(copy) === command && copyProgram(held) !== command) commands.set(command, copy);
  }
  return [...commands.values()];
}

function readableDescription(value: unknown): boolean {
  if (!isRecord(value) || !isRecord(value.models) || !isRecord(value.models.further_encoding)
    || !isRecord(value.permissions) || !isRecord(value.mcp)) return false;
  const encoding = value.models.further_encoding;
  return typeof value.rendering_contract === 'string' && typeof value.models.minimum === 'number'
    && (value.models.maximum === null || typeof value.models.maximum === 'number')
    && (encoding.kind === 'array' || encoding.kind === 'delimited' && typeof encoding.separator === 'string')
    && strings(value.permissions.modes) && strings(value.permissions.rule_forms)
    && strings(value.mcp.transports) && strings(value.mcp.channel_policies)
    && typeof value.mcp.working_directory === 'boolean' && typeof value.mcp.handle_variables === 'boolean';
}

function readableProgram(value: unknown, machine: string): value is Program {
  return isRecord(value) && typeof value.name === 'string' && Boolean(value.name.trim()) && typeof value.line === 'string'
    && Array.isArray(value.models) && value.models.every((model) => isRecord(model) && typeof model.id === 'string' && typeof model.label === 'string')
    && Array.isArray(value.modes) && value.modes.every((mode) => isRecord(mode) && typeof mode.id === 'string' && typeof mode.meaning === 'string')
    && (value.instructions_modes === undefined || strings(value.instructions_modes) && value.instructions_modes.every((mode) => ['keep', 'append', 'replace'].includes(mode)))
    && readableDescription(value.description) && (value.not_found === undefined || typeof value.not_found === 'string')
    && Array.isArray(value.builds) && value.builds.every((copy) => isRecord(copy)
      && typeof copy.name === 'string' && Boolean(copy.name.trim()) && typeof copy.package === 'string' && Boolean(copy.package.trim())
      && typeof copy.program === 'string' && commandOrAbsolutePath(copy.program)
      && (copy.command === undefined || copy.command === null || typeof copy.command === 'string' && !copy.command.startsWith('/') && commandOrAbsolutePath(copy.command))
      && (copy.from === 'profile' || copy.from === 'runner' || copy.from === 'installed')
      && (copy.machine === undefined || copy.machine === machine));
}

/** A computer reports where it found each copy through its login's PATH, which may name a folder through `..`; the
 *  copy is the same file at its plain path, so it is read at that path rather than refusing the computer's whole answer. */
function plainCopies(value: unknown): unknown {
  if (!isRecord(value) || !Array.isArray(value.programs)) return value;
  return { ...value, programs: value.programs.map((program) => !isRecord(program) || !Array.isArray(program.builds) ? program
    : { ...program, builds: program.builds.map((copy) => isRecord(copy) && typeof copy.program === 'string' ? { ...copy, program: plainPath(copy.program) } : copy) }) };
}

function computerPrograms(answer: unknown, machine: Computer): Program[] {
  const value = plainCopies(answer);
  if (!isRecord(value) || value.machine !== undefined && value.machine !== machine.id
    || value.reason !== undefined && typeof value.reason !== 'string'
    || !Array.isArray(value.programs) || !value.programs.every((program) => readableProgram(program, machine.id))
    || new Set(value.programs.map((program) => program.name)).size !== value.programs.length) {
    throw new Refused(200, { refusal: 'harness_programs_unreadable', reason: 'The installed-program answer for ' + machine.name + ' did not match that computer or the program-list format.' });
  }
  return value.programs.map((program) => ({ ...program, builds: distinctCopies(program.builds) }));
}

/** Installed copies belong to the computer explicitly asked, never to the aggregate profile list. */
export function useProfileCopies(machines: Computer[], preferred: string | undefined): { catalogues: ComputerPrograms[]; pending: boolean } {
  const computers = preferred ? machines.filter((machine) => machine.id === preferred) : machines;
  const key = JSON.stringify(computers.map(({ id, name }) => ({ id, name })));
  const [read, setRead] = useState<{ key: string; catalogues: ComputerPrograms[] } | null>(null);
  useEffect(() => {
    let current = true;
    const asked: Computer[] = JSON.parse(key);
    void Promise.all(asked.map(async (machine): Promise<ComputerPrograms> => {
      try {
        const answer = await request<unknown>('/harnesses?machine=' + encodeURIComponent(machine.id));
        const programs = computerPrograms(answer, machine);
        const reason = isRecord(answer) && typeof answer.reason === 'string' ? answer.reason : '';
        return { machine, programs, problem: reason || null, failed: false };
      } catch (problem) { return { machine, programs: [], problem, failed: true }; }
    })).then((catalogues) => { if (current) setRead({ key, catalogues }); });
    return () => { current = false; };
  }, [key]);
  return { catalogues: read?.key === key ? read.catalogues : [], pending: computers.length > 0 && read?.key !== key };
}

export function ProfileCopiesProblems({ catalogues }: { catalogues: ComputerPrograms[] }) {
  return <>{catalogues.filter((catalogue) => catalogue.problem !== null).map((catalogue) => <div className="wide" key={catalogue.machine.id}>
    <p>Installed programs on {catalogue.machine.name}</p><ErrorWords problem={catalogue.problem} />
  </div>)}</>;
}
