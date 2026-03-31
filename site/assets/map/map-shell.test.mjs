import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const shellHtml = readFileSync(new URL("./map-shell.html", import.meta.url), "utf8");

test("map shell windows are Datastar-driven for open and collapsed state", () => {
  assert.match(shellHtml, /layers: \{ expandedLayerIds: \[\], hoverFactsVisibleByLayer: \{\} \}/);
  assert.match(shellHtml, /id="fishymap-search-window"[\s\S]*data-show="\$_map_ui\.windowUi\.search\.open"/);
  assert.match(shellHtml, /id="fishymap-bookmarks-window"[\s\S]*data-show="\$_map_ui\.windowUi\.bookmarks\.open"/);
  assert.match(shellHtml, /id="fishymap-zone-info-window"[\s\S]*data-show="\$_map_ui\.windowUi\.zoneInfo\.open"/);
  assert.match(shellHtml, /id="fishymap-layers-window"[\s\S]*data-show="\$_map_ui\.windowUi\.layers\.open"/);
  assert.match(shellHtml, /id="fishymap-panel"[\s\S]*data-show="\$_map_ui\.windowUi\.settings\.open"/);
  assert.match(shellHtml, /id="fishymap-search-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.search\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-bookmarks-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.bookmarks\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-zone-info-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.zoneInfo\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-layers-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.layers\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-panel-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.settings\.collapsed"/);
});

test("map shell toolbar and status affordances derive from Datastar signals", () => {
  assert.match(
    shellHtml,
    /data-init="el\.closest\('#map-page-shell'\)\.dispatchEvent\(new CustomEvent\('fishymap-live-init', \{ bubbles: true, detail: \$ \}\)\)"/,
  );
  assert.doesNotMatch(shellHtml, /data-on:fishymap-signals-patch=/);
  assert.match(shellHtml, /data-window-toggle="search"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.search\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="search"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.search\.open"/);
  assert.match(shellHtml, /data-window-toggle="bookmarks"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.bookmarks\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="bookmarks"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.bookmarks\.open"/);
  assert.match(shellHtml, /data-window-toggle="settings"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.settings\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="settings"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.settings\.open"/);
  assert.match(shellHtml, /data-window-toggle="zone-info"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.zoneInfo\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="zone-info"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.zoneInfo\.open"/);
  assert.match(shellHtml, /data-window-toggle="layers"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.layers\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="layers"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.layers\.open"/);
  assert.match(shellHtml, /id="fishymap-ready-pill"[\s\S]*data-class:badge-success="\$_map_runtime\.ready"/);
  assert.match(shellHtml, /id="fishymap-diagnostics"[\s\S]*data-attr:open="\$_map_bridged\.ui\.diagnosticsOpen"/);
  assert.match(shellHtml, /id="fishymap-reset-view"[\s\S]*data-on:click="\$_map_actions\.resetViewToken = \(\$_map_actions\.resetViewToken \|\| 0\) \+ 1"/);
  assert.match(shellHtml, /id="fishymap-reset-ui"[\s\S]*data-on:click="\$_map_actions\.resetUiToken = \(\$_map_actions\.resetUiToken \|\| 0\) \+ 1"/);
});
