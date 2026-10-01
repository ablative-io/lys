"""Advance readiness retries without delaying real HTTP and process operations."""

import importlib.util
import sys


class LogicalClock:
    def __init__(self):
        self.elapsed = 0

    def monotonic(self):
        return self.elapsed

    def sleep(self, seconds):
        self.elapsed += seconds


spec = importlib.util.spec_from_file_location("readiness_leg", sys.argv[1])
if spec is None or spec.loader is None:
    raise RuntimeError("the readiness script has no Python loader")
leg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(leg)
# Only the readiness loop sees logical time; sockets and child processes stay real.
leg.time = LogicalClock()
sys.exit(leg.main(sys.argv[2:]))
