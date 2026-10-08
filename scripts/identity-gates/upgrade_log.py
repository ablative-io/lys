"""Read committed leaf bytes without opening a store for writing."""

import hashlib
import json
from pathlib import Path
import re

NAME = re.compile(r"[0-9]{20}")
TEMP = re.compile(r"\.\d+-\d{20}-\d+\.tmp")


def proof_directory(root, directory, label):
    original = root.absolute()
    root = root.resolve()
    directory = Path(directory)
    if not directory.is_absolute() or not directory.resolve().is_relative_to(root):
        raise RuntimeError(f"fixture {label} leaves its root")
    if directory.is_relative_to(original):
        relative = directory.relative_to(original)
    elif directory.is_relative_to(root):
        relative = directory.relative_to(root)
    else:
        raise RuntimeError(f"fixture {label} leaves its root")
    if ".." in relative.parts:
        raise RuntimeError(f"fixture {label} leaves its root")
    directory = root / relative
    current = directory
    while current != root:
        if current.is_symlink():
            raise RuntimeError(f"fixture {label} contains a symlink")
        current = current.parent
    return directory


def regular(path):
    if path.is_symlink() or not path.is_file():
        raise RuntimeError(f"log entry is a symlink or not a file: {path}")


def physical_files(directory):
    """Include metadata and unfinished writes in the refusal oracle."""
    if directory.is_symlink() or not directory.is_dir():
        raise RuntimeError(f"log directory is a symlink or absent: {directory}")
    result = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(f"log entry is a symlink: {path}")
        if path.is_dir():
            continue
        regular(path)
        result[str(path.relative_to(directory))] = hashlib.sha256(
            path.read_bytes()
        ).hexdigest()
    return result


def crc_table():
    table = []
    for byte in range(256):
        crc = byte
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82F63B78 if crc & 1 else 0)
        table.append(crc)
    return tuple(table)


CRC_TABLE = crc_table()


def crc32c(content):
    crc = 0xFFFFFFFF
    for byte in content:
        crc = CRC_TABLE[(crc ^ byte) & 255] ^ (crc >> 8)
    return crc ^ 0xFFFFFFFF


def segment_leaves(directory):
    if directory.is_symlink() or not directory.is_dir():
        raise RuntimeError(f"segments directory is a symlink or absent: {directory}")
    segments = []
    offsets = set()
    for path in sorted(directory.iterdir()):
        if TEMP.fullmatch(path.name):
            continue
        regular(path)
        name = path.name.removesuffix(".offsets")
        if not NAME.fullmatch(name) or int(name) > 2**64 - 1:
            raise RuntimeError(f"unexpected segment leaf name {path.name}")
        if path.name.endswith(".offsets"):
            offsets.add(int(name))
        else:
            segments.append(path)
    firsts = {int(path.name) for path in segments}
    if offsets - firsts:
        raise RuntimeError("offsets file has no segment")
    if not segments or int(segments[0].name) != 0:
        raise RuntimeError("segment leaf indexes are not contiguous from zero")
    leaves = []
    committed = 0
    for number, path in enumerate(segments):
        if int(path.name) != len(leaves):
            raise RuntimeError("segment leaf indexes are not contiguous")
        content = memoryview(path.read_bytes())
        cursor = 0
        while cursor < len(content):
            start = cursor
            if len(content) - cursor < 4:
                break
            length = int.from_bytes(content[cursor : cursor + 4], "little")
            flag_at = cursor + 4 + length
            if flag_at + 5 > len(content):
                break
            flag = content[flag_at]
            if flag not in (0, 1):
                raise RuntimeError(
                    f"invalid pin flag in segment {path.name} at {cursor}"
                )
            end = flag_at + 1 + 40 * flag
            if end + 4 > len(content):
                break
            expected = int.from_bytes(content[end : end + 4], "little")
            if crc32c(content[start:end]) != expected:
                raise RuntimeError(
                    f"CRC-32C mismatch in segment {path.name} at {cursor}"
                )
            leaves.append(bytes(content[cursor + 4 : flag_at]))
            cursor = end + 4
            if flag:
                size = int.from_bytes(content[flag_at + 1 : flag_at + 9], "little")
                if size != len(leaves):
                    raise RuntimeError(
                        f"segment pin has tree size {size}, expected {len(leaves)}"
                    )
                committed = size
        if number + 1 < len(segments) and (
            cursor != len(content) or committed != len(leaves)
        ):
            raise RuntimeError(f"sealed segment {path.name} has an unfinished act")
    del leaves[committed:]
    return leaves


def log_leaves(directory):
    """Return leaf bytes in index order, ending at the last complete pinned act."""
    if directory.is_symlink():
        raise RuntimeError(f"log directory is a symlink: {directory}")
    leaves = directory / "leaves"
    if leaves.is_symlink() or not leaves.is_dir():
        raise RuntimeError(f"leaves directory is a symlink or absent: {leaves}")
    marker = directory / "log.json"
    format_name = None
    if marker.exists() or marker.is_symlink():
        regular(marker)
        format_name = json.loads(marker.read_bytes()).get("format")
        if format_name not in ("lys/log-dir/v1", "lys/log-dir/v2"):
            raise RuntimeError(f"unsupported log format {format_name!r}")
    segments = leaves / "segments"
    if segments.exists() or segments.is_symlink() or format_name == "lys/log-dir/v2":
        if format_name != "lys/log-dir/v2":
            raise RuntimeError("segments require the v2 log marker")
        for path in leaves.iterdir():
            if path.name != "segments" and not TEMP.fullmatch(path.name):
                raise RuntimeError(f"unexpected segment leaf name {path.name}")
        return segment_leaves(segments)
    result = []
    for path in sorted(leaves.iterdir()):
        if TEMP.fullmatch(path.name):
            continue
        if NAME.fullmatch(path.name) is None:
            raise RuntimeError(f"unexpected directory leaf name {path.name}")
        regular(path)
        if int(path.name) != len(result):
            raise RuntimeError("directory leaf indexes are not contiguous from zero")
        result.append(path.read_bytes())
    state = directory / "state.json"
    if state.exists() or state.is_symlink():
        regular(state)
        size = json.loads(state.read_bytes()).get("tree_size")
        if type(size) is not int or not 0 <= size <= len(result):
            raise RuntimeError(
                f"v1 pin tree size exceeds its contiguous leaf extent: {state}"
            )
        del result[size:]
    return result


def indexed_files(root, directory, encode):
    prefix = str((directory / "leaves").relative_to(root.resolve()))
    return {
        f"{prefix}/{index:020}": encode(content)
        for index, content in enumerate(log_leaves(directory))
    }
