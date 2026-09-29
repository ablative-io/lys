"""Observe a verified fixture process's exit before handing its ports to another leg."""

import os
from contextlib import closing
import select
import signal
import sys


def terminate_fixture(pid):
    """Register before signalling; already-exited processes require no further wait."""
    if sys.platform == "darwin":
        with closing(select.kqueue()) as queue:
            event = select.kevent(pid, filter=select.KQ_FILTER_PROC,
                                  flags=select.KQ_EV_ADD | select.KQ_EV_ONESHOT,
                                  fflags=select.KQ_NOTE_EXIT)
            try:
                queue.control([event], 0, 0)
            except ProcessLookupError:
                return
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                return
            events = queue.control(None, 1, None)
            if not events or events[0].ident != pid or not events[0].fflags & select.KQ_NOTE_EXIT:
                raise RuntimeError(f"fixture process {pid}: expected kernel exit event, got {events}")
    elif sys.platform == "linux":
        try:
            descriptor = os.pidfd_open(pid)
        except ProcessLookupError:
            return
        try:
            poller = select.poll()
            poller.register(descriptor, select.POLLIN)
            try:
                signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            except ProcessLookupError:
                return
            events = poller.poll()
            if not events or events[0][0] != descriptor or not events[0][1] & select.POLLIN:
                raise RuntimeError(f"fixture process {pid}: expected kernel exit event, got {events}")
        finally:
            os.close(descriptor)
    else:
        raise RuntimeError(f"fixture process {pid}: kernel exit observation unsupported on {sys.platform}")
