# Haematite conversation — separate working notes

21 September 2026, 18:17 Melbourne. Tom corrected the process: talk first, capture notes separately; this need not go into the roadmap. The prematurely added roadmap section has been removed. No architecture decision settled.

Opening discussion distinguishes (1) Haematite/database-front-end consuming shared identity and permissions, with server enforcement for searches, reads and writes, from (2) Haematite becoming storage for some identity or permission records. Proposed starting principle: establish authoritative ownership and failure behaviour before combining storage. Asked Tom which aspect he meant; answer pending.

Previous proposed agenda retained below as discussion material only, not a roadmap requirement or accepted design:

## Haematite discussion before the Lys service design

Tom added this explicitly at 18:14–18:15: have the extended Haematite conversation and know what we want at every stage. Reserve a working conversation before settling checkpoint five; it can occupy the next two-hour block rather than being added on top.

Start from the actual current Haematite implementation and the storage/database front end Tom means. Decide ownership of identity mappings, credentials, permission facts, application records and audit evidence. Establish which component is authoritative for each, how changes survive interruption, and how the services discover and authenticate each other. Keeping upstream identity/permission stores initially does not settle whether Haematite later stores any of these records. Replacing their storage engines is not assumed or included without a separately sized design.

The conversation is complete when we have: one agreed user journey for the first usable stage; a record-ownership map; the interfaces each product must provide; explicit outage/revocation behaviour; and a short acceptance demonstration for every next stage. Record unresolved choices by name. Do not quietly convert a discussion proposal into a settled storage contract.


18:19 Tom clarified: Haematite as an authorization database in the role of SpiceDB, not merely a login database. Spoken proposal: Lys owns relation schema and decision evaluation, Haematite persists relationships/revisions; Rauthy handles authentication. Example team membership -> project access -> document inheritance. Investigate bounded direct grants/team/project inheritance before broad engine semantics. Need consistent point checks and authorized listings, revocation freshness, restart/concurrency and explainable decisions. No assertion that current Haematite provides the required transaction/read guarantees; implementation must be inspected before that claim. Sources read: https://authzed.com/docs/spicedb/concepts/relationships and https://authzed.com/docs/spicedb/concepts/consistency . This remains discussion, not a settled decision or roadmap amendment.

18:22–18:25 Tom asks suitability, not necessity: does content-addressed architecture help authorization or make it a poor fit? Answer: useful for immutable policy/relationship snapshots, reproducibility and historical evidence; a valid old hash does not establish current authority after revocation. Need authoritative publication/freshness plus snapshot-consistent evaluations, and retention of policy/context inputs. Current source read: api/event_store.rs:200 read_from explicitly uses two actor commands without a consistent snapshot; db/observer.rs docs say successive calls may see different committed roots; branch/snapshot.rs retains named committed roots. No blanket claim that Haematite lacks a suitable snapshot API; deeper inspection needed. Initial fit judgment: promising substrate, not already-proved authorization engine. No benchmarks/tests performed.
