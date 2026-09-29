"""Resolve the installer's declared paths before a disposable upgrade can start."""

import ast
import hashlib
import re
import shutil
from pathlib import Path

LAYOUT = "crates/lys/src/identity/install/layout.rs"
IDENTITY = "crates/lys/src/identity/"


def require(text, fragment, source):
    if re.sub(r"\s+", "", fragment) not in re.sub(r"\s+", "", text):
        raise RuntimeError(f"upgrade path contract absent in {source}: {fragment}")


def binary_names(text):
    declaration = re.search(r'pub const BINARIES: \[&str; (\d+)\] = \[([^;]+)\];', text)
    if declaration is None:
        raise RuntimeError(f"{LAYOUT}: installed BINARIES declaration is absent")
    names = re.findall(r'"([^"/]+)"', declaration[2])
    if len(names) != int(declaration[1]) or len(set(names)) != len(names) or not names:
        raise RuntimeError(f"{LAYOUT}: installed BINARIES declaration cannot be resolved")
    return names


def resolve_layout(text):
    bodies = dict(re.findall(r'pub fn (\w+)\(&self\) -> PathBuf \{\s*([^{}]+?)\s*\}', text))
    resolved = {}

    def resolve(name, visiting):
        if name in visiting or name not in bodies:
            raise RuntimeError(f"{LAYOUT}: missing or cyclic path getter {name}")
        body = re.sub(r"\s+", "", bodies[name])
        match = re.fullmatch(r'self\.(root|\w+\(\))((?:\.join\("[^"/]+"\))*)', body)
        if match is None:
            raise RuntimeError(f"{LAYOUT}: cannot resolve path getter {name}: {bodies[name]}")
        parent = Path() if match[1] == "root" else resolve(match[1][:-2], visiting | {name})
        value = parent.joinpath(*re.findall(r'\.join\("([^"/]+)"\)', match[2]))
        resolved[name] = value
        return value

    for name in bodies:
        resolve(name, set())
    return resolved


def inventory(read, commit, candidate=False):
    """Validate every fixed product path the harness uses against this revision's producers."""
    text = read(LAYOUT)
    getters = resolve_layout(text)
    binaries = binary_names(text)
    rows = []

    def row(name, path, source, kind="generated"):
        rows.append({"name": name, "path": str(path), "source": source, "kind": kind})

    expected = {
        "deployment_config": "deployment.toml", "headless_setup_code": "setup-code",
        "service_config": "identity.json", "bin_dir": "bin", "bin_previous_dir": "bin.previous",
        "config_previous_dir": "config.previous", "upgrade_intent": "install/upgrade.json",
        "logs_dir": "logs", "run_dir": "run", "runner_socket": "run/runner.sock",
        "deploy_dir": "deploy", "data_dir": "data",
    }
    for name, path in expected.items():
        if name not in getters or getters[name] != Path(path):
            raise RuntimeError(f"{commit} {LAYOUT}: {name} does not resolve to harness path {path}")
        row(name, path, f"{LAYOUT}::{name}")
    require(text, 'self.bin_dir().join(name)', LAYOUT)
    for name in binaries:
        for directory in ("bin_dir", "bin_previous_dir"):
            row(f"{directory}/{name}", getters[directory] / name, f"{LAYOUT}::BINARIES")
    row("rollback_identity", getters["config_previous_dir"] / "identity.json", IDENTITY + "upgrade/render.rs")
    require(read(IDENTITY + "upgrade/render.rs"), 'file("identity.json", layout.service_config()', IDENTITY + "upgrade/render.rs")

    units = read(IDENTITY + "upgrade.rs")
    install = read(IDENTITY + "install.rs")
    for name in ("secrets", "identity", "runner"):
        source = IDENTITY + ("install.rs" if name == "runner" else "upgrade.rs")
        producer = install if name == "runner" else units
        for directory, suffix in (("logs_dir", "log"), ("run_dir", "pid")):
            fragment = f'layout.{directory}().join("{name}.{suffix}")'
            require(producer, fragment, source)
            row(f"{name}_{suffix}", getters[directory] / f"{name}.{suffix}", source)
    exit_source = IDENTITY + "install/exit_wait.rs"
    require(read(exit_source), 'pid_file.with_extension("exit")', exit_source)
    row("identity_exit", getters["run_dir"] / "identity.exit", exit_source)

    config_source = IDENTITY + "install/server_config.rs"
    config = read(config_source)
    require(config, 'let data = layout.data_dir();', config_source)
    require(config, 'let dir = |name: &str| Value::String(data.join(name).display().to_string());', config_source)
    for key, directory in (("log_dir", "directory-log"), ("grant_log_dir", "grant-log"),
                           ("teams_dir", "teams"), ("budgets_dir", "budgets"),
                           ("provisioning_file", "provisioning.json")):
        require(config, f'"{key}": dir("{directory}")', config_source)
        row(key, getters["data_dir"] / directory, config_source)
    # These recursively hashed records are deliberately discovered, never guessed from a count.
    for directory in ("directory-log", "teams", "budgets"):
        row(directory + "_records", getters["data_dir"] / directory / "**/*", config_source, "discovered")
    row("app_leaves", getters["data_dir"] / "apps/leaves/[0-9]{20}",
        "crates/lys-identity-server/src/config.rs", "discovered")
    require(read("crates/lys-identity-server/src/config.rs"), '.with_file_name("apps")',
            "crates/lys-identity-server/src/config.rs")
    import_source = IDENTITY + "import.rs"
    require(read(import_source), 'layout.root.join("keys").join("identity-loader.credential")', import_source)
    row("import_credential_file", "keys/identity-loader.credential", import_source)
    template_source = IDENTITY + "install/deployment.template.toml"
    require(read(template_source), 'state_dir = "state"', template_source)
    for constant, filename in (("OPERATOR_TOKEN_FILE", "operator-token"),
                               ("PROVIDERS_KEY_FILE", "sign-in-providers-api-key")):
        if constant == "OPERATOR_TOKEN_FILE" and not candidate:
            # Old builds have no operator-token API. Only the candidate verifier reads it.
            row(constant, "state/" + filename, "candidate server_config.rs", "candidate-only")
            continue
        require(config, f'pub const {constant}: &str = "{filename}";', config_source)
        require(config, f'state.join({constant})', config_source)
        row(constant, "state/" + filename, config_source)
    require(read(IDENTITY + "prepare.rs"), 'pub const COMPOSE_ENV: &str = "compose.env";', IDENTITY + "prepare.rs")
    row("compose_env", "state/compose.env", IDENTITY + "prepare.rs")
    # The installer copies the shipped compose to Layout::deploy_dir.
    require(install, 'layout.deploy_dir().join("compose.yaml")', IDENTITY + "install.rs")
    row("compose", "deploy/compose.yaml", IDENTITY + "install.rs")
    return {"commit": commit, "installed_binaries": binaries, "paths": rows,
            "layout_sha256": hashlib.sha256(text.encode()).hexdigest()}


