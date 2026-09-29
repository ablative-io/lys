"""Probe exact fixture socket paths before any installer or shared TCP port is used."""

import socket


def socket_paths(work):
    """Use the kernel's AF_UNIX contract, not an invented maximum path length."""
    checked = []
    for leg in ("positive", "negative"):
        path = work / leg / "install" / "run" / "runner.sock"
        created = []
        parent = path.parent
        while not parent.exists():
            created.append(parent)
            parent = parent.parent
        if path.exists() or path.is_symlink():
            raise RuntimeError(f"fixture socket path already exists: {path}")
        bound = False
        try:
            for directory in reversed(created):
                directory.mkdir(mode=0o700)
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
                try:
                    probe.bind(str(path))
                    bound = True
                except OSError as error:
                    raise RuntimeError(f"fixture runner socket unavailable at {path}: {error}") from error
            checked.append(str(path))
        finally:
            if bound:
                path.unlink()
            for directory in created:
                directory.rmdir()
    return checked
