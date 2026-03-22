#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a headless browser smoke check against the local /map page."
    )
    parser.add_argument(
        "--url",
        default="http://127.0.0.1:1990/map/",
        help="Map page URL to load.",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=30.0,
        help="Maximum time to wait for FishyMapBridge.ready.",
    )
    parser.add_argument(
        "--poll-interval-seconds",
        type=float,
        default=0.5,
        help="Polling interval for bridge state checks.",
    )
    parser.add_argument(
        "--output-json",
        type=Path,
        help="Optional path for a machine-readable JSON result.",
    )
    parser.add_argument(
        "--chromium-binary",
        default=os.environ.get("CHROMIUM_BINARY", "chromium"),
        help="Chromium binary to execute.",
    )
    return parser.parse_args()


def pick_free_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        sock.listen(1)
        return int(sock.getsockname()[1])


def probe_page(url: str) -> None:
    request = urllib.request.Request(url, method="GET")
    with urllib.request.urlopen(request, timeout=5.0) as response:
        if response.status != 200:
            raise RuntimeError(f"{url} returned HTTP {response.status}")


def fetch_json(url: str) -> Any:
    with urllib.request.urlopen(url, timeout=2.0) as response:
        return json.load(response)


def tail_lines(path: Path, limit: int = 40) -> list[str]:
    if not path.exists():
        return []
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return []
    lines = [line.rstrip() for line in text.splitlines() if line.strip()]
    return lines[-limit:]


