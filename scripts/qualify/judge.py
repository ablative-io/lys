"""Parses what qual.sh observes. Never prints harness text, a credential or an environment value.

judge.py stage-a FILE ADAPTER     one evidence file from qualify_adapter (DIRECTORY-064 amendment 38)
judge.py versions FILE            `claude --version` / `codex --version` output -> "adapter version"
judge.py answer                   an agent-browser eval answer on stdin -> "STATUS" line, body to fd 3
judge.py get FILE DOTTED.PATH     one field of a JSON file, or nothing
judge.py drift A B                two versions files -> "same adapter" / "drift adapter old new"

Runs under /usr/bin/python3 3.9.
"""

import hashlib
import json
import re
import sys

# Amendment 50: Claude binds on the initialize answer and sends init only with the first turn.
ORDER = ["launcher", "version_report", "bound", "reminder_admitted", "init", "compaction", "stop",
         "terminal_writes"]
FORBIDDEN = {"text", "content", "message", "prompt", "words", "env", "environment", "token",
             "secret", "password", "credential"}


def fail(adapter, step, reason):
    print(f"FAIL {adapter} {step} {reason}")
    return 1


def stage_a(path, adapter):
    raw = open(path, "rb").read()
    digest = hashlib.sha256(raw).hexdigest()
    try:
        lines = [json.loads(line) for line in raw.decode().splitlines() if line.strip()]
    except ValueError:
        return fail(adapter, "file", "not-json-lines")
    seen = {}
    for at, entry in enumerate(lines):
        if not isinstance(entry, dict) or "step" not in entry:
            return fail(adapter, "file", f"line-{at + 1}-has-no-step")
        leaked = FORBIDDEN.intersection(entry)
        if leaked:
            return fail(adapter, entry["step"], "carries-" + ",".join(sorted(leaked)))
        if "observed" in entry and type(entry["observed"]) is not bool:
            return fail(adapter, entry["step"], "observed-not-boolean")
        if entry.get("observed") is False:
            # Amendment 50: the protocol failure comes whole, with cleanup's result apart.
            reason = " ".join(str(entry.get("reason") or "").split())
            if not reason:
                return fail(adapter, entry["step"], "not-observed:reason-empty")
            if "cleanup" in entry:
                cleanup = " ".join(str(entry.get("cleanup") or "").split())
                reason += ";cleanup:" + (cleanup or "cleanup-empty")
            return fail(adapter, entry["step"], "not-observed:" + reason)
        seen.setdefault(entry["step"], entry)
    steps = [entry["step"] for entry in lines]
    if steps != ORDER:
        missing = [step for step in ORDER if step not in seen]
        return fail(adapter, missing[0] if missing else "order", "steps-" + ",".join(steps))
    launcher = seen["launcher"]
    if launcher.get("kind") != "example-owned" or launcher.get("claims_spawn") is not False:
        return fail(adapter, "launcher", "not-example-owned-or-claims-spawn")
    if type(launcher.get("fixture")) is not bool:
        return fail(adapter, "launcher", "fixture-not-boolean")
    if not seen["bound"].get("correlation"):
        return fail(adapter, "bound", "no-correlation")
    reported, started = seen["version_report"].get("version"), seen["init"].get("version")
    if not reported or reported != started:
        return fail(adapter, "init", f"version-report-{reported}-init-{started}")
    if not seen["reminder_admitted"].get("correlation"):
        return fail(adapter, "reminder_admitted", "no-correlation")
    compaction = seen["compaction"]
    if not compaction.get("evidence") or compaction.get("turn_completed") is not True:
        return fail(adapter, "compaction", "no-evidence-or-turn-completion")
    if seen["stop"].get("exit_observed") is not True:
        return fail(adapter, "stop", "exit-not-observed")
    if seen["terminal_writes"].get("count") != 0:
        return fail(adapter, "terminal_writes", f"count-{seen['terminal_writes'].get('count')}")
    # Amendment 42: a fixture file passes as FIXTURE and is never a pin's evidence.
    print(f"{'FIXTURE' if launcher['fixture'] else 'PASS'} {adapter} {reported} {digest}")
    return 0


def versions(path):
    text = open(path).read()
    claude = re.search(r"^(\d+\.\d+\.\d+) \(Claude Code\)\s*$", text, re.M)
    codex = re.search(r"^codex-cli (\d+\.\d+\.\d+)\s*$", text, re.M)
    print("claude-code", claude.group(1) if claude else "unread")
    print("codex", codex.group(1) if codex else "unread")
    return 0


def answer():
    raw = sys.stdin.read().strip()
    value = raw
    for _ in range(3):
        if isinstance(value, dict):
            break
        try:
            value = json.loads(value)
        except (TypeError, ValueError):
            break
    if isinstance(value, dict) and "data" in value and isinstance(value["data"], dict):
        value = value["data"].get("result", value["data"])
        if isinstance(value, str):
            value = json.loads(value)
    if not isinstance(value, dict) or "status" not in value:
        print("UNREAD")
        return 1
    with open(3, "w") as out:
        out.write(value.get("body", ""))
    print(value["status"])
    return 0


def get(path, dotted):
    try:
        value = json.load(open(path))
    except (OSError, ValueError):
        return 1
    for part in dotted.split("."):
        if isinstance(value, list) and part.isdigit() and int(part) < len(value):
            value = value[int(part)]
        elif isinstance(value, dict) and part in value:
            value = value[part]
        else:
            return 1
    print(json.dumps(value) if isinstance(value, (dict, list)) else value)
    return 0


def drift(first, second):
    def read(path):
        return dict(line.split() for line in open(path) if line.strip())
    old, new = read(first), read(second)
    for adapter in ("claude-code", "codex"):
        before, after = old.get(adapter, "unread"), new.get(adapter, "unread")
        if before == after and before != "unread":
            print("same", adapter, after)
        else:
            print("drift", adapter, before, after)
    return 0


if __name__ == "__main__":
    command, rest = sys.argv[1], sys.argv[2:]
    sys.exit({"stage-a": stage_a, "versions": versions, "answer": answer, "get": get,
              "drift": drift}[command](*rest))
