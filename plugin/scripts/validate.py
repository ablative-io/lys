#!/usr/bin/env python3
"""Validate both Lys plugin distributions and optional live schema receipts."""

import argparse
import json
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
BASES = (ROOT / "plugin", ROOT / "plugins/lys")
TOOLS = {"what-can-I-do", "read", "change", "drafts"}
ENDPOINT = "https://lys.ablative.com.au/api/mcp"
METHODS = {"get", "post", "put", "patch", "delete"}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load(path):
    return json.loads(path.read_text())


def frontmatter(path):
    parts = path.read_text().split("---\n", 2)
    require(len(parts) == 3 and not parts[0], f"frontmatter_missing: {path.name}")
    fields = {}
    for line in parts[1].splitlines():
        key, separator, value = line.partition(": ")
        require(separator and key not in fields, f"frontmatter_invalid: {path.name}")
        fields[key] = json.loads(value) if value.startswith('"') else value
    require(bool(fields.get("description")), f"description_missing: {path.name}")
    return fields, parts[2].strip()


def within(base, relative):
    require(relative.startswith("./"), "component_path_requires_dot_slash")
    path = base / relative
    require(
        path.resolve().is_relative_to(base.resolve()), "component_path_escapes_plugin"
    )
    require(path.exists(), "component_path_missing")
    return path


def validate_packages():
    claude_market = load(ROOT / ".claude-plugin/marketplace.json")
    codex_market = load(ROOT / ".agents/plugins/marketplace.json")
    require(
        claude_market["name"] == codex_market["name"] == "lys-local",
        "marketplace_name_mismatch",
    )
    require(
        len(claude_market["plugins"]) == len(codex_market["plugins"]) == 1,
        "marketplace_entry_count",
    )
    require(
        within(ROOT, claude_market["plugins"][0]["source"]) == BASES[0],
        "claude_source_mismatch",
    )
    require(
        within(ROOT, codex_market["plugins"][0]["source"]["path"]) == BASES[1],
        "codex_source_mismatch",
    )
    require(
        codex_market["plugins"][0]["source"]["source"] == "local", "codex_source_kind"
    )
    require(
        codex_market["plugins"][0]["policy"]["authentication"] == "ON_INSTALL",
        "oauth_policy_missing",
    )
    catalogs = []
    for base, flavor in zip(BASES, (".claude-plugin", ".codex-plugin")):
        manifest = load(base / flavor / "plugin.json")
        require(
            manifest["name"] == "lys" and manifest["version"] == "0.1.0",
            "plugin_identity_mismatch",
        )
        config = load(within(base, manifest["mcpServers"]))
        require(
            config == {"mcpServers": {"lys": {"type": "http", "url": ENDPOINT}}},
            "remote_oauth_wiring_mismatch",
        )
        if flavor == ".codex-plugin":
            require(
                within(base, manifest["skills"]) == base / "skills",
                "codex_skills_missing",
            )
        require((base / "README.md").is_file(), "readme_missing")
        for path in base.rglob("*"):
            require(not path.is_symlink(), "package_symlink_not_portable")
            if path.is_file():
                require(
                    len(path.read_text().splitlines()) < 500,
                    f"file_too_long: {path.name}",
                )
        for path in (base / "skills").glob("*/SKILL.md"):
            fields, body = frontmatter(path)
            require(
                fields["name"] == path.parent.name and bool(body),
                "skill_name_or_body_invalid",
            )
        catalog = load(base / "config/routes.json")
        pairs = [(row["method"], row["path"]) for row in catalog]
        require(len(set(pairs)) == len(pairs), "duplicate_route")
        for row in catalog:
            require(row["method"].lower() in METHODS, "route_method_invalid")
            require(
                row["path"].startswith("/") and not row["path"].startswith("/api/"),
                "route_path_invalid",
            )
            reference = base / "skills" / row["skill"] / "references/routes.md"
            require(reference.is_file(), "route_skill_reference_missing")
            require(
                f"| {row['method']} | `{row['path']}` |" in reference.read_text(),
                "route_reference_missing",
            )
        catalogs.append(catalog)
    require(catalogs[0] == catalogs[1], "catalogs_drifted")
    for path in (BASES[0] / "skills").rglob("*"):
        if path.is_file():
            other = BASES[1] / path.relative_to(BASES[0])
            require(other.read_bytes() == path.read_bytes(), "shared_skill_drifted")
    commands = list((BASES[0] / "commands").glob("*.md"))
    require(
        {path.stem for path in commands}
        == {
            "setup",
            "whoami",
            "capabilities",
            "my-agents",
            "read-route",
            "give-access",
            "drafts",
        },
        "command_inventory_incomplete",
    )
    for path in commands:
        fields, body = frontmatter(path)
        require(
            fields.get("disable-model-invocation") == "true",
            "command_implicit_invocation",
        )
        exported, exported_body = frontmatter(
            BASES[1] / "skills" / path.stem / "SKILL.md"
        )
        require(
            exported["description"] == fields["description"],
            "command_description_drifted",
        )
        require(
            exported_body
            == body.replace("$ARGUMENTS", "the user's invocation arguments"),
            "command_export_drifted",
        )
    for path in (BASES[0] / "agents").glob("*.md"):
        fields, body = frontmatter(path)
        native = tomllib.loads(
            (BASES[1] / "agents" / f"lys-{path.stem}.toml").read_text()
        )
        require(native["name"] == f"lys-{fields['name']}", "agent_name_mismatch")
        require(native["sandbox_mode"] == "read-only", "companion_sandbox_mismatch")
        require(
            native["developer_instructions"].strip() == body,
            "agent_instructions_drifted",
        )
    return catalogs[0], len(commands)


