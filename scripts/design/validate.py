#!/usr/bin/env python3
"""Validate design-system JSON documents against their schemas.

Usage:
  validate.py <path>

<path> is either a single JSON document or a directory. Directories are
searched recursively and documents are discovered by name.

The schema subset includes oneOf, which a gate entry needs: an entry is
either the three-line shorthand or the named legs a repository declares,
and exactly one of those shapes must match.

Documents are discovered by name:

  roadmap.json    -> roadmap.schema.json
  decisions.json  -> decisions.schema.json
  project.json    -> project.schema.json
  design.json     -> design.schema.json
  checklist.json  -> checklist.schema.json
  stories.json    -> stories.schema.json
  briefs/<ID>.json -> brief.schema.json (not <ID>-input.json run records)

Schemas are resolved relative to this script's own location
(../schemas/). The validator implements exactly the subset the v2
schemas use: type (object/array/string/boolean/integer), required,
additionalProperties:false, properties, items, enum, pattern, minItems,
oneOf.

Every violation is reported with a JSON-pointer-style path
(e.g. items/3/provenance/quote: missing required field). Exits 1 if any
violation is found. A directory containing no known documents is
reported but is not an error.

Beside the schema subset, one rule the schemas cannot state: no document
names a machine. A design's gate trees and its structure and inventory
paths, and a brief's requirement file paths, are read on whatever machine
the document is fired on, so an absolute path is one machine's and the
same document cannot run on the next. A path inside the repository is
spelled relative to the repository root; a path outside it is spelled
with a root token ($NAME/...) the ledger resolves from its launch
settings and refuses by name when the launch set none. A root-token path
is accepted here as written and never resolved: the validator runs on
machines no root of that document is set on. The exception is a brief
that already carries an execution record: what a round measured is kept
as written, absolute paths and all.
"""

import argparse
import json
import re
import sys
from pathlib import Path

SCHEMA_DIR = Path(__file__).resolve().parent / "schemas"

DOCUMENT_SCHEMAS = {
    "roadmap.json": "roadmap.schema.json",
    "decisions.json": "decisions.schema.json",
    "project.json": "project.schema.json",
    "design.json": "design.schema.json",
    "checklist.json": "checklist.schema.json",
    "stories.json": "stories.schema.json",
}
BRIEF_SCHEMA = "brief.schema.json"

# A run record kept beside the brief it fired (`WA-001-input.json`) is
# not a brief: the brief is `briefs/<ID>.json` and nothing else in that
# directory is read against the brief schema.
BRIEF_SIDECAR_SUFFIXES = ("-input.json", "-dispatch.json")


def is_brief_document(path: Path) -> bool:
    """`briefs/<ID>.json`, and not a run record parked beside it."""
    return (
        path.parent.name == "briefs"
        and path.suffix == ".json"
        and not path.name.endswith(BRIEF_SIDECAR_SUFFIXES)
    )

# What a path is spelled with when it leaves the repository, named in
# the refusal so an author reads the fix and not only the fault.
ROOT_TOKEN_SHAPE = "$NAME/"

TYPE_CHECKS = {
    "object": lambda v: isinstance(v, dict),
    "array": lambda v: isinstance(v, list),
    "string": lambda v: isinstance(v, str),
    "boolean": lambda v: isinstance(v, bool),
    "integer": lambda v: isinstance(v, int) and not isinstance(v, bool),
}


def type_name(value) -> str:
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, dict):
        return "object"
    if isinstance(value, list):
        return "array"
    if isinstance(value, str):
        return "string"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if value is None:
        return "null"
    return type(value).__name__


def join(path: str, key) -> str:
    return f"{path}/{key}" if path else str(key)


def loc(path: str) -> str:
    return path if path else "(root)"