class DevToolsClient:
    def __init__(self, websocket_url: str) -> None:
        parsed = urllib.parse.urlparse(websocket_url)
        host = parsed.hostname or "127.0.0.1"
        port = parsed.port or 80
        path = parsed.path or "/"
        self.sock = socket.create_connection((host, port), timeout=10.0)
        self.sock.settimeout(10.0)
        self.next_id = 1
        self._handshake(host, port, path)
        self.send("Runtime.enable")

    def _handshake(self, host: str, port: int, path: str) -> None:
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: {host}:{port}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(request.encode("utf-8"))
        response = self._recv_http_headers()
        if (
            "101 WebSocket Protocol Handshake" not in response
            and "101 Switching Protocols" not in response
        ):
            raise RuntimeError(f"DevTools websocket handshake failed:\n{response}")

    def _recv_http_headers(self) -> str:
        data = b""
        while b"\r\n\r\n" not in data:
            chunk = self.sock.recv(4096)
            if not chunk:
                break
            data += chunk
        return data.decode("utf-8", "replace")

    def _send_frame(self, payload: str) -> None:
        data = payload.encode("utf-8")
        header = bytearray([0x81])
        length = len(data)
        if length < 126:
            header.append(0x80 | length)
        elif length < (1 << 16):
            header.append(0x80 | 126)
            header.extend(struct.pack("!H", length))
        else:
            header.append(0x80 | 127)
            header.extend(struct.pack("!Q", length))
        mask = os.urandom(4)
        header.extend(mask)
        header.extend(bytes(byte ^ mask[idx % 4] for idx, byte in enumerate(data)))
        self.sock.sendall(header)

    def _recv_frame(self) -> str | None:
        first = self.sock.recv(2)
        if len(first) < 2:
            return None
        b1, b2 = first
        opcode = b1 & 0x0F
        masked = (b2 & 0x80) != 0
        length = b2 & 0x7F
        if length == 126:
            length = struct.unpack("!H", self.sock.recv(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self.sock.recv(8))[0]
        mask = self.sock.recv(4) if masked else b""
        data = b""
        while len(data) < length:
            chunk = self.sock.recv(length - len(data))
            if not chunk:
                break
            data += chunk
        if masked:
            data = bytes(byte ^ mask[idx % 4] for idx, byte in enumerate(data))
        if opcode == 0x8:
            return None
        if opcode == 0x9:
            self.sock.sendall(bytes([0x8A, 0x00]))
            return self._recv_frame()
        return data.decode("utf-8", "replace")

    def send(self, method: str, params: dict[str, Any] | None = None) -> int:
        message_id = self.next_id
        self.next_id += 1
        payload = {"id": message_id, "method": method}
        if params:
            payload["params"] = params
        self._send_frame(json.dumps(payload))
        return message_id

    def wait_for_result(
        self, message_id: int, timeout_seconds: float | None = None
    ) -> dict[str, Any]:
        deadline = time.monotonic() + timeout_seconds if timeout_seconds is not None else None
        while True:
            if deadline is not None:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError(f"timed out waiting for DevTools result {message_id}")
                self.sock.settimeout(max(0.1, min(1.0, remaining)))
            try:
                message = self._recv_frame()
            except TimeoutError:
                continue
            if message is None:
                raise RuntimeError("DevTools websocket closed")
            payload = json.loads(message)
            if payload.get("id") == message_id:
                return payload

    def evaluate_json(
        self,
        expression: str,
        await_promise: bool = False,
        timeout_seconds: float | None = None,
    ) -> Any:
        message_id = self.send(
            "Runtime.evaluate",
            {
                "expression": expression,
                "returnByValue": True,
                "awaitPromise": await_promise,
            },
        )
        payload = self.wait_for_result(message_id, timeout_seconds=timeout_seconds)
        if "error" in payload:
            raise RuntimeError(str(payload["error"]))
        result = payload.get("result", {}).get("result", {})
        if "value" not in result:
            raise RuntimeError(f"Runtime.evaluate returned no value: {payload}")
        value = result["value"]
        if isinstance(value, str):
            return json.loads(value)
        return value

    def close(self) -> None:
        try:
            self.sock.close()
        except OSError:
            pass


def wait_for_page_target(devtools_port: int, target_url: str, deadline: float) -> dict[str, Any]:
    normalized = target_url.rstrip("/")
    endpoint = f"http://127.0.0.1:{devtools_port}/json/list"
    while time.monotonic() < deadline:
        try:
            targets = fetch_json(endpoint)
        except (urllib.error.URLError, TimeoutError, OSError, json.JSONDecodeError):
            time.sleep(0.2)
            continue
        for target in targets:
            if target.get("type") != "page":
                continue
            page_url = str(target.get("url", "")).rstrip("/")
            if page_url == normalized:
                return target
        time.sleep(0.2)
    raise RuntimeError(f"timed out waiting for DevTools target for {target_url}")


def build_state_expression() -> str:
    return """
JSON.stringify((() => {
  const bridge = globalThis.window?.FishyMapBridge ?? null;
  const state = bridge?.getCurrentState?.() ?? null;
  const readyPill = document.getElementById("fishymap-ready-pill")?.textContent?.trim() ?? null;
  const errorOverlay = document.getElementById("fishymap-error-overlay");
  const errorMessage =
    document.getElementById("fishymap-error-message")?.textContent?.trim() ?? null;
  return {
    hasBridge: Boolean(bridge),
    ready: Boolean(state?.ready),
    readyPill,
    errorVisible: Boolean(errorOverlay && !errorOverlay.hidden),
    errorMessage,
    statuses: state?.statuses ?? null,
    layerCount: Array.isArray(state?.catalog?.layers) ? state.catalog.layers.length : 0,
    patchCount: Array.isArray(state?.catalog?.patches) ? state.catalog.patches.length : 0,
    fishCount: Array.isArray(state?.catalog?.fish) ? state.catalog.fish.length : 0,
    viewMode: state?.view?.viewMode ?? null,
  };
})())
""".strip()


def write_output(path: Path | None, payload: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def print_summary(result: dict[str, Any]) -> None:
    bridge = result.get("bridge_state") or {}
    statuses = bridge.get("statuses") or {}
    status_bits = [
        f"ready={bridge.get('ready')}",
        f"ready_pill={bridge.get('readyPill')!r}",
        f"meta={statuses.get('metaStatus')!r}",
        f"layers={statuses.get('layersStatus')!r}",
        f"zones={statuses.get('zonesStatus')!r}",
        f"points={statuses.get('pointsStatus')!r}",
        f"fish={statuses.get('fishStatus')!r}",
        f"fish_count={bridge.get('fishCount')}",
    ]
    prefix = "PASS" if result.get("ok") else "FAIL"
    print(
        f"{prefix} map browser smoke in {result['elapsed_ms']:.1f} ms: {result['reason']}",
        file=sys.stderr,
    )
    print("  " + " ".join(status_bits), file=sys.stderr)
    if result.get("output_json"):
        print(f"  json={result['output_json']}", file=sys.stderr)
    if not result.get("ok") and result.get("chromium_stderr_tail"):
        print("  chromium-stderr-tail:", file=sys.stderr)
        for line in result["chromium_stderr_tail"]:
            print(f"    {line}", file=sys.stderr)


def main() -> int:
    args = parse_args()
    start = time.monotonic()
    probe_page(args.url)

    devtools_port = pick_free_tcp_port()
    result: dict[str, Any] = {
        "ok": False,
        "reason": "not-started",
        "url": args.url,
        "timeout_seconds": args.timeout_seconds,
        "elapsed_ms": 0.0,
        "bridge_state": None,
        "output_json": str(args.output_json) if args.output_json else None,
        "chromium_stderr_tail": [],
    }

    with tempfile.TemporaryDirectory(prefix="fishystuff-map-smoke-") as temp_dir:
        temp_root = Path(temp_dir)
        stderr_path = temp_root / "chromium.stderr.log"
        stdout_path = temp_root / "chromium.stdout.log"
        chromium_args = [
            args.chromium_binary,
            "--headless",
            f"--remote-debugging-port={devtools_port}",
            "--use-angle=swiftshader-webgl",
            "--enable-webgl",
            "--enable-unsafe-swiftshader",
            "--ignore-gpu-blocklist",
            "--window-size=1280,720",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-background-networking",
            "--disable-background-timer-throttling",
            "--disable-renderer-backgrounding",
            f"--user-data-dir={temp_root / 'profile'}",
            args.url,
        ]

        with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
            proc = subprocess.Popen(
                chromium_args,
                stdout=stdout_file,
                stderr=stderr_file,
            )

            try:
                deadline = time.monotonic() + args.timeout_seconds
                target = wait_for_page_target(devtools_port, args.url, deadline)
                client = DevToolsClient(str(target["webSocketDebuggerUrl"]))
                try:
                    state_expr = build_state_expression()
                    last_state: dict[str, Any] | None = None
                    while time.monotonic() < deadline:
                        if proc.poll() is not None:
                            raise RuntimeError(
                                f"chromium exited before the map reached ready (code {proc.returncode})"
                            )
                        state = client.evaluate_json(state_expr)
                        last_state = state
                        result["bridge_state"] = state
                        if state.get("errorVisible"):
                            result["reason"] = (
                                state.get("errorMessage")
                                or "renderer error overlay became visible"
                            )
                            break
                        fish_status = str((state.get("statuses") or {}).get("fishStatus") or "")
                        fish_pending = fish_status.strip().lower() == "fish: pending"
                        fish_ready = int(state.get("fishCount") or 0) > 0
                        if state.get("ready") and fish_ready:
                            result["ok"] = True
                            result["reason"] = "bridge reached ready with fish catalog"
                            break
                        if state.get("ready") and fish_status and not fish_pending and not fish_ready:
                            result["reason"] = (
                                f"bridge became ready but fish catalog is unusable ({fish_status})"
                            )
                            break
                        time.sleep(args.poll_interval_seconds)
                    else:
                        bridge_reason = "timeout waiting for FishyMapBridge.ready"
                        if last_state and not last_state.get("hasBridge"):
                            bridge_reason = "timeout waiting for FishyMapBridge to initialize"
                        elif last_state and last_state.get("ready"):
                            bridge_reason = "timeout waiting for fish catalog after ready"
                        result["reason"] = bridge_reason
                finally:
                    client.close()
            except Exception as exc:
                result["reason"] = str(exc)
            finally:
                try:
                    proc.terminate()
                except OSError:
                    pass
                try:
                    proc.wait(timeout=5.0)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5.0)

        result["elapsed_ms"] = (time.monotonic() - start) * 1000.0
        result["chromium_stderr_tail"] = tail_lines(stderr_path)
        write_output(args.output_json, result)
        print_summary(result)
        return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
