"""Exercise real Caddy routing and listener isolation without needing root."""

import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import runpy
import shutil
import socket
import subprocess
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent
START = runpy.run_path(str(ROOT / "start.py"))


class DiscoveryTests(unittest.TestCase):
    def test_missing_tailscale_stays_loopback(self):
        with patch("os.access", return_value=False):
            self.assertEqual(START["bindings"](), ["localhost", "127.0.0.1", "[::1]"])

    def test_invalid_or_wildcard_output_stays_loopback(self):
        for output in ("0.0.0.0\n", "::\n", "192.168.1.5\n", "garbage\n"):
            with patch("os.access", return_value=True), patch(
                "subprocess.run", return_value=subprocess.CompletedProcess([], 0, output)
            ):
                self.assertEqual(START["bindings"](), ["localhost", "127.0.0.1", "[::1]"])

    def test_current_tailscale_addresses(self):
        with patch("os.access", return_value=True), patch(
            "subprocess.run", return_value=subprocess.CompletedProcess(
                [], 0, "100.64.0.1\nfd7a:115c:a1e0::1\n")
        ), patch("socket.socket"):
            self.assertEqual(START["bindings"](), [
                "localhost", "127.0.0.1", "[::1]", "100.64.0.1", "[fd7a:115c:a1e0::1]"])

    def test_root_uses_console_user_session(self):
        with patch("os.access", return_value=True), patch("os.geteuid", return_value=0), patch(
            "os.stat"
        ) as console, patch("subprocess.run", return_value=subprocess.CompletedProcess(
            [], 0, "100.64.0.1\n")) as run, patch("socket.socket"):
            console.return_value.st_uid = 501
            START["bindings"]()
            self.assertEqual(run.call_args.args[0], [
                "/bin/launchctl", "asuser", "501", "/usr/bin/sudo", "-n", "-u",
                "#501", START["TAILSCALE_CANDIDATES"][0], "ip"])


class Backend(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get("Upgrade", "").lower() == "websocket":
            self.send_response(101)
            self.send_header("Connection", "Upgrade")
            self.send_header("Upgrade", "websocket")
            self.end_headers()
            self.wfile.write(b"portmap-websocket")
            self.wfile.flush()
            return
        body = (self.path + " " + self.headers["Host"]).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class RoutingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.backend = ThreadingHTTPServer(("127.0.0.1", 0), Backend)
        cls.backend6 = type("IPv6Server", (ThreadingHTTPServer,), {
            "address_family": socket.AF_INET6})(("::1", 0), Backend)
        for backend in (cls.backend, cls.backend6):
            threading.Thread(target=backend.serve_forever, daemon=True).start()
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            cls.port = probe.getsockname()[1]
        cls.temp = tempfile.TemporaryDirectory()
        config = Path(cls.temp.name) / "Caddyfile"
        config.write_text((ROOT / "Caddyfile").read_text().replace(
            "http://:80 {", f"http://:{cls.port} {{"))
        cls.log = open(Path(cls.temp.name) / "caddy.log", "w+")
        cls.proxy = subprocess.Popen([
            shutil.which("caddy"), "run", "--config", str(config), "--adapter", "caddyfile"
        ], env={**os.environ, "PORTMAP_BIND": "localhost 127.0.0.1 [::1]"},
            stdout=cls.log, stderr=cls.log)
        for _ in range(100):
            try:
                with socket.create_connection(("::1", cls.port), timeout=0.1):
                    break
            except OSError:
                time.sleep(0.05)
        else:
            cls.proxy.terminate()
            cls.proxy.wait(timeout=5)
            cls.log.seek(0)
            raise RuntimeError(cls.log.read())

    @classmethod
    def tearDownClass(cls):
        cls.proxy.terminate()
        cls.proxy.wait(timeout=5)
        for backend in (cls.backend, cls.backend6):
            backend.shutdown()
            backend.server_close()
        cls.log.close()
        cls.temp.cleanup()

    def request(self, address, hostname):
        connection = http.client.HTTPConnection(address, self.port, timeout=5)
        try:
            connection.request("GET", "/test?query=kept", headers={"Host": hostname})
            response = connection.getresponse()
            return response.status, response.read().decode()
        finally:
            connection.close()

    def test_ipv4_and_ipv6_listeners_and_upstreams(self):
        for address in ("127.0.0.1", "::1"):
            for backend in (self.backend, self.backend6):
                port = backend.server_port
                for host in (f"{port}.localhost", f"{port}.LOCALHOST."):
                    self.assertEqual(self.request(address, host), (
                        200, f"/test?query=kept localhost:{port}"))

    def test_invalid_hosts_and_proxy_loop_rejected(self):
        for host in ("localhost", "example.com", "0.localhost", "65536.localhost",
                     "03000.localhost", "80.localhost", "80.LOCALHOST."):
            self.assertEqual(self.request("127.0.0.1", host)[0], 404, host)

    def test_closed_port_returns_bad_gateway(self):
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        self.assertEqual(self.request("127.0.0.1", f"{port}.localhost")[0], 502)

    def test_websocket_upgrade_and_payload(self):
        with socket.create_connection(("::1", self.port), timeout=5) as connection:
            connection.sendall((f"GET / HTTP/1.1\r\nHost: {self.backend.server_port}.localhost\r\n"
                                "Connection: Upgrade\r\nUpgrade: websocket\r\n\r\n").encode())
            data = b""
            while b"portmap-websocket" not in data:
                block = connection.recv(4096)
                if not block:
                    break
                data += block
            self.assertIn(b"101 Switching Protocols", data)
            self.assertIn(b"portmap-websocket", data)

    def test_no_wildcard_listener(self):
        listeners = subprocess.check_output([
            "lsof", "-nP", "-a", "-p", str(self.proxy.pid), "-iTCP", "-sTCP:LISTEN"
        ], text=True)
        self.assertIn(f"127.0.0.1:{self.port}", listeners)
        self.assertIn(f"[::1]:{self.port}", listeners)
        self.assertNotIn("*:", listeners)
        for line in listeners.splitlines()[1:]:
            self.assertIn(line.split()[8], (f"127.0.0.1:{self.port}", f"[::1]:{self.port}"))


if __name__ == "__main__":
    unittest.main()