def validate(value, schema: dict, path: str, errors: list[str]) -> None:
    """Validate value against the schema subset, appending violations."""
    expected = schema.get("type")
    if expected is not None:
        check = TYPE_CHECKS.get(expected)
        if check is None:
            errors.append(f"{loc(path)}: schema uses unsupported type '{expected}'")
            return
        if not check(value):
            errors.append(f"{loc(path)}: expected {expected}, got {type_name(value)}")
            return

    alternatives = schema.get("oneOf")
    if alternatives is not None:
        validate_one_of(value, alternatives, path, errors)
        return

    if "enum" in schema and value not in schema["enum"]:
        allowed = ", ".join(json.dumps(v) for v in schema["enum"])
        errors.append(f"{loc(path)}: value {json.dumps(value)} not one of [{allowed}]")

    if "pattern" in schema and isinstance(value, str):
        if not re.search(schema["pattern"], value):
            errors.append(
                f"{loc(path)}: value {json.dumps(value)} does not match "
                f"pattern {schema['pattern']}"
            )

    if isinstance(value, dict):
        for req_key in schema.get("required", []):
            if req_key not in value:
                errors.append(f"{join(path, req_key)}: missing required field")
        props = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in props:
                    errors.append(
                        f"{join(path, key)}: unexpected field "
                        "(additionalProperties is false)"
                    )
        for key, sub_schema in props.items():
            if key in value:
                validate(value[key], sub_schema, join(path, key), errors)

    if isinstance(value, list):
        min_items = schema.get("minItems")
        if min_items is not None and len(value) < min_items:
            errors.append(
                f"{loc(path)}: array has {len(value)} item(s), "
                f"minimum is {min_items}"
            )
        item_schema = schema.get("items")
        if item_schema is not None:
            for index, element in enumerate(value):
                validate(element, item_schema, join(path, index), errors)


def validate_one_of(value, alternatives: list, path: str, errors: list[str]) -> None:
    """Validate a value that may take one of several shapes.

    A gate entry is written either as the three-line shorthand or as the
    named legs the repository declares. The shapes are exclusive, so
    exactly one must match. When none matches, every shape's own
    violations are reported under its name: a reader who left a field out
    of the shorthand is told which field, and is not told only that the
    entry "matched nothing".
    """
    matched = []
    refused = []
    for index, alternative in enumerate(alternatives):
        found: list[str] = []
        validate(value, alternative, path, found)
        title = alternative.get("title", f"shape {index}")
        if found:
            refused.append((title, found))
        else:
            matched.append(title)
    if len(matched) == 1:
        return
    if matched:
        errors.append(
            f"{loc(path)}: matches {len(matched)} of the shapes this "
            f"field may take ({', '.join(matched)}); exactly one is valid"
        )
        return
    errors.append(
        f"{loc(path)}: matches none of the {len(alternatives)} shapes "
        "this field may take"
    )
    for title, found in refused:
        for message in found:
            errors.append(f"as {title}: {message}")


def machine_path(pointer: str, value) -> list[str]:
    """The violation an absolute path is, or nothing.

    A path under a root token is accepted as spelled; the validator never
    resolves one, because it runs on machines that root is not set on.
    """
    if not isinstance(value, str) or not value.startswith("/"):
        return []
    return [
        f"{pointer}: {value} is an absolute path, which is one machine's "
        "— a path inside the repository is spelled relative to the "
        "repository root, and a path outside it with a root token "
        f"({ROOT_TOKEN_SHAPE}) the ledger resolves from its launch settings"
    ]


def machine_paths(document, schema_name: str) -> list[str]:
    """Every path this document spells absolutely, as violations.

    A landed brief is exempt: its requirement paths are part of the record
    a round measured and are kept as written, so a document carrying an
    execution block is read for its shape alone.
    """
    if not isinstance(document, dict):
        return []
    errors: list[str] = []
    if schema_name == DOCUMENT_SCHEMAS["design.json"]:
        for index, entry in enumerate(document.get("gate", []) or []):
            if isinstance(entry, dict):
                errors.extend(machine_path(f"gate/{index}/tree", entry.get("tree")))
        for field in ("structure", "inventory"):
            for index, entry in enumerate(document.get(field, []) or []):
                if isinstance(entry, dict):
                    errors.extend(machine_path(f"{field}/{index}/path", entry.get("path")))
    if schema_name == BRIEF_SCHEMA and not document.get("execution"):
        for index, req in enumerate(document.get("requirements", []) or []):
            files = req.get("files", {}) if isinstance(req, dict) else {}
            for change in ("create", "modify", "delete"):
                for spot, value in enumerate(files.get(change, []) or []):
                    errors.extend(
                        machine_path(
                            f"requirements/{index}/files/{change}/{spot}",
                            value,
                        )
                    )
    return errors


