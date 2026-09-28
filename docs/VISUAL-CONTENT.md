# Visual content plan

Placement suggestions for terminal recordings and diagrams that would
strengthen Lys's documentation. The README's worked examples are
text-heavy by design (reproducible shell sessions), so visual content
here is supplementary -- recordings that show the flow at speed, and
diagrams that show the trust relationships at a glance.

## README

| Location | Content | Format |
|---|---|---|
| After "What exists today" | Architecture diagram: the five primitives (identity, capability certs, attestations, sealed transport, transparency logs) and how they compose | SVG or Excalidraw export |
| After Example 1 | Terminal recording: the full audit-log walkthrough -- key generate, log init, three appends, checkpoint, prove inclusion, verify inclusion, tamper-and-fail | asciinema or GIF, ~45s |
| After Example 2 | Terminal recording: the agent trust walkthrough -- orchestrator key, ca issue, ca verify, agent key, seal, open, open-wrong-sender-fail | asciinema or GIF, ~40s |
| After "Verify without lys" | Terminal recording: reproducing a leaf hash with coreutils only (the `printf '\x00' | shasum` trick) | asciinema or GIF, ~10s |

## docs/DESIGN.md

| Location | Content | Format |
|---|---|---|
| Top | Primitive relationship diagram: Identity -> CA -> Attestation -> Log, with Sealed Transport as a cross-cutting channel | SVG |
| Wire format sections | Byte-layout diagrams for COSE_Sign1 attestations and C2SP signed-note checkpoints | SVG, monospace font |

## docs/VISION.md

| Location | Content | Format |
|---|---|---|
| "The accountability gap" section | Before/after diagram: trust-on-vibes (screenshot, chat log) vs structural trust (signed attestation, Merkle proof, capability cert) | SVG or Excalidraw export |

## docs/design/WIRE-FORMATS.md

| Location | Content | Format |
|---|---|---|
| Each format section | Annotated hex dump of a real artifact with field boundaries marked | PNG or SVG, monospace |
| Merkle tree section | Tree diagram showing RFC 6962 leaf hashing and interior node construction for a 3-leaf example matching Example 1 | SVG |

## Video walkthrough ideas

| Topic | Duration | Audience |
|---|---|---|
| "Zero to tamper-evident log" -- install from source, run Example 1 end to end | 5 min | Developers evaluating Lys |
| "Agent trust infrastructure" -- run Example 2, explain each primitive's role | 7 min | AI/ML engineers building agent systems |
| "Verify without Lys" -- reproduce every verification step with standard tools (shasum, Go sumdb/note, go-cose) | 5 min | Security engineers, auditors |
| "Why boring cryptography" -- the design philosophy, with live interop demos against Go and Cloudflare implementations | 8 min | Cryptography-aware developers |

## Tools

- **Terminal recordings**: [asciinema](https://asciinema.org) (renders as text, accessible) or [VHS](https://github.com/charmbracelet/vhs) (GIF/MP4 from a script)
- **Architecture diagrams**: hand-drawn SVG or [Excalidraw](https://excalidraw.com) for the sketch aesthetic
- **Hex dumps**: captured from `xxd` output, annotated in a vector editor
- **Screenshots**: macOS with a clean terminal (ghostty or iTerm2, dark theme matching the Ablative brand)
