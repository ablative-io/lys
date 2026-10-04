/**
 * A call read as what went in and what came out: the provider's own request and response taken apart into the pieces
 * a person reads, in their order. Nothing is guessed: a body that is not in the shape read here answers null, and the
 * view says so and points at the JSON, which is always whole.
 */

/** One piece of a message: words, thinking, a tool asked for, a tool's result, or a part this view does not know. */
export interface Piece { kind: 'text' | 'thinking' | 'tool' | 'result' | 'other'; label: string; text: string | null; json: unknown }
export interface Turn { role: string; pieces: Piece[] }
export interface ReadIn { settings: [string, string][]; system: Piece[]; turns: Turn[]; tools: string[] }
export interface ReadOut { pieces: Piece[]; stop: string | null; error: { type: string; message: string } | null }

type Json = Record<string, unknown>;
const object = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);
const word = (value: unknown): string | null => typeof value === 'string' ? value : null;

function piece(block: unknown): Piece {
  if (!object(block)) return { kind: 'other', label: 'Part', text: word(block), json: block };
  const type = word(block.type) ?? '';
  if (type === 'text') return { kind: 'text', label: 'Text', text: word(block.text), json: block };
  if (type === 'thinking' || type === 'redacted_thinking') return { kind: 'thinking', label: 'Thinking', text: word(block.thinking), json: block };
  if (type === 'tool_use' || type === 'server_tool_use') return { kind: 'tool', label: word(block.name) ?? 'A tool', text: null, json: block.input };
  if (type === 'tool_result') {
    const said = typeof block.content === 'string' ? block.content : Array.isArray(block.content) ? block.content.map((each) => object(each) ? word(each.text) : null) : null;
    const text = typeof said === 'string' ? said : said && said.every((each) => each !== null) ? said.join('\n') : null;
    return { kind: 'result', label: block.is_error === true ? 'Tool result, an error' : 'Tool result', text, json: block.content };
  }
  return { kind: 'other', label: type || 'Part', text: null, json: block };
}

/** A message's content, which a provider gives as words or as a list of parts. */
const pieces = (content: unknown): Piece[] => typeof content === 'string' ? [{ kind: 'text', label: 'Text', text: content, json: content }] : Array.isArray(content) ? content.map(piece) : [];

/** The request as it is read; null when it is not a list of messages. */
export function readIn(request: unknown): ReadIn | null {
  if (!object(request) || !Array.isArray(request.messages)) return null;
  const said = (name: string, value: unknown): [string, string][] => value === undefined || value === null || typeof value === 'object' ? [] : [[name, String(value)]];
  const settings = [...said('Model', request.model), ...said('Most tokens out', request.max_tokens), ...said('Streamed', request.stream),
    ...said('Thinking', object(request.thinking) ? request.thinking.type : undefined), ...said('Effort', object(request.output_config) ? request.output_config.effort : undefined)];
  return {
    settings, system: pieces(request.system),
    turns: request.messages.map((each) => object(each) ? { role: word(each.role) ?? 'Not said', pieces: pieces(each.content) } : { role: 'Not said', pieces: [piece(each)] }),
    tools: Array.isArray(request.tools) ? request.tools.map((each) => object(each) ? word(each.name) ?? 'A tool with no name' : 'A tool with no name') : [],
  };
}

const failure = (error: unknown): ReadOut['error'] => object(error) ? { type: word(error.type) ?? 'error', message: word(error.message) ?? '' } : null;

/** A streamed response's events put back together: each part as its start, with what its deltas added. */
function streamed(events: unknown[]): ReadOut | null {
  const parts = new Map<number, { block: Json; text: string; thinking: string; input: string }>();
  let [stop, error, seen]: [string | null, ReadOut['error'], boolean] = [null, null, false];
  for (const event of events) {
    const data = object(event) && object(event.data) ? event.data : null;
    if (!data) continue;
    const [type, index] = [word(data.type), typeof data.index === 'number' ? data.index : -1];
    if (type === 'message_start') seen = true;
    if (type === 'content_block_start' && object(data.content_block)) parts.set(index, { block: data.content_block, text: word(data.content_block.text) ?? '', thinking: word(data.content_block.thinking) ?? '', input: '' });
    const [part, delta] = [parts.get(index), object(data.delta) ? data.delta : null];
    if (type === 'content_block_delta' && part && delta) {
      part.text += word(delta.text) ?? '';
      part.thinking += word(delta.thinking) ?? '';
      part.input += word(delta.partial_json) ?? '';
    }
    if (type === 'message_delta' && delta) stop = word(delta.stop_reason) ?? stop;
    if (type === 'error') error = failure(data.error);
  }
  if (!seen && !error) return null;
  const whole = [...parts.entries()].sort(([left], [right]) => left - right).map(([, part]) => {
    let input: unknown = part.block.input;
    if (part.input) { try { input = JSON.parse(part.input); } catch { input = part.input; } }
    return piece({ ...part.block, text: part.text, thinking: part.thinking || part.block.thinking, input });
  });
  return { pieces: whole, stop, error };
}

/** The response as it is read: one message, a stream of events, or the provider's error; null when it is none of them. */
export function readOut(response: unknown): ReadOut | null {
  if (Array.isArray(response)) return streamed(response);
  if (!object(response)) return null;
  if (response.type === 'error' || object(response.error)) return { pieces: [], stop: null, error: failure(response.error) ?? { type: 'error', message: '' } };
  if (!Array.isArray(response.content)) return null;
  return { pieces: pieces(response.content), stop: word(response.stop_reason), error: null };
}
