# ACCESS-005 R2: the install walk, with pictures

Owed for the compile pass: written 9 October 2026 with the code, not yet walked. The brief's acceptance for R2 is
"Walked with pictures on the install: a machine with one link grant." Walk it once, after the battery is green and the
install is upgraded to the commit that carries ACCESS-005, in a browser signed in as the administrator. Save each
picture under the walk's own dated folder and name it by its step number.

## Before

- The install serves Lys at an https address another computer can reach (a connection code is refused otherwise).
- A second computer with `lys` installed, or a scratch user account standing in for one.
- The link kind exists: an approved app whose schema declares a link kind (for liminal, `liminal.link`) with an
  action set that includes export and import. Without it, use any kind the administrator holds a root on and say so.

## Steps

1. Network, no machine yet. Open Network and choose the computer. Picture: the panel's "As a machine" section reads
   "This computer has not joined with a connection code".
2. Give a code. In the panel, give a connection code; run the shown command on the other computer and paste the code.
   Picture: the panel after the runner connects (status Up).
3. The machine. Picture: the "As a machine" card: its `machine-` id, the key id (the 64 hex digits the other computer
   joined with; check them against `~/.lys/runner` or the key file there), "Answers to" naming the administrator,
   Stands "Active", and "It holds no grant."
4. A root that may reach machines. In Access, issue the administrator a root on the link (relation carrying export
   and import) whose pass-on names `machine` (through the API if the root form does not yet offer `machine`:
   `POST /grants/roots` with `"recipients": ["machine"]`). Picture: the root in Access.
5. Give the machine one link grant. Back on Network, "Give a grant" on the machine card; choose the root and the
   relation; Give. Picture: the notice "Given. machine-… now holds … on link …" and the card's grants table with one
   row, whose "On" link opens Access on the link (picture that page too).
6. Read it back. `GET /network/machine-identities` as the administrator answers the same grant with
   `"admitted": true`. Picture or paste the answer.
7. A kept act is refused. Try a root naming `machine` in its pass-on with a relation carrying `grant.delegate` on a
   Lys kind. Picture: the refusal `MachineRefused`.
8. A second join replaces the first. Give the same computer a new code and join again with a new key. Picture: two
   cards, the new one first and Active, the old one "Replaced by a later join", its grant no longer counting.

## Not walked here

- "In its pass" (R1 acceptance): no pass issued to a machine carries a rights claim yet; that waits on the
  provider/issuer work, so this walk does not picture it.