def schema_name_for(doc_path: Path) -> str | None:
    """Map a document path to its schema file name, or None if unknown."""
    if is_brief_document(doc_path):
        return BRIEF_SCHEMA
    return DOCUMENT_SCHEMAS.get(doc_path.name)


def v2_clusters(root: Path) -> set[str] | None:
    """Cluster names registered in the roadmap ledger, or None when the
    directory has no roadmap.json.

    The roadmap is the v2 cluster registry: a cluster earns `designed`
    status by being linked from a roadmap item, so any cluster directory
    NOT linked is either v1-era history or an orphan — neither validates
    against the v2 schemas. Explicit file or cluster-dir arguments still
    validate anything directly.
    """
    ledger = root / "roadmap.json"
    if not ledger.is_file():
        return None
    try:
        with open(ledger) as handle:
            items = json.load(handle).get("items", [])
    except (OSError, json.JSONDecodeError):
        return None
    return {
        item["links"]["cluster"]
        for item in items
        if isinstance(item, dict)
        and isinstance(item.get("links"), dict)
        and item["links"].get("cluster")
    }


def discover(root: Path) -> tuple[list[tuple[Path, str]], list[str]]:
    """Find (document, schema-name) pairs under a directory.

    Returns the documents to validate and the names of legacy cluster
    directories that were skipped (present on disk, absent from the
    roadmap registry).
    """
    registered = v2_clusters(root)
    found = []
    skipped: set[str] = set()
    for candidate in sorted(root.rglob("*.json")):
        schema_name = schema_name_for(candidate)
        if schema_name is None:
            continue
        relative = candidate.relative_to(root)
        if registered is not None and len(relative.parts) > 1:
            cluster = relative.parts[0]
            if cluster not in registered:
                skipped.add(cluster)
                continue
        found.append((candidate, schema_name))
    return found, sorted(skipped)


def load_schema(name: str, cache: dict) -> dict:
    if name not in cache:
        schema_path = SCHEMA_DIR / name
        with open(schema_path) as handle:
            cache[name] = json.load(handle)
    return cache[name]


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate design-system JSON documents against their schemas."
    )
    parser.add_argument(
        "path",
        help="A JSON document, or a directory to search for documents "
        "(roadmap.json, decisions.json, project.json, design.json, "
        "checklist.json, stories.json, briefs/*.json).",
    )
    args = parser.parse_args()

    target = Path(args.path)
    if not target.exists():
        print(f"error: {target} does not exist", file=sys.stderr)
        sys.exit(1)

    if target.is_file():
        schema_name = schema_name_for(target)
        if schema_name is None:
            print(
                f"error: no schema known for {target.name} — expected one of "
                f"{', '.join(DOCUMENT_SCHEMAS)} or a file under briefs/",
                file=sys.stderr,
            )
            sys.exit(1)
        documents = [(target, schema_name)]
        skipped_clusters: list[str] = []
    else:
        documents, skipped_clusters = discover(target)
        if not documents:
            print(f"No documents found under {target} (nothing to validate).")
            sys.exit(0)

    schema_cache: dict[str, dict] = {}
    total_violations = 0

    for doc_path, schema_name in documents:
        try:
            schema = load_schema(schema_name, schema_cache)
        except (OSError, json.JSONDecodeError) as exc:
            print(f"error: cannot load schema {schema_name}: {exc}", file=sys.stderr)
            sys.exit(1)

        errors: list[str] = []
        try:
            with open(doc_path) as handle:
                document = json.load(handle)
        except json.JSONDecodeError as exc:
            errors.append(f"(file): invalid JSON — {exc}")
        else:
            validate(document, schema, "", errors)
            errors.extend(machine_paths(document, schema_name))

        if errors:
            total_violations += len(errors)
            print(f"{doc_path}: {len(errors)} violation(s)  [{schema_name}]")
            for error in errors:
                print(f"  {error}")
        else:
            print(f"{doc_path}: OK  [{schema_name}]")

    print()
    if skipped_clusters:
        print(
            f"Skipped {len(skipped_clusters)} cluster(s) not registered in "
            f"roadmap.json (legacy/v1): {', '.join(skipped_clusters)}"
        )
    if total_violations:
        print(
            f"{total_violations} violation(s) across "
            f"{len(documents)} document(s).",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"All {len(documents)} document(s) valid.")


if __name__ == "__main__":
    main()
