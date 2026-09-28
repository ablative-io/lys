# Lys-Gate — User Stories

## Lead — Reads a red gate round before landing a card

**S1.** As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

**S4.** As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.

## AI Agent — Runs the gates by hand before a commit

**S2.** As an agent running the gates by hand from CLAUDE.md, I want its test line to be the landing hook's test command so that my run measures what landing measures.

**S5.** As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after.

## Contributor — Reads CI results on a pull request

**S3.** As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.
