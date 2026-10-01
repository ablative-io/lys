import { request } from '../../api';
import { machineAdmissionOf } from '../network/machine-admission';

export interface RegistrationCapability { answersTo: boolean; problem: unknown }
export interface AddAndRunCapability { answersTo: boolean | null; problem: unknown; machineAdmission: boolean | null; admissionProblem: unknown }
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function member(value: unknown, key: string): unknown {
  if (!object(value) || !(key in value)) throw new Error('The service did not describe its agent registration fields.');
  return value[key];
}

function registrationOf(document: unknown): boolean {
  const paths = member(document, 'paths');
  const body = member(member(member(paths, '/agents'), 'post'), 'requestBody');
  let schema = member(member(member(body, 'content'), 'application/json'), 'schema');
  if (object(schema) && typeof schema.$ref === 'string') {
    const prefix = '#/components/schemas/';
    if (!schema.$ref.startsWith(prefix)) throw new Error('The registration fields refer to an unsupported schema.');
    schema = member(member(member(document, 'components'), 'schemas'), schema.$ref.slice(prefix.length));
  }
  const properties = member(schema, 'properties');
  if (!object(properties)) throw new Error('The registration fields could not be read.');
  return 'answers_to' in properties;
}

export async function registrationCapability(): Promise<RegistrationCapability> {
  try { return { answersTo: registrationOf(await request<unknown>('/openapi.json')), problem: null }; }
  catch (problem) { return { answersTo: false, problem }; }
}

export async function addAndRunCapability(): Promise<AddAndRunCapability> {
  let document: unknown;
  try { document = await request<unknown>('/openapi.json'); }
  catch (problem) { return { answersTo: null, problem, machineAdmission: null, admissionProblem: problem }; }
  let answersTo: boolean | null = null; let problem: unknown = null;
  let machineAdmission: boolean | null = null; let admissionProblem: unknown = null;
  try { answersTo = registrationOf(document); } catch (error) { problem = error; }
  try { machineAdmission = machineAdmissionOf(document); } catch (error) { admissionProblem = error; }
  return { answersTo, problem, machineAdmission, admissionProblem };
}
