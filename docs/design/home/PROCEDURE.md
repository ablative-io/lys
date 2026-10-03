# Giving out work and handing it back

Waffles, 3 October 2026, from Tom's words on Dot 19:09 to 19:12: "a general procedure, not try to code it
into anything... ten lines maybe... here's what your handback should include... here's what your briefing
request should look like... what needs a user story, what doesn't... maybe we have tiers." Not enforced in
code. It is what worked today, written down so it is not rediscovered. RULES.md is the longer think.

## Giving out work (the request)

1. What, in one sentence a stranger could check.
2. Where: the repository, main, the files or the crate. Never a folder outside the checkout.
3. By when: a time box of thirty minutes, ending in a sha or a written stop that names what is left.
4. The checklist: each requirement on its own line, lettered, each one checkable by reading or by a test.
5. The default lines, on every checklist: written before anything is built; one act per commit, pushed;
   no test proves less than before; files under the length limit; the hospital line on the handback.
6. Who reads it, named, and that the reader is not the writer.
7. What is out of scope, if anything nearby is tempting.

## Handing it back (the handback)

1. "Written", the final sha, and that nothing is uncommitted.
2. Each checklist letter answered: the sha and file that meets it, or "not done" and why.
3. What was read and not run, said plainly ("not compiled"), so nobody mistakes a read for a pass.
4. Anything found that was not on the list, named, not fixed in passing.
5. Where the test was wrong and where the code was wrong, said which; a test is changed only with the
   reason it proved the wrong thing.
6. The hospital line: the worst credible failure and about how many people.

## Tiers: what needs what

- **A fix named by a gate** (a lint, a red test, a length overrun): the request is the gate's own line; the
  handback is the sha and one sentence. No brief, no story.
- **A slice of an existing brief**: the request points at the brief's requirement numbers; the handback
  answers each. No new story.
- **New behaviour a person will meet**: a brief in the repository's design folder with numbered
  requirements, and a user story for each thing a person does or sees. Written and read before any code.
- **A change to how data is stored or who is trusted**: the brief, the stories, and a design page first;
  Tom sees the design page before it is built.

## Keeping it small

- A seat's CLAUDE.md stays under 200 lines; what does not fit is not important enough or belongs in a
  design page. Design pages live in the repository they are about.
- A seat silent for ten minutes inside a box is measured twice, then its session is read for a rate limit
  before anyone is chased.
- The lead reports to Tom at outcomes: the install, a true block with the workaround already moving, or a
  changed time.
