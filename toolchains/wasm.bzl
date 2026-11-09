"""The C/C++ toolchain a wasm32-unknown-unknown target (build//buck:wasm32) links with.

The prelude links a Rust cdylib through the C++ toolchain's linker: it hands
rustc -Clinker=<its wrapper> and the linker type's shared-library flags. For
wasm32 that linker is the recorded Rust toolchain's own rust-lld
(host_tools.rust_lld, which bucksetup finds under its sysroot), with no flags
of its own, so rustc invokes it as Cargo's wasm32 build does (`-flavor wasm`
first), and the linker type is the prelude's "wasm", which asks for no soname.
system_cxx_toolchain cannot say either: its linker type is the host's, and an
OS-less target looks like linux to it (the prelude's os_lookup default), so
it puts -fuse-ld=lld before rustc's -flavor, which rust-lld refuses.

Nothing here compiles C for wasm32: the compilers are the host's, so a crate
whose build script compiles C for wasm32 fails there, by name.
"""

load(
    "@prelude//cxx:cxx_toolchain_types.bzl",
    "BinaryUtilitiesInfo",
    "CCompilerInfo",
    "CxxCompilerInfo",
    "CxxInternalTools",
    "CxxPlatformInfo",
    "CxxToolchainInfo",
    "DepTrackingMode",
    "LinkerInfo",
    "LinkerType",
    "PicBehavior",
    "RuntimeDependencyHandling",
    "ShlibInterfacesMode",
)
load("@prelude//cxx:headers.bzl", "HeaderMode")
load("@prelude//linking:link_info.bzl", "LinkStyle")
load("@prelude//linking:lto.bzl", "LtoMode")

def _run(tool):
    return RunInfo(args = [tool])

def _impl(ctx):
    return [
        DefaultInfo(),
        CxxToolchainInfo(
            internal_tools = ctx.attrs._internal_tools[CxxInternalTools],
            linker_info = LinkerInfo(
                linker = _run(ctx.attrs.linker),
                linker_flags = [],
                post_linker_flags = [],
                archiver = _run(ctx.attrs.archiver),
                archiver_type = "gnu",
                archiver_supports_argfiles = True,
                generate_linker_maps = False,
                lto_mode = LtoMode("none"),
                type = LinkerType("wasm"),
                link_binaries_locally = True,
                link_libraries_locally = True,
                archive_objects_locally = True,
                use_archiver_flags = True,
                static_dep_runtime_ld_flags = [],
                static_pic_dep_runtime_ld_flags = [],
                shared_dep_runtime_ld_flags = [],
                independent_shlib_interface_linker_flags = [],
                shlib_interfaces = ShlibInterfacesMode("disabled"),
                link_style = LinkStyle("static"),
                link_weight = 1,
                binary_extension = "wasm",
                object_file_extension = "o",
                shared_library_name_default_prefix = "",
                shared_library_name_format = "{}.wasm",
                shared_library_versioned_name_format = "{}.wasm",
                static_library_extension = "a",
                force_full_hybrid_if_capable = False,
                is_pdb_generated = False,
            ),
            bolt_enabled = False,
            binary_utilities_info = BinaryUtilitiesInfo(
                nm = RunInfo(args = ["nm"]),
                objcopy = RunInfo(args = ["objcopy"]),
                objdump = RunInfo(args = ["objdump"]),
                ranlib = RunInfo(args = ["ranlib"]),
                strip = RunInfo(args = ["strip"]),
                dwp = None,
            ),
            cxx_compiler_info = CxxCompilerInfo(compiler = _run(ctx.attrs.cxx_compiler), preprocessor_flags = [], compiler_flags = [], compiler_type = "clang"),
            c_compiler_info = CCompilerInfo(compiler = _run(ctx.attrs.compiler), preprocessor_flags = [], compiler_flags = [], compiler_type = "clang"),
            as_compiler_info = CCompilerInfo(compiler = _run(ctx.attrs.compiler), compiler_type = "clang"),
            asm_compiler_info = CCompilerInfo(compiler = _run(ctx.attrs.compiler), compiler_type = "clang"),
            header_mode = HeaderMode("symlink_tree_only"),
            cpp_dep_tracking_mode = DepTrackingMode("show_headers"),
            # wasm32 has no position-independent static code (Cargo's default
            # relocation model for the target is static; toolchains/BUCK).
            pic_behavior = PicBehavior("not_supported"),
            llvm_link = None,
            use_dep_files = True,
            runtime_dependency_handling = RuntimeDependencyHandling("no_symlink"),
        ),
        CxxPlatformInfo(name = "wasm32"),
    ]

wasm_cxx_toolchain = rule(
    impl = _impl,
    attrs = {
        "archiver": attrs.string(),
        "compiler": attrs.string(),
        "cxx_compiler": attrs.string(),
        "linker": attrs.string(),
        "_internal_tools": attrs.default_only(attrs.exec_dep(providers = [CxxInternalTools], default = "prelude//cxx/tools:internal_tools")),
    },
    is_toolchain_rule = True,
)