def harness_inventory(directory):
    """Every local verifier module must exist before the installer can invoke Python."""
    pending = ["upgrade_live"]
    modules = {}
    while pending:
        name = pending.pop()
        if name in modules:
            continue
        path = directory / (name + ".py")
        if not path.is_file():
            raise RuntimeError(f"upgrade verifier module is missing: {path}")
        text = path.read_text()
        modules[name] = {"path": str(path), "sha256": hashlib.sha256(text.encode()).hexdigest()}
        pending.extend(re.findall(r"^from (upgrade_\w+) import", text, re.M))
    return modules


def harness_paths(modules, layouts):
    """List every Path division in executable proof modules; fixed root paths must have a producer."""
    products = {row["path"] for layout in layouts.values() for row in layout["paths"]}
    owned = {".upgrade-proof", ".upgrade-window.json"}
    rows = []
    for module, entry in modules.items():
        if module in ("upgrade_layout", "upgrade_preflight"):
            continue
        tree = ast.parse(Path(entry["path"]).read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "Path":
                rows.append({"source": f"{entry['path']}:{node.lineno}",
                             "expression": ast.unparse(node), "producer": "artifact, source, or scoped configuration read"})
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "with_name":
                rows.append({"source": f"{entry['path']}:{node.lineno}",
                             "expression": ast.unparse(node), "producer": "sibling apps store or verified module"})
            if not isinstance(node, ast.BinOp) or not isinstance(node.op, ast.Div):
                continue
            left = ast.unparse(node.left)
            right = node.right.value if isinstance(node.right, ast.Constant) else None
            if left == "root" and isinstance(right, str) and right not in products | owned:
                raise RuntimeError(f"{entry['path']}:{node.lineno}: no installer or harness producer for {right}")
            rows.append({"source": f"{entry['path']}:{node.lineno}",
                         "expression": ast.unparse(node),
                         "producer": "installer" if left == "root" and right in products else "harness or discovered record"})
    return rows


def executables():
    result = {}
    for name in ("python3", "git", "docker", "ps", "sh"):
        path = shutil.which(name)
        if path is None:
            raise RuntimeError(f"upgrade executable is absent from PATH: {name}")
        result[name] = path
    if not Path("/bin/sh").is_file():
        raise RuntimeError("headless fixture interpreter is absent: /bin/sh")
    result["headless_interpreter"] = "/bin/sh"
    return result
