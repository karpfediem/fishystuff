#!/usr/bin/env python3
import os
import socket
import sys


def notify_socket_path() -> str:
    raw = os.environ.get("NOTIFY_SOCKET", "").strip()
    if not raw:
        return ""
    if raw.startswith("@"):
        return "\0" + raw[1:]
    return raw


def send_message(lines: list[str]) -> int:
    target = notify_socket_path()
    if not target:
        return 0

    payload = "\n".join(line for line in lines if line).encode()
    if not payload:
        return 0

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    try:
        sock.connect(target)
        sock.sendall(payload)
    finally:
        sock.close()
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: devenv_notify.py <ready|status|reloading|stopping> [message]", file=sys.stderr)
        return 2

    command = argv[1]
    message = " ".join(argv[2:]).strip()

    if command == "ready":
        lines = ["READY=1"]
        if message:
            lines.append(f"STATUS={message}")
        return send_message(lines)
    if command == "status":
        return send_message([f"STATUS={message}"] if message else [])
    if command == "reloading":
        lines = ["RELOADING=1"]
        if message:
            lines.append(f"STATUS={message}")
        return send_message(lines)
    if command == "stopping":
        lines = ["STOPPING=1"]
        if message:
            lines.append(f"STATUS={message}")
        return send_message(lines)

    print(f"unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
