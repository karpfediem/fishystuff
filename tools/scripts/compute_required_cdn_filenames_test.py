#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from compute_required_cdn_filenames import Report, add_runtime_assets


class RuntimeAssetReportTests(unittest.TestCase):
    def test_runtime_source_maps_are_optional(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            cdn_root = Path(tmp_dir)
            map_dir = cdn_root / "map"
            map_dir.mkdir()
            (map_dir / "runtime-manifest.json").write_text(
                json.dumps({
                    "module": "fishystuff_ui_bevy.abc123.js",
                    "wasm": "fishystuff_ui_bevy_bg.def456.wasm",
                }),
                encoding="utf-8",
            )
            (map_dir / "fishystuff_ui_bevy.abc123.js").write_text("", encoding="utf-8")
            (map_dir / "fishystuff_ui_bevy_bg.def456.wasm").write_bytes(b"")

            report = Report(cdn_root)
            add_runtime_assets(report, cdn_root, "")

            payload = report.finalize()
            self.assertEqual(payload["summary"]["missing_local_count"], 0)
            self.assertNotIn("map_runtime_source_maps", payload["paths_by_group"])

    def test_runtime_source_maps_are_required_when_present(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            cdn_root = Path(tmp_dir)
            map_dir = cdn_root / "map"
            map_dir.mkdir()
            (map_dir / "runtime-manifest.json").write_text(
                json.dumps({
                    "module": "fishystuff_ui_bevy.abc123.js",
                    "wasm": "fishystuff_ui_bevy_bg.def456.wasm",
                }),
                encoding="utf-8",
            )
            (map_dir / "runtime-manifest.cache-key.json").write_text("{}", encoding="utf-8")
            (map_dir / "fishystuff_ui_bevy.abc123.js").write_text("", encoding="utf-8")
            (map_dir / "fishystuff_ui_bevy.abc123.js.map").write_text("", encoding="utf-8")
            (map_dir / "fishystuff_ui_bevy_bg.def456.wasm").write_bytes(b"")
            (map_dir / "fishystuff_ui_bevy_bg.def456.wasm.map").write_text("", encoding="utf-8")

            report = Report(cdn_root)
            add_runtime_assets(report, cdn_root, "cache-key")

            payload = report.finalize()
            self.assertEqual(payload["summary"]["missing_local_count"], 0)
            self.assertEqual(
                payload["paths_by_group"]["map_runtime_source_maps"],
                [
                    "map/fishystuff_ui_bevy.abc123.js.map",
                    "map/fishystuff_ui_bevy_bg.def456.wasm.map",
                ],
            )


if __name__ == "__main__":
    unittest.main()
