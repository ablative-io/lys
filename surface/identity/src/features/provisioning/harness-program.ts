/** Legacy updater-owned Claude installs follow the command; every other program is kept exactly. */
export function launchCommand(program: string): string {
  if (!program.startsWith('/')) return program;
  const parts = program.split('/');
  const versioned = parts.some((part, index) => part === 'claude' && parts[index + 1] === 'versions' && /^[0-9]/.test(parts[index + 2] ?? ''));
  const local = parts.some((part, index) => part === '.local' && parts[index + 1] === 'share' && parts[index + 2] === 'claude');
  const file = parts.filter(Boolean).at(-1);
  return versioned || local && (file === 'claude' || file === 'claude-code') ? 'claude' : program;
}

export const PROGRAM_RULE = 'Enter a plain command name (for instance cdx), or the program’s full path starting with / and ending in a file name.';

/** An absolute path with its `.` and `..` parts worked out, as a login's PATH may name a folder
 *  (`~/.local/share/../bin`); anything else is returned unchanged. */
export function plainPath(path: string): string {
  if (!path.startsWith('/')) return path;
  const parts: string[] = [];
  for (const part of path.split('/')) {
    if (part === '' || part === '.') continue;
    if (part === '..') parts.pop(); else parts.push(part);
  }
  return '/' + parts.join('/');
}

/** The form and the launch boundary accept the same plain program shapes. */
export function commandOrAbsolutePath(program: string): boolean {
  if (!program || /[=\\\u0000-\u001f\u007f-\u009f]/.test(program)) return false;
  if (program.startsWith('/')) return program.slice(1).split('/').every((part) => part !== '' && part !== '.' && part !== '..');
  return program !== '.' && program !== '..' && !program.startsWith('-') && /^[A-Za-z0-9_.-]+$/.test(program);
}
