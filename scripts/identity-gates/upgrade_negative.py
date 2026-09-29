"""One deliberate new-format write must make a real old-binary rollback fail."""

import json
from pathlib import Path
import subprocess

from upgrade_window import legacy_files, pending, unchanged


def require_old_refusal(text, record):
    index = int(record.name)
    if f"leaf {index} is not a team line" not in text or "held" not in text:
        raise RuntimeError(f"old binary did not refuse the injected team record {record}")


def exercise(root, evidence, driver, installed, env, files, run, stamp, old_commit):
    """All mutations are confined to the separately marked negative fixture."""
    pending(root)
    run([str(driver), "--root", str(root), "poison", "--fixture",
         str(evidence / "legacy-before.json")], evidence / "negative-write.log", env)
    prefix = "negative control new-format record: "
    paths = [line[len(prefix):] for line in (evidence / "negative-write.log").read_text().splitlines()
             if line.startswith(prefix)]
    if len(paths) != 1:
        raise RuntimeError("negative control did not name exactly one new record")
    record = Path(paths[0]).resolve()
    if not record.is_relative_to(root) or not record.is_file():
        raise RuntimeError(f"negative control record leaves its fixture or is absent: {record}")
    if json.loads(record.read_text()).get("line") != "held":
        raise RuntimeError(f"negative control did not write a held line: {record}")
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
    identity_log = root / "logs/identity.log"
    offset = identity_log.stat().st_size
    recovery = evidence / "negative-recovery.log"
    with recovery.open("wb") as output:
        completed = subprocess.run(installed, stdout=output, stderr=subprocess.STDOUT, env=env)
    if completed.returncode == 0:
        raise RuntimeError(f"old installer accepted the incompatible team record {record}")
    # A generic startup failure is insufficient: the old decoder must name this leaf.
    stamp(root / "bin", old_commit)
    with identity_log.open("rb") as source:
        source.seek(offset)
        refusal = source.read().decode()
    require_old_refusal(refusal, record)
    return {"passed": True, "negative_control": True, "record": str(record),
            "old_binary": old_commit, "old_recovery_exit": completed.returncode,
            "named_leaf_refusal": True, "byte_verifier_refused": True}
