"""Host executables are discovered into ignored machine-local config."""
load("@prelude//rust:rust_toolchain.bzl", "PanicRuntime", "RustToolchainInfo")

def _toolchain(ctx, compiler):
    return [
        DefaultInfo(),
        RustToolchainInfo(
            compiler = compiler,
            clippy_driver = RunInfo(args = [read_root_config("host_tools", "clippy")]),
            rustdoc = RunInfo(args = [ctx.attrs._runner[RunInfo], "--rustdoc", read_root_config("host_tools", "rustdoc"), "--"]),
            rustc_target_triple = read_root_config("host_tools", "rust_target"),
            default_edition = "2024",
            panic_runtime = PanicRuntime("unwind"),
            nightly_features = read_root_config("host_tools", "rust_nightly", "false") == "true",
        ),
    ]

def _impl(ctx):
    return _toolchain(ctx, RunInfo(args = [read_root_config("host_tools", "rustc")]))

# Crates declared runtime_manifest in buckgen.json (see @build//buck:thirdparty.bzl).
def _runtime_manifest_impl(ctx):
    return _toolchain(ctx, RunInfo(args = [ctx.attrs._runner[RunInfo], "--runtime-manifest-rustc", read_root_config("host_tools", "rustc"), "--"]))

_attrs = {"_runner": attrs.exec_dep(default = "toolchains//:buckrun", providers = [RunInfo])}

host_rust_toolchain = rule(impl = _impl, attrs = _attrs, is_toolchain_rule = True)

runtime_manifest_rust_toolchain = rule(impl = _runtime_manifest_impl, attrs = _attrs, is_toolchain_rule = True)
