"""Build shipped packages without borrowing another package's selected features."""

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path


PACKAGES = ("lys", "lys-identity-server", "lys-secrets", "lys-runner")


def checkout_identity(repository):
    values = []
    for arguments in (("rev-parse", "HEAD"), ("rev-parse", "HEAD^{tree}"),
                      ("status", "--porcelain=v1", "--untracked-files=normal")):
        command = ["git", "--no-optional-locks", *arguments]
        try:
            completed = subprocess.run(command, cwd=repository, text=True,
                                       capture_output=True, check=False)
        except OSError as error:
            raise RuntimeError(f"release_isolation_git_unavailable: {error}") from error
        if completed.returncode != 0:
            raise RuntimeError(
                f"release_isolation_git_failed: {arguments[0]} exited "
                f"{completed.returncode}: {completed.stderr.strip()}"
            )
        values.append(completed.stdout.strip())
    if values[2]:
        raise RuntimeError(f"release_isolation_dirty_checkout: {values[2]}")
    if any(re.fullmatch(r"[0-9a-f]{40}", value) is None for value in values[:2]):
        raise RuntimeError("release_isolation_invalid_identity: Git did not return a commit and tree")
    return values[0], values[1]


def build_commands():
    return [["cargo", "build", "--locked", "--release",
             *[argument for package in PACKAGES for argument in ("-p", package)]]]


def prove(repository):
    result = {"commit": None, "tree": None, "packages": list(PACKAGES),
              "builds": [], "source_refusal": None, "passed": False,
              "counts": {"expected": len(PACKAGES), "executed": 0,
                         "passed": 0, "failed": 0, "not_started": len(PACKAGES)}}
    try:
        result["commit"], result["tree"] = checkout_identity(repository)
    except RuntimeError as error:
        result["source_refusal"] = str(error)
        return result
    for command in build_commands():
        package = command[-1]
        row = {"package": package, "command": command, "exit_code": None,
               "refusal": None, "elapsed_seconds": None}
        started = time.perf_counter()
        try:
            completed = subprocess.run(command, cwd=repository, check=False)
            row["exit_code"] = completed.returncode
            if completed.returncode != 0:
                row["refusal"] = (
                    f"release_isolation_build_failed: {package} exited {completed.returncode}"
                )
        except OSError as error:
            row["refusal"] = f"release_isolation_build_unavailable: {package}: {error}"
        row["elapsed_seconds"] = time.perf_counter() - started
        result["builds"].append(row)
    try:
        commit, tree = checkout_identity(repository)
        if (commit, tree) != (result["commit"], result["tree"]):
            result["source_refusal"] = (
                f"release_isolation_source_changed: {result['commit']}/{result['tree']} "
                f"became {commit}/{tree}"
            )
    except RuntimeError as error:
        result["source_refusal"] = str(error)
    rows = result["builds"]
    result["counts"] = {
        "expected": len(PACKAGES),
        "executed": sum(row["exit_code"] is not None for row in rows),
        "passed": sum(row["exit_code"] == 0 for row in rows),
        "failed": sum(row["exit_code"] not in (None, 0) for row in rows),
        "not_started": sum(row["exit_code"] is None for row in rows),
    }
    result["passed"] = (result["source_refusal"] is None
                        and len(rows) == len(PACKAGES)
                        and all(row["exit_code"] == 0 for row in rows))
    return result


def main(arguments=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    options = parser.parse_args(arguments)
    repository = Path(__file__).resolve().parents[2]
    report = options.report.resolve()
    if report.is_relative_to(repository):
        print(f"release_isolation_report_inside_checkout: {report}", file=sys.stderr)
        return 1
    try:
        with report.open("x", encoding="utf-8") as output:
            result = prove(repository)
            json.dump(result, output, indent=2)
            output.write("\n")
    except OSError as error:
        print(f"release_isolation_report_unavailable: {report}: {error}", file=sys.stderr)
        return 1
    for row in result["builds"]:
        if row["refusal"]:
            print(row["refusal"], file=sys.stderr)
    if result["source_refusal"]:
        print(result["source_refusal"], file=sys.stderr)
    print(json.dumps({"commit": result["commit"], "tree": result["tree"],
                      "counts": result["counts"], "passed": result["passed"],
                      "report": str(report)}))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
