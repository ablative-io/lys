/**
 * The Computers page's labels, defined once (AGENTS-016 R1). The rail and the Connect group read them from here, and
 * `lys` reads this file when it is built (crates/lys/src/commands/launch_device_tests.rs), so advice printed on a
 * computer names only what these screens show; a renamed label fails that test.
 */

/** The rail's name for the Computers page. */
export const COMPUTERS = 'Computers';

/** The start of the Connect group's name for one computer. */
export const CONNECT = 'Connect ';

/** The Connect group's name for the computer called `name`. */
export const connectLabel = (name: string): string => `${CONNECT}${name}`;