def validate_live(catalog, api_path, tools_path):
    if api_path:
        api = load(api_path)
        declared = {
            (method.upper(), path)
            for path, operations in api["paths"].items()
            for method in operations
            if method in METHODS
        }
        packaged = {(row["method"], row["path"]) for row in catalog}
        require(
            declared == packaged,
            f"live_route_coverage_mismatch: missing={len(declared - packaged)} extra={len(packaged - declared)}",
        )
    if tools_path:
        listed = load(tools_path)["tools"]
        require(
            len(listed) == 4 and {tool["name"] for tool in listed} == TOOLS,
            "live_tool_inventory_mismatch",
        )
        schemas = {tool["name"]: tool["inputSchema"] for tool in listed}
        require(
            schemas["what-can-I-do"]["additionalProperties"] is False,
            "discovery_schema_changed",
        )
        require(
            schemas["what-can-I-do"]["properties"] == {}, "discovery_arguments_changed"
        )
        require(
            set(schemas["change"]["required"]) == {"method", "path"},
            "change_arguments_changed",
        )
        require(
            set(schemas["change"]["properties"]["method"]["enum"])
            == {"POST", "PUT", "PATCH", "DELETE"},
            "change_methods_changed",
        )
        require(
            schemas["drafts"]["properties"]["path"]["pattern"] == "^/drafts(?:/|\\?|$)",
            "draft_path_contract_changed",
        )
        for name in ("read", "drafts"):
            require(
                set(schemas[name]["required"]) == {"path"}, "reading_arguments_changed"
            )
            require(
                set(schemas[name]["properties"]["method"]["enum"]) == {"GET", "POST"},
                "reading_methods_changed",
            )


def validate_person_reads():
    for name, route in (
        ("whoami", "/me"),
        ("my-agents", "/dashboard"),
        ("drafts", "/drafts"),
    ):
        _, body = frontmatter(BASES[0] / "commands" / f"{name}.md")
        words = body.lower()
        require("person reads" in words, f"person_read_consent_missing: {name}")
        require("agent mode" in words, f"agent_mode_boundary_missing: {name}")
        require("browser" in words, f"person_presence_boundary_missing: {name}")
        require(route in body, f"person_read_route_missing: {name}")
    _, drafts = frontmatter(BASES[0] / "commands/drafts.md")
    require(
        "deciding a draft needs you in Lys:" in drafts,
        "named_draft_decision_refusal_missing",
    )
    require("/product-drafts?limit=50" in drafts, "own_product_draft_read_missing")
    require("before paging" in drafts, "own_product_draft_page_scope_missing")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--openapi", type=Path)
    parser.add_argument("--tools-list", type=Path)
    args = parser.parse_args()
    try:
        catalog, commands = validate_packages()
        validate_person_reads()
        validate_live(catalog, args.openapi, args.tools_list)
    except (OSError, ValueError, KeyError, TypeError) as error:
        if isinstance(error, ValueError) and not isinstance(
            error, json.JSONDecodeError
        ):
            print(f"plugin_validation_refused: {error}", file=sys.stderr)
        else:
            print(
                f"plugin_validation_refused: {type(error).__name__}; check the supplied package or schema files",
                file=sys.stderr,
            )
        return 1
    groups = {row["path"].split("/")[1] for row in catalog}
    print(
        f"VERIFIED plugins=2 operations={len(catalog)} path_groups={len(groups)} commands={commands} live_openapi={bool(args.openapi)} live_tools={bool(args.tools_list)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
