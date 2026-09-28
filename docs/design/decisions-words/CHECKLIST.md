# Decisions-Words — Checklist

## Quotes

- [ ] **C1** — The quote of each of ADR-001 to ADR-008 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
- [ ] **C2** — The quote of ADR-011 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
- [ ] **C3** — No quote value held by docs/design/decisions.json at 7b53625 for ADR-001 to ADR-008 or ADR-011 appears anywhere in the file.

## Personal content

- [ ] **C4** — No context, decision or consequence of ADR-005 to ADR-008 contains its row's decided_by value.
- [ ] **C5** — No context, decision or consequence of ADR-005 to ADR-008 contains a time of day.
- [ ] **C6** — ADR-005's context states the need the operator raised in place of what a person said or hoped.
- [ ] **C7** — ADR-005's decision and its second consequence name the operator's workstation in place of a personal machine.

## Ledger integrity

- [ ] **C8** — Every field of every decision that C1 to C7 do not name is byte-identical to its value at 7b53625.
- [ ] **C9** — docs/design/decisions.json holds 18 decisions, ADR-001 to ADR-018 in order.
- [ ] **C10** — The ledger's updated field is the date of the commit that makes the change.
- [ ] **C11** — docs/design/decisions.json keeps the serialisation of json.dump(indent=2, ensure_ascii=False) plus a trailing newline.
- [ ] **C12** — scripts/design/schemas/decisions.schema.json is unchanged from 7b53625.
- [ ] **C13** — sh scripts/design/gate.sh exits 0.

## Rendered documents

- [ ] **C14** — Every committed markdown file that renders ADR-005's decision is the output of scripts/design/render-cluster.py over the changed ledger.
