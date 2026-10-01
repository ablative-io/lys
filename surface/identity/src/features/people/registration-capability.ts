import { request } from '../../api';

export interface RegistrationCapability { answersTo: boolean; problem: unknown }
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function member(value: unknown, key: string): unknown {
  if (!object(value) || !(key in value)) throw new Error('The service did not describe its agent registration fields.');
  return value[key];
}

export async function registrationCapability(): Promise<RegistrationCapability> {
  try {
    const document = await request<unknown>('/openapi.json');
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
    return { answersTo: 'answers_to' in properties, problem: null };
  } catch (problem) { return { answersTo: false, problem }; }
}
