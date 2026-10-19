/**
 * The lines that set up an agent where it runs and connect Claude Code and Codex to it through `lys mcp`
 * (AGENTS-016 R2, AGENTS-017 R1), built in this one place beside the Computers labels, from the agent's id, the page's
 * origin and the seat file's path. The agent file's Connect section renders what these return and writes no command
 * text of its own.
 */

/** Where a seat file goes when the agent's profile names none: under the login's own `~/.lys`, one file an agent. */
export const defaultSeatFile = (id: string): string => `~/.lys/seats/${id}.seat.json`;

/**
 * Where the enrol line names the registering agent's seat file, which signs the first certificate's request
 * (AGENTS-016 R4). Lys keeps no path for it, so the line says what goes there; a person enrols with `--pass` instead.
 */
export const REGISTRAR_SEAT = '<path to the seat file of the agent that registered this one>';

/** The sentence above the lines. */
export const CONNECT_INTRO = 'On the computer where this agent will run, run the first line once: it makes the agent’s key there, has Lys certify it and writes its seat file. Put the path to the seat file of the agent that registered this one where the line has angle brackets, or, enrolling it yourself, use --pass with your Lys app pass file in place of --as. Then run one of the others to reach this agent through Lys from Claude Code or Codex.';

/** What a connect line is built from. */
export interface ConnectFor { id: string; origin: string; keyFile: string }

/** One line and the words its copy act names it by. */
export interface ConnectLine { what: string; line: string }

/** `lys agent enrol`: the key made where the agent runs, certified by this Lys, written to the seat file's path. */
const enrol = ({ id, origin, keyFile }: ConnectFor): string =>
  `lys agent enrol --url ${origin}/api --agent ${id} --as ${REGISTRAR_SEAT} --out ${keyFile}`;

/** The bridge both lines run: `lys mcp` reaching this Lys as the agent. */
const bridge = ({ id, origin, keyFile }: ConnectFor): string => `lys mcp --url ${origin}/api/mcp --agent ${id} --key-file ${keyFile}`;

/** The enrol line, then the Claude Code line, then the Codex line, for the agent `of` names. */
export const connectLines = (of: ConnectFor): ConnectLine[] => [
  { what: 'The enrol line', line: enrol(of) },
  { what: 'The Claude Code line', line: `claude mcp add --transport stdio lys -- ${bridge(of)}` },
  { what: 'The Codex line', line: `codex mcp add lys -- ${bridge(of)}` },
];
