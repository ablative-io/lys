/** A rule is written from a kind and one target, in the grammar Claude Code reads, and read back in plain words. */
import { describe, expect, it } from 'vitest';
import { ruleFor, summary, wordsFor } from '../src/features/provisioning/permission-rules';

describe('permission rules', () => {
  it('writes a folder with two leading slashes, so it means the top of the disk and not beside the settings file', () => {
    expect(ruleFor('read-folder', '/srv/site')).toEqual({ rule: 'Read(//srv/site/**)' });
    expect(ruleFor('change-folder', '/srv/site/')).toEqual({ rule: 'Edit(//srv/site/**)' });
    expect(ruleFor('read-folder', '/')).toEqual({ rule: 'Read(//**)' });
  });

  it('writes a command, a website and a whole tool', () => {
    expect(ruleFor('command', ' git push ')).toEqual({ rule: 'Bash(git push *)' });
    expect(ruleFor('command', 'git push *')).toEqual({ rule: 'Bash(git push *)' });
    expect(ruleFor('website', 'Example.com.')).toEqual({ rule: 'WebFetch(domain:example.com)' });
    expect(ruleFor('website', '*.example.com')).toEqual({ rule: 'WebFetch(domain:*.example.com)' });
    expect(ruleFor('tool', 'WebSearch')).toEqual({ rule: 'WebSearch' });
  });

  it('says why a target cannot be written, and writes nothing', () => {
    expect(ruleFor('read-folder', '')).toEqual({ problem: 'Choose a folder.' });
    expect(ruleFor('read-folder', 'srv/site')).toEqual({ problem: 'Choose a folder from the computer’s own folders.' });
    expect(ruleFor('change-folder', '/srv/[2024] notes')).toHaveProperty('problem');
    expect(ruleFor('command', 'echo (hi)')).toEqual({ problem: 'A rule cannot hold a round bracket. Leave the brackets out.' });
    expect(ruleFor('website', 'https://example.com/a')).toHaveProperty('problem');
    expect(ruleFor('website', 'localhost')).toHaveProperty('problem');
    expect(ruleFor('tool', 'Stop Task')).toHaveProperty('problem');
    expect(ruleFor('command', 'ls\n-la')).toEqual({ problem: 'That holds a character that cannot be written in a rule.' });
  });

  it('reads every rule it writes back in plain words', () => {
    expect(wordsFor('Read(//srv/site/**)')).toBe('Read files under /srv/site');
    expect(wordsFor('Edit(//srv/site/**)')).toBe('Change files under /srv/site');
    expect(wordsFor('Read(///**)')).toBe('Read files under /');
    expect(wordsFor('Read(//**)')).toBe('Read files under /');
    expect(wordsFor('Bash(git push *)')).toBe('Run commands that start with git push');
    expect(wordsFor('Bash(npm run build)')).toBe('Run the command npm run build');
    expect(wordsFor('WebFetch(domain:example.com)')).toBe('Reach example.com');
    expect(wordsFor('Read')).toBe('Read any file');
    expect(wordsFor('Bash')).toBe('Run any command');
    expect(wordsFor('Glob')).toBe('Use the tool Glob');
    expect(wordsFor('mcp__puppeteer')).toBe('Use the connected tools named puppeteer');
  });

  it('shows a rule it did not write and cannot read as itself, never as something else', () => {
    expect(wordsFor('Read(./.env)')).toBe('The rule Read(./.env)');
    expect(wordsFor('Bash(git * main)')).toBe('The rule Bash(git * main)');
    expect(wordsFor('Read(//)')).toBe('The rule Read(//)');
    expect(wordsFor('Agent(Explore')).toBe('The rule Agent(Explore');
  });

  it('sums the rules in one line', () => {
    expect(summary(null)).toBe('No extra rules.');
    expect(summary({ allow: [], deny: [], ask: [], additional_directories: [] })).toBe('No extra rules.');
    expect(summary({ deny: ['Read', 'Write'], ask: ['Bash(git push *)'], allow: ['WebSearch'], additional_directories: ['/srv/kept'] })).toBe('4 rules: 2 refused, 1 asks first, 1 without asking. 1 extra folder.');
    expect(summary({ ask: ['Bash', 'WebFetch'] })).toBe('2 rules: 2 ask first.');
    expect(summary({ additional_directories: ['/a', '/b'] })).toBe('No extra rules. 2 extra folders.');
  });
});
