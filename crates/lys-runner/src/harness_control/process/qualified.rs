//! The launched harness binaries a managed spawn admits: each pinned by a real run.

pub(super) struct Qualification {
    pub(super) adapter: &'static str,
    pub(super) version: &'static str,
}

// A wire fixture is insufficient to qualify an installed executable.
// Each entry is one real launched-binary run of tests/qualify_launched.rs (12 Oct 2026, Tom's
// Mac, the login's HOME, a scratch working folder): start proved, one answered turn, a
// turn-boundary delivery, a real compaction and a clean stop.
pub(super) const QUALIFIED: &[Qualification] = &[
    // Claude Code 2.1.296, executable sha256 c9b5341637becbd423ddffc5b254afb645682a3868cb708bbc6cc0e7bb419937.
    Qualification {
        adapter: "claude-stream-json/1",
        version: "2.1.296",
    },
    // codex-cli 0.161.0-alpha.3 (/Users/tom/.local/bin/cdx), executable sha256
    // 5bd48834daff9572d4bfb9c7e7e9cd8fb63323a545ce4a911faabc4ed5b4db7a.
    Qualification {
        adapter: "codex-app-server/1",
        version: "0.161.0-alpha.3",
    },
];
