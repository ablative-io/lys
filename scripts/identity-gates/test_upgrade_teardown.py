"""A leg hands over only after process exit; closed TCP connections are not listeners."""

import socket
import subprocess
import sys
import unittest
from types import SimpleNamespace
from pathlib import Path
from unittest.mock import patch
import tempfile

from upgrade_live import require_free, stop_fixture
from upgrade_teardown import terminate_fixture


class TeardownTests(unittest.TestCase):
    def test_linux_registers_exit_descriptor_before_signalling_and_waiting(self):
        events = []
        poller = SimpleNamespace(
            register=lambda fd, mask: events.append("register"),
            poll=lambda: events.append("exit") or [(7, 1)])
        with patch("upgrade_teardown.sys.platform", "linux"), \
                patch("upgrade_teardown.os.pidfd_open", create=True, return_value=7), \
                patch("upgrade_teardown.select.poll", return_value=poller), \
                patch("upgrade_teardown.signal.pidfd_send_signal", create=True,
                      side_effect=lambda fd, sig: events.append("signal")), \
                patch("upgrade_teardown.os.close", side_effect=lambda fd: events.append("close")):
            terminate_fixture(123)
        self.assertEqual(events, ["register", "signal", "exit", "close"])

    def test_unsupported_exit_observation_refuses_without_signalling(self):
        with patch("upgrade_teardown.sys.platform", "unsupported"), \
                patch("upgrade_teardown.os.kill") as kill:
            with self.assertRaisesRegex(RuntimeError, "process 123.*unsupported"):
                terminate_fixture(123)
        kill.assert_not_called()

    def test_teardown_refuses_process_not_owned_by_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "run").mkdir()
            (root / "run/identity.pid").write_text("123")
            with patch("upgrade_live.subprocess.run", return_value=subprocess.CompletedProcess(
                    [], 0, stdout="server --root /unrelated")), \
                    patch("upgrade_live.terminate_fixture") as terminate:
                with self.assertRaisesRegex(RuntimeError, "refuse to stop 123"):
                    stop_fixture(root, "fixture", root)
            terminate.assert_not_called()

    def test_real_child_exit_releases_listener_before_return(self):
        source = ("import socket,signal; s=socket.socket(); "
                  "s.bind(('127.0.0.1',0)); s.listen(); "
                  "print(s.getsockname()[1],flush=True); signal.pause()")
        with subprocess.Popen([sys.executable, "-c", source], stdout=subprocess.PIPE,
                              text=True) as child:
            try:
                number = int(child.stdout.readline())
                terminate_fixture(child.pid)
                require_free(number)
                self.assertIsNotNone(child.poll())
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait()

    def test_closed_connection_does_not_refuse_reusable_service_port(self):
        with socket.socket() as listener, socket.socket() as client:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            address = listener.getsockname()
            client.connect(address)
            accepted, peer = listener.accept()
            self.assertEqual(peer, client.getsockname())
            accepted.close()
            self.assertEqual(client.recv(1), b"")
            client.close()
            listener.close()
            require_free(address[1])

    def test_active_listener_is_refused_and_names_port(self):
        with socket.socket() as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            number = listener.getsockname()[1]
            with self.assertRaisesRegex(RuntimeError, str(number)):
                require_free(number)

    def test_teardown_waits_for_verified_fixture_process(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "run").mkdir()
            (root / "run/identity.pid").write_text("123")
            seen = []
            with patch("upgrade_live.subprocess.run", return_value=subprocess.CompletedProcess(
                    [], 0, stdout=f"server --root {root}")), \
                    patch("upgrade_live.os.kill", side_effect=lambda *args: seen.append("signal")), \
                    patch("upgrade_live.terminate_fixture", create=True,
                          side_effect=lambda pid: seen.append("exit observed")):
                stop_fixture(root, "fixture", root)
            self.assertEqual(seen, ["exit observed"])


if __name__ == "__main__":
    unittest.main()
