#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""TVPN canary endpoint: logs who connects, nothing about what they asked.

Subcommands:
    https --port 443 --cert fullchain.pem --key privkey.pem
    udp   --port 9999

Logged per connection: UTC time, source IP, source port, protocol, byte count.
Never logged: path, query, headers, payload, user agent, referer.

This service is intentionally tiny and stateless. Run it on a host that is
disposable and dedicated to leak testing (see docs/canary.md).
"""

from __future__ import annotations

import argparse
import json
import socket
import ssl
import sys
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def _stamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _normalize(ip: str) -> str:
    # Dual-stack sockets report IPv4 peers as ::ffff:a.b.c.d
    if ip.startswith("::ffff:"):
        return ip[7:]
    return ip


def _log(proto: str, ip: str, port: int, extra: str = "") -> None:
    line = f"{_stamp()} {proto} {_normalize(ip)} {port}"
    if extra:
        line = f"{line} {extra}"
    print(line, flush=True)


class EchoHandler(BaseHTTPRequestHandler):
    server_version = "tvpn-canary/0.1"
    sys_version = ""

    def do_GET(self) -> None:  # noqa: N802 (http.server API)
        ip, port = self.client_address[0], self.client_address[1]
        _log("https", ip, port)
        body = json.dumps(
            {
                "seen_ip": _normalize(ip),
                "seen_port": port,
                "time": _stamp(),
            }
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args: object) -> None:
        # Never log request lines: they contain paths and user agents.
        return


class DualStackServer(ThreadingHTTPServer):
    address_family = socket.AF_INET6
    allow_reuse_address = True

    def server_bind(self) -> None:
        self.socket.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
        super().server_bind()


def run_https(port: int, cert: str | None, key: str | None) -> None:
    httpd = DualStackServer(("::", port), EchoHandler)
    if cert and key:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(certfile=cert, keyfile=key)
        httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
        scheme = "https"
    else:
        scheme = "http"
    print(f"canary: {scheme} echo on {port}", flush=True)
    httpd.serve_forever()


def run_udp(port: int) -> None:
    sock = socket.socket(socket.AF_INET6, socket.SOCK_DGRAM)
    sock.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
    sock.bind(("::", port))
    print(f"canary: udp listener on {port}", flush=True)
    while True:
        data, addr = sock.recvfrom(2048)
        # Only the size, never the payload.
        _log("udp", addr[0], addr[1], f"bytes={len(data)}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="TVPN canary endpoint")
    sub = parser.add_subparsers(dest="mode", required=True)

    https = sub.add_parser("https", help="HTTPS echo that reports the peer address")
    https.add_argument("--port", type=int, default=443)
    https.add_argument("--cert")
    https.add_argument("--key")

    udp = sub.add_parser("udp", help="UDP listener that logs senders")
    udp.add_argument("--port", type=int, default=9999)

    args = parser.parse_args(argv)

    if args.mode == "https":
        if bool(args.cert) != bool(args.key):
            parser.error("--cert and --key must be given together")
        run_https(args.port, args.cert, args.key)
    elif args.mode == "udp":
        run_udp(args.port)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(0)
