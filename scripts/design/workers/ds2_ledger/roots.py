"""The root token a design or a brief spells a path under, parsed in one place.

A gate tree, a structure path and a brief's file path are read on whatever machine the document
is fired on. A path inside the repository the document lives in can be written relative to its
root; a path outside it - the method's own tree, carrying this ledger, is the one this cluster
needs - can only be written absolutely, and an absolute path is one machine's. So such a path is
written with a root at its head instead: a dollar sign, an upper-case name and a slash. The name
is resolved to a directory from the launch settings of the process that reads it, under the
prefix every setting of this ledger already carries.

This module is the only place in the method that reads the token, and it holds nothing but that
reading: no environment, no refusal, no SDK. The ledger's own resolver (`documents.resolve_root`)
is built on it and raises the refusal; the method's coverage check, which compares a brief's
paths against the structure as spelled and never resolves one, imports it directly - a script run
by a plain interpreter that has no worker SDK installed. One parser, whatever reads it.
"""

from __future__ import annotations

import re

# The prefix the ledger's launch settings already carry (DS2_BATTERY_LOCK, DS2_SEAT): a root
# named METHOD is the setting DS2_METHOD, so a lane names its roots on the same launch line as
# everything else the ledger is told.
ROOT_SETTING_PREFIX = "DS2_"

_ROOT_TOKEN = re.compile(r"^\$([A-Z][A-Z0-9_]*)/")


def named_root(path: str) -> str | None:
    """The root a path is spelled under, or None when it names none."""
    found = _ROOT_TOKEN.match(path)
    return found.group(1) if found else None


def root_setting(name: str) -> str:
    """The launch setting one root's directory is named by."""
    return ROOT_SETTING_PREFIX + name


def without_root(path: str) -> str:
    """What a rooted path names under its root: the path with the token taken off its head."""
    name = named_root(path)
    return path[len(name) + 2 :] if name is not None else path
