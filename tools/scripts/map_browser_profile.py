#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from map_browser_smoke import (
    DevToolsClient,
    build_state_expression,
    pick_free_tcp_port,
    probe_page,
    tail_lines,
    wait_for_page_target,
    write_output,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a headless browser profiling scenario against the local /map page."
    )
    parser.add_argument(
        "scenario",
        choices=[
            "load_map",
            "vector_region_groups_enable",
            "vector_region_groups_dom_toggle",
            "zone_mask_hover_sweep",
        ],
        help="Integrated browser profiling scenario to run.",
    )
    parser.add_argument(
        "--url",
        default="http://127.0.0.1:1990/map/",
        help="Map page URL to load.",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=45.0,
        help="Maximum total runtime for the browser profiling scenario.",
    )
    parser.add_argument(
        "--poll-interval-seconds",
        type=float,
        default=0.25,
        help="Polling interval while waiting for bridge readiness.",
    )
    parser.add_argument(
        "--capture-frames",
        type=int,
        help="Frames to capture after scenario setup. Defaults depend on the scenario.",
    )
    parser.add_argument(
        "--output-json",
        type=Path,
        help="Optional path for the machine-readable profiling report.",
    )
    parser.add_argument(
        "--chromium-binary",
        default=os.environ.get("CHROMIUM_BINARY", "chromium"),
        help="Chromium binary to execute.",
    )
    return parser.parse_args()


def scenario_capture_frames(scenario: str, capture_frames: int | None) -> int:
    if capture_frames is not None:
        return max(0, capture_frames)
    if scenario == "load_map":
        return 0
    if scenario == "vector_region_groups_enable":
        return 180
    if scenario == "vector_region_groups_dom_toggle":
        return 180
    if scenario == "zone_mask_hover_sweep":
        return 120
    return 0


