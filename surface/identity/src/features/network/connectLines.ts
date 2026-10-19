/**
 * The lines that connect Claude Code and Codex to an agent through `lys mcp` (AGENTS-016 R2), built in this one place
 * beside the Computers labels, from the agent's id, the page's origin and the key file's path. The agent file's
 * Connect section renders what these return and writes no command text of its own.
 */

/** The key file as the Connect section shows it: the person puts in the path on the computer the line runs on. */
export const KEY_FILE = '<path to this agent’s key file on that computer>';

/** The sentence above the lines. */
export const CONNECT_INTRO = 'Run one of these on the computer where Claude Code or Codex runs, to reach this agent through Lys.';

/** What a connect line is built from. */
export interface ConnectFor { id: string; origin: string; keyFile: string }

/** One line and the words its copy act names it by. */
export interface ConnectLine { what: string; line: string }

/** The bridge both lines run: `lys mcp` reaching this Lys as the agent. */
const bridge = ({ id, origin, keyFile }: ConnectFor): string => `lys mcp --url ${origin}/api/mcp --agent ${id} --key-file ${keyFile}`;

/** The Claude Code line, then the Codex line, for the agent `of` names. */
export const connectLines = (of: ConnectFor): ConnectLine[] => [
  { what: 'The Claude Code line', line: `claude mcp add --transport stdio lys -- ${bridge(of)}` },
  { what: 'The Codex line', line: `codex mcp add lys -- ${bridge(of)}` },
];
