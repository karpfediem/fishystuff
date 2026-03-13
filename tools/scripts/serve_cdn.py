#!/usr/bin/env python3
from __future__ import annotations

import argparse
import functools
import http.server
import mimetypes
import socketserver
from pathlib import Path

DEFAULT_CACHE_CONTROL = "public, max-age=3600"
IMMUTABLE_CACHE_CONTROL = "public, max-age=31536000, immutable"
NO_STORE_CACHE_CONTROL = "no-store"


class ReusableThreadingTCPServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


class CdnHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, directory: str, cache_control: str, **kwargs):
        self._cache_control = cache_control
        super().__init__(*args, directory=directory, **kwargs)

    def end_headers(self) -> None:
        self.send_header("Cache-Control", self._cache_control_for_path())
        self.send_header("Access-Control-Allow-Origin", "*")
        super().end_headers()

    def _cache_control_for_path(self) -> str:
        path = self.path.split("?", 1)[0]
        name = Path(path).name
        if name == "runtime-manifest.json":
            return NO_STORE_CACHE_CONTROL
        if (
            name.startswith("fishystuff_ui_bevy.")
            and name.endswith(".js")
            or name.startswith("fishystuff_ui_bevy_bg.")
            and name.endswith(".wasm")
        ):
            return IMMUTABLE_CACHE_CONTROL
        return self._cache_control

    def guess_type(self, path: str) -> str:
        guessed = super().guess_type(path)
        if guessed != "application/octet-stream":
            return guessed
        if path.endswith(".wasm"):
            return "application/wasm"
        if path.endswith(".geojson"):
            return "application/geo+json"
        if path.endswith(".json"):
            return "application/json"
        return guessed


def main() -> None:
    parser = argparse.ArgumentParser(description="Serve the local CDN staging tree with cache headers.")
    parser.add_argument("--root", default="data/cdn/public", help="CDN staging root to serve")
    parser.add_argument("--host", default="127.0.0.1", help="Bind host")
    parser.add_argument("--port", type=int, default=4040, help="Bind port")
    parser.add_argument(
        "--cache-control",
        default=DEFAULT_CACHE_CONTROL,
        help="Cache-Control header value",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    if not root.is_dir():
        raise SystemExit(f"CDN root does not exist: {root}")

    mimetypes.add_type("application/wasm", ".wasm")
    mimetypes.add_type("application/geo+json", ".geojson")

    handler = functools.partial(
        CdnHandler,
        directory=str(root),
        cache_control=args.cache_control,
    )

    with ReusableThreadingTCPServer((args.host, args.port), handler) as httpd:
        print(f"Serving {root} at http://{args.host}:{args.port}/")
        httpd.serve_forever()


if __name__ == "__main__":
    main()
