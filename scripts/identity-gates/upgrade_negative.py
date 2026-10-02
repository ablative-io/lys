"""One deliberate team record the old release cannot read must be gone after a real put-back.

The record is appended in the reversible window, after the upgrade kept the
data it started from. The byte verifier must name it; then the put-back, by
the old installer for a release that keeps data, or by the candidate's
installer after the old one refuses the record for a release that does not,
must return the team log byte for byte, without the record, and the old
build must start on it.

A release whose team answer has no `held` (`unguarded`) cannot read the
candidate's held line, so the record is a real candidate-format hold. A
`guarded` release already reads every team line the candidate writes, so no
candidate record is unreadable to it; the record is then a line kind no
release defines, and the receipt says so rather than claiming a format gap.
"""

import json
from pathlib import Path
import subprocess

from upgrade_window import legacy_files, pending, unchanged


UNDEFINED_LINE = "upgrade_proof_undefined"


def poison_line(teams):
    """The team line kind the old release (`unguarded` or `guarded`) cannot read."""
    if teams == "unguarded":
        return "held"
    if teams == "guarded":
        return UNDEFINED_LINE
    raise RuntimeError(f"no negative control for a release whose teams are {teams!r}")


def refuse_old_put_back(installed, log, env, root):
    """An old installer that keeps no data must refuse the record by name and touch nothing.

    It cannot put back the data the candidate wrote, which its own build cannot
    read, so it must refuse the record on the step it does not know before it
    stops or moves anything: the record and the kept binaries stand for the
    candidate's installer, which puts the kept data back, to finish.
    """
    with log.open("wb") as output:
        completed = subprocess.run(installed, stdout=output, stderr=subprocess.STDOUT, env=env)
    if completed.returncode == 0:
        raise RuntimeError(f"old installer reported a put-back it cannot make; evidence: {log}")
    if "unknown variant `data_kept`" not in log.read_text(errors="replace"):
        raise RuntimeError(f"old installer did not refuse the record by its kept-data step; evidence: {log}")
    if not (root / "install/upgrade.json").is_file():
        raise RuntimeError(f"old installer's refusal removed the upgrade record; evidence: {log}")
    if not (root / "bin.previous").is_dir():
        raise RuntimeError(f"old installer moved the kept binaries before refusing; evidence: {log}")


def put_back(root, evidence, driver, installed, env, run, release_put_back, prefix):
    """The put-back each release can make: the old installer's own, or, for a release
    that keeps no data, the candidate's after the old installer refuses the record."""
    if release_put_back == "keeps_data":
        run(installed, evidence / f"{prefix}recover-old.log", env)
        return "old"
    if release_put_back != "no_data":
        raise RuntimeError(f"no put-back for a release whose put-back is {release_put_back!r}")
    refuse_old_put_back(installed, evidence / f"{prefix}recover-old.log", env, root)
    run([str(driver), "--root", str(root), "recover"], evidence / f"{prefix}recover-after-old.log", env)
    return "candidate"


def team_log_restored(root, files, current, record):
    """Every legacy file of the record's team log reads back as before, and the record is gone."""
    if record.exists():
        raise RuntimeError(f"put-back left the record the old build cannot read: {record}")
    log = str(record.parent.parent.relative_to(root)) + "/"
    team = {path for path in files.keys() | current.keys() if path.startswith(log)}
    if not team:
        raise RuntimeError(f"no legacy file of the team log {log} to compare")
    for path in sorted(team):
        if files.get(path) != current.get(path):
            raise RuntimeError(f"put-back did not return team log file {path}")


def exercise(root, evidence, driver, installed, env, files, run, stamp, old_commit, teams,
             release_put_back):
    """All mutations are confined to the separately marked negative fixture."""
    line = poison_line(teams)
    pending(root)
    run([str(driver), "--root", str(root), "poison", "--line", line, "--fixture",
         str(evidence / "legacy-before.json")], evidence / "negative-write.log", env)
    prefix = "negative control new-format record: "
    paths = [line[len(prefix):] for line in (evidence / "negative-write.log").read_text().splitlines()
             if line.startswith(prefix)]
    if len(paths) != 1:
        raise RuntimeError("negative control did not name exactly one new record")
    record = Path(paths[0]).resolve()
    if not record.is_relative_to(root) or not record.is_file():
        raise RuntimeError(f"negative control record leaves its fixture or is absent: {record}")
    if json.loads(record.read_text()).get("line") != line:
        raise RuntimeError(f"negative control did not write a {line} line: {record}")
    config = json.loads((root / "identity.json").read_text())
    current = legacy_files(root, config)
    name = str(record.relative_to(root))
    if name in files or name not in current:
        raise RuntimeError(f"negative record is not a newly appended leaf: {record}")
    try:
        unchanged(files, current)
    except RuntimeError as error:
        if name not in str(error):
            raise RuntimeError(f"negative byte check did not name {record}: {error}") from error
        (evidence / "negative-byte-refusal.txt").write_text(str(error) + "\n")
    else:
        raise RuntimeError(f"byte verifier accepted the deliberately incompatible record {record}")
    who = put_back(root, evidence, driver, installed, env, run, release_put_back, "negative-")
    stamp(root / "bin", old_commit)
    after = legacy_files(root, json.loads((root / "identity.json").read_text()))
    team_log_restored(root, files, after, record)
    return {"passed": True, "negative_control": True, "record": str(record),
            "old_binary": old_commit, "put_back_by": who, "record_removed": True,
            "team_log_restored": True, "byte_verifier_refused": True,
            "line": line, "candidate_format": line == "held"}