def wait_for_ready(
    client: DevToolsClient,
    proc: subprocess.Popen[bytes],
    timeout_seconds: float,
    poll_interval_seconds: float,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    state_expr = build_state_expression()
    last_state: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(
                f"chromium exited before the map reached ready (code {proc.returncode})"
            )
        state = client.evaluate_json(state_expr)
        last_state = state
        if state.get("errorVisible"):
            message = state.get("errorMessage") or "renderer error overlay became visible"
            raise RuntimeError(message)
        fish_status = str((state.get("statuses") or {}).get("fishStatus") or "")
        fish_pending = fish_status.strip().lower() == "fish: pending"
        fish_ready = int(state.get("fishCount") or 0) > 0
        if state.get("ready") and fish_ready:
            return state
        if state.get("ready") and fish_status and not fish_pending and not fish_ready:
            raise RuntimeError(
                f"bridge became ready but fish catalog is unusable ({fish_status})"
            )
        time.sleep(poll_interval_seconds)
    if last_state and not last_state.get("hasBridge"):
        raise RuntimeError("timeout waiting for FishyMapBridge to initialize")
    if last_state and last_state.get("ready"):
        raise RuntimeError("timeout waiting for fish catalog after ready")
    raise RuntimeError("timeout waiting for FishyMapBridge.ready")


def wait_frames_js(frame_count: int) -> str:
    return f"""
await new Promise((resolve) => {{
  let remaining = {frame_count};
  if (remaining <= 0) {{
    resolve({{ completedFrames: 0, timedOut: false }});
    return;
  }}
  let completedFrames = 0;
  let finished = false;
  const finish = (timedOut) => {{
    if (finished) {{
      return;
    }}
    finished = true;
    globalThis.clearTimeout(timeoutId);
    resolve({{ completedFrames, timedOut }});
  }};
  const timeoutId = globalThis.setTimeout(
    () => finish(true),
    Math.max(5000, {frame_count} * 120),
  );
  const tick = () => {{
    completedFrames += 1;
    remaining -= 1;
    if (remaining <= 0) {{
      finish(false);
      return;
    }}
    globalThis.requestAnimationFrame(tick);
  }};
  globalThis.requestAnimationFrame(tick);
}});
""".strip()


def wait_for_raster_idle_js() -> str:
    return """
const waitForRasterIdle = async () => {
  const deadline = performance.now() + 10000;
  while (performance.now() < deadline) {
    const state = typeof bridge.refreshCurrentStateNow === "function"
      ? bridge.refreshCurrentStateNow()
      : bridge.getCurrentState();
    const layers = Array.isArray(state?.catalog?.layers) ? state.catalog.layers : [];
    const busyLayers = layers.filter((layer) =>
      layer?.kind === "tiled-raster" &&
      ((Number(layer?.pendingCount) || 0) > 0 || (Number(layer?.inflightCount) || 0) > 0),
    );
    const busyLayerIds = busyLayers.map((layer) => String(layer?.layerId || ""));
    const busyTiles = busyLayers.reduce(
      (sum, layer) => sum + (Number(layer?.pendingCount) || 0) + (Number(layer?.inflightCount) || 0),
      0,
    );
    if (busyTiles <= 0) {
      return { timedOut: false, busyLayers: busyLayers.length, busyLayerIds, busyTiles, state };
    }
    await new Promise((resolve) => globalThis.setTimeout(resolve, 100));
  }
  const state = typeof bridge.refreshCurrentStateNow === "function"
    ? bridge.refreshCurrentStateNow()
    : bridge.getCurrentState();
  const layers = Array.isArray(state?.catalog?.layers) ? state.catalog.layers : [];
  const busyLayers = layers.filter((layer) =>
    layer?.kind === "tiled-raster" &&
    ((Number(layer?.pendingCount) || 0) > 0 || (Number(layer?.inflightCount) || 0) > 0),
  );
  const busyLayerIds = busyLayers.map((layer) => String(layer?.layerId || ""));
  const busyTiles = busyLayers.reduce(
    (sum, layer) => sum + (Number(layer?.pendingCount) || 0) + (Number(layer?.inflightCount) || 0),
    0,
  );
  return { timedOut: true, busyLayers: busyLayers.length, busyLayerIds, busyTiles, state };
};
""".strip()


def build_profile_expression(scenario: str, capture_frames: int) -> str:
    wait_frames = wait_frames_js(capture_frames)
    wait_for_raster_idle = wait_for_raster_idle_js()
    if scenario == "load_map":
        return f"""
(async () => {{
  const bridge = globalThis.window?.FishyMapBridge ?? null;
  if (!bridge?.getPerformanceSnapshot) {{
    throw new Error("FishyMapBridge profiling API is unavailable");
  }}
  const frameWait = {wait_frames};
  const report = bridge.getPerformanceSnapshot();
  report.browser_action = {{
    capture_frames_target: {capture_frames},
    completed_frames: frameWait.completedFrames,
    frame_wait_timed_out: frameWait.timedOut,
  }};
  return report;
}})()
""".strip()
    if scenario == "vector_region_groups_enable":
        return f"""
(async () => {{
  const bridge = globalThis.window?.FishyMapBridge ?? null;
  if (!bridge?.resetPerformanceSnapshot || !bridge?.setState || !bridge?.getCurrentState) {{
    throw new Error("FishyMapBridge profiling API is unavailable");
  }}
  {wait_for_raster_idle}
  const settle = await waitForRasterIdle();
  const state = settle.state || bridge.getCurrentState();
  const layers = Array.isArray(state?.catalog?.layers) ? state.catalog.layers : [];
  const targetLayer =
    layers.find((layer) => layer?.layerId === "region_groups") ||
    layers.find((layer) => layer?.kind === "vector-geojson" && layer?.visible !== true) ||
    layers.find((layer) => layer?.kind === "vector-geojson") ||
    null;
  if (!targetLayer?.layerId) {{
    throw new Error("No vector layer is available for profiling");
  }}
  const visibleLayerIds = Array.isArray(state?.filters?.layerIdsVisible)
    ? state.filters.layerIdsVisible.slice()
    : layers.filter((layer) => layer?.visible === true).map((layer) => layer.layerId);
  if (!visibleLayerIds.includes(targetLayer.layerId)) {{
    visibleLayerIds.push(targetLayer.layerId);
  }}
  bridge.resetPerformanceSnapshot({{
    scenario: "vector_region_groups_enable",
    warmupFrames: 0,
  }});
  bridge.setState({{
    filters: {{
      layerIdsVisible: visibleLayerIds,
    }},
  }});
  const frameWait = {wait_frames};
  const report = bridge.getPerformanceSnapshot();
  report.browser_action = {{
    target_layer_id: targetLayer.layerId,
    pre_capture_raster_idle_timed_out: settle.timedOut,
    pre_capture_busy_raster_layers: settle.busyLayers,
    pre_capture_busy_raster_layer_ids: settle.busyLayerIds,
    pre_capture_busy_raster_tiles: settle.busyTiles,
    capture_frames_target: {capture_frames},
    completed_frames: frameWait.completedFrames,
    frame_wait_timed_out: frameWait.timedOut,
  }};
  return report;
}})()
""".strip()
    if scenario == "vector_region_groups_dom_toggle":
        return f"""
(async () => {{
  const bridge = globalThis.window?.FishyMapBridge ?? null;
  if (!bridge?.resetPerformanceSnapshot || !bridge?.getPerformanceSnapshot) {{
    throw new Error("FishyMapBridge profiling API is unavailable");
  }}
  {wait_for_raster_idle}
  const waitForLayerToggle = async () => {{
    const deadline = performance.now() + 10000;
    while (performance.now() < deadline) {{
      const button = document.querySelector('button[data-layer-visibility="region_groups"]');
      if (button) {{
        return button;
      }}
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }}
    throw new Error("region_groups visibility button not found");
  }};
  const button = await waitForLayerToggle();
  const settle = await waitForRasterIdle();
  bridge.resetPerformanceSnapshot({{
    scenario: "vector_region_groups_dom_toggle",
    warmupFrames: 0,
  }});
  button.click();
  const frameWait = {wait_frames};
  const report = bridge.getPerformanceSnapshot();
  report.browser_action = {{
    target_layer_id: "region_groups",
    pre_capture_raster_idle_timed_out: settle.timedOut,
    pre_capture_busy_raster_layers: settle.busyLayers,
    pre_capture_busy_raster_layer_ids: settle.busyLayerIds,
    pre_capture_busy_raster_tiles: settle.busyTiles,
    capture_frames_target: {capture_frames},
    completed_frames: frameWait.completedFrames,
    frame_wait_timed_out: frameWait.timedOut,
    trigger: "dom_click",
  }};
  return report;
}})()
""".strip()
    if scenario == "zone_mask_hover_sweep":
        return f"""
(async () => {{
  const bridge = globalThis.window?.FishyMapBridge ?? null;
  if (!bridge?.resetPerformanceSnapshot || !bridge?.getPerformanceSnapshot) {{
    throw new Error("FishyMapBridge profiling API is unavailable");
  }}
  {wait_for_raster_idle}
  const canvas = document.getElementById("bevy");
  if (!canvas) {{
    throw new Error("map canvas not found");
  }}
  const rect = canvas.getBoundingClientRect();
  if (!rect || !(rect.width > 0) || !(rect.height > 0)) {{
    throw new Error("map canvas has no measurable size");
  }}
  const settle = await waitForRasterIdle();
  bridge.resetPerformanceSnapshot({{
    scenario: "zone_mask_hover_sweep",
    warmupFrames: 0,
  }});
  const dispatchHover = (fx, fy) => {{
    canvas.dispatchEvent(new PointerEvent("pointermove", {{
      bubbles: true,
      cancelable: true,
      clientX: rect.left + rect.width * fx,
      clientY: rect.top + rect.height * fy,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      buttons: 0,
    }}));
  }};
  const hoverPoints = [
    [0.20, 0.24],
    [0.33, 0.30],
    [0.46, 0.36],
    [0.59, 0.42],
    [0.72, 0.48],
  ];
  for (const [fx, fy] of hoverPoints) {{
    dispatchHover(fx, fy);
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  }}
  const frameWait = {wait_frames};
  const report = bridge.getPerformanceSnapshot();
  report.browser_action = {{
    pre_capture_raster_idle_timed_out: settle.timedOut,
    pre_capture_busy_raster_layers: settle.busyLayers,
    pre_capture_busy_raster_layer_ids: settle.busyLayerIds,
    pre_capture_busy_raster_tiles: settle.busyTiles,
    hover_points: hoverPoints.length,
    capture_frames_target: {capture_frames},
    completed_frames: frameWait.completedFrames,
    frame_wait_timed_out: frameWait.timedOut,
  }};
  return report;
}})()
""".strip()
    raise RuntimeError(f"unsupported profiling scenario: {scenario}")


def top_spans(report: dict[str, Any], limit: int = 8) -> list[tuple[str, float]]:
    spans = report.get("named_spans") or {}
    ranked: list[tuple[str, float]] = []
    for name, stats in spans.items():
        try:
            total_ms = float((stats or {}).get("total_ms") or 0.0)
        except (TypeError, ValueError):
            total_ms = 0.0
        ranked.append((name, total_ms))
    ranked.sort(key=lambda item: item[1], reverse=True)
    return ranked[:limit]


def print_summary(report: dict[str, Any], output_json: Path | None) -> None:
    frame = report.get("frame_time_ms") or {}
    print(
        (
            f"PASS browser profile scenario={report.get('scenario')} "
            f"frames={report.get('frames')} warmup={report.get('warmup_frames')} "
            f"frame_avg_ms={float(frame.get('avg') or 0.0):.3f} "
            f"p95_ms={float(frame.get('p95') or 0.0):.3f}"
        ),
        file=sys.stderr,
    )
    for name, total_ms in top_spans(report):
        print(f"  {name} total_ms={total_ms:.3f}", file=sys.stderr)
    if output_json is not None:
        print(f"  json={output_json}", file=sys.stderr)


def main() -> int:
    args = parse_args()
    capture_frames = scenario_capture_frames(args.scenario, args.capture_frames)
    probe_page(args.url)

    devtools_port = pick_free_tcp_port()
    with tempfile.TemporaryDirectory(
        prefix="fishystuff-map-profile-", ignore_cleanup_errors=True
    ) as temp_dir:
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

        report: dict[str, Any] = {
            "scenario": args.scenario,
            "url": args.url,
            "ok": False,
            "reason": "not-started",
            "frame_time_ms": {
                "avg": 0,
                "p50": 0,
                "p95": 0,
                "p99": 0,
                "max": 0,
            },
            "named_spans": {},
            "counters": {},
        }

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
                    ready_state = wait_for_ready(
                        client, proc, args.timeout_seconds, args.poll_interval_seconds
                    )
                    profile_expr = build_profile_expression(args.scenario, capture_frames)
                    profile_report = client.evaluate_json(
                        profile_expr,
                        await_promise=True,
                        timeout_seconds=args.timeout_seconds,
                    )
                    if not isinstance(profile_report, dict):
                        raise RuntimeError("browser profiling expression returned no JSON object")
                    report = {
                        **profile_report,
                        "scenario": profile_report.get("scenario") or args.scenario,
                        "url": args.url,
                        "ok": True,
                        "reason": "browser profiling scenario completed",
                        "bridge_state": ready_state,
                    }
                finally:
                    client.close()
            except Exception as exc:
                report["reason"] = str(exc)
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

        report["chromium_stderr_tail"] = tail_lines(stderr_path)
        write_output(args.output_json, report)
        if report.get("ok"):
            print_summary(report, args.output_json)
            return 0

        print(
            f"FAIL browser profile scenario={args.scenario}: {report.get('reason')}",
            file=sys.stderr,
        )
        if report.get("chromium_stderr_tail"):
            print("  chromium-stderr-tail:", file=sys.stderr)
            for line in report["chromium_stderr_tail"]:
                print(f"    {line}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
