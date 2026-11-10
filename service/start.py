#!/usr/bin/python3
"""Discover this machine's Tailscale interfaces, then replace ourselves with Caddy."""

import ipaddress
import os
from pathlib import Path
import socket
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
TAILSCALE_CANDIDATES = (
    "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
    "/opt/homebrew/bin/tailscale",
    "/usr/local/bin/tailscale",
)


def bindings():
    hosts = ["localhost", "127.0.0.1", "[::1]"]
    cli = next((p for p in TAILSCALE_CANDIDATES if os.access(p, os.X_OK)), None)
    if cli is None:
        print("PortMap: Tailscale CLI unavailable; using loopback only.", flush=True)
        return hosts
    try:
        command = [cli, "ip"]
        if os.geteuid() == 0:
            # The macOS app's LocalAPI belongs to the console user's GUI session.
            uid = os.stat("/dev/console").st_uid
            if uid == 0:
                raise ValueError("no logged-in Tailscale user yet")
            command = ["/bin/launchctl", "asuser", str(uid), "/usr/bin/sudo",
                       "-n", "-u", f"#{uid}", cli, "ip"]
        result = subprocess.run(command, capture_output=True, text=True,
                                timeout=10, check=True)
        for line in result.stdout.splitlines():
            address = ipaddress.ip_address(line.strip())
            network = ipaddress.ip_network(
                "100.64.0.0/10" if address.version == 4 else "fd7a:115c:a1e0::/48")
            if address not in network:
                raise ValueError("Tailscale returned an address outside its address ranges")
            # Ignore stale addresses not currently assigned to this machine.
            family = socket.AF_INET if address.version == 4 else socket.AF_INET6
            try:
                with socket.socket(family) as probe:
                    probe.bind((str(address), 0))
            except OSError:
                print(f"PortMap: skipping unassigned Tailscale address {address}", flush=True)
                continue
            host = str(address) if address.version == 4 else f"[{address}]"
            if host not in hosts:
                hosts.append(host)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"PortMap: Tailscale discovery failed ({error}); using loopback only.", flush=True)
        return hosts[:3]
    return hosts


if __name__ == "__main__":
    hosts = bindings()
    print("PortMap: binding port 80 to " + ", ".join(hosts), flush=True)
    os.environ["PORTMAP_BIND"] = " ".join(hosts)
    os.environ["XDG_DATA_HOME"] = str(ROOT / "data")
    os.environ["XDG_CONFIG_HOME"] = str(ROOT / "config")
    os.execv(str(ROOT / "caddy"), [str(ROOT / "caddy"), "run", "--config",
                                 str(ROOT / "Caddyfile"), "--adapter", "caddyfile"])
