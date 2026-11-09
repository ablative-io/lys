"""Host executables are discovered into ignored machine-local config."""
load("@prelude//rust:rust_toolchain.bzl", "PanicRuntime", "RustToolchainInfo")

def _toolchain(ctx, compiler, rustdoc = None):
    return [
        DefaultInfo(),
        RustToolchainInfo(
            compiler = compiler,
            clippy_driver = RunInfo(args = [read_root_config("host_tools", "clippy")]),
            rustdoc = rustdoc or RunInfo(args = [ctx.attrs._runner[RunInfo], "--rustdoc", read_root_config("host_tools", "rustdoc"), "--"]),
            rustc_target_triple = ctx.attrs.target_triple or read_root_config("host_tools", "rust_target"),
            rustc_flags = ctx.attrs.rustc_flags,
            default_edition = "2024",
            panic_runtime = PanicRuntime("unwind"),
            nightly_features = read_root_config("host_tools", "rust_nightly", "false") == "true",
        ),
    ]

# Third-party crates compile through buckrun too, so each rustc takes one of
# the machine's build slots (@build//buck/buckrun/slots.go).
def _impl(ctx):
    return _toolchain(ctx, RunInfo(args = [ctx.attrs._runner[RunInfo], "--exec", read_root_config("host_tools", "rustc"), "--"]))

# Crates declared runtime_manifest in buckgen.json (see @build//buck:thirdparty.bzl).
def _runtime_manifest_impl(ctx):
    return _toolchain(ctx, RunInfo(args = [ctx.attrs._runner[RunInfo], "--runtime-manifest-rustc", read_root_config("host_tools", "rustc"), "--"]))

# Workspace crates compile, as Cargo compiles them, with CARGO_MANIFEST_DIR
# naming the package in the checkout (see @build//buck/buckrun/workspace.go).
# The checkout's path is in the command line, so it is in the cache key.
def _workspace_impl(ctx):
    root = read_root_config("host_tools", "project_root")
    if not root:
        fail("host_tools.project_root is unset; build through toolchains/_local/buckbuild")
    return _toolchain(
        ctx,
        RunInfo(args = [ctx.attrs._runner[RunInfo], "--root", root, "--rustc", read_root_config("host_tools", "rustc"), "--"]),
        RunInfo(args = [ctx.attrs._runner[RunInfo], "--root", root, "--test-builder", read_root_config("host_tools", "rustc"), "--rustdoc", read_root_config("host_tools", "rustdoc"), "--"]),
    )

# target_triple and rustc_flags are selected per target platform in BUCK: a
# static musl deployable (build//buck:linux-x86_64-musl) names its own triple
# and links through zig's musl, while build scripts and proc macros, built for
# the execution platform, keep the host's.
_attrs = {
    "rustc_flags": attrs.list(attrs.string(), default = []),
    "target_triple": attrs.option(attrs.string(), default = None),
    "_runner": attrs.exec_dep(default = "toolchains//:buckrun", providers = [RunInfo]),
}

host_rust_toolchain = rule(impl = _impl, attrs = _attrs, is_toolchain_rule = True)

runtime_manifest_rust_toolchain = rule(impl = _runtime_manifest_impl, attrs = _attrs, is_toolchain_rule = True)

workspace_rust_toolchain = rule(impl = _workspace_impl, attrs = _attrs, is_toolchain_rule = True)
