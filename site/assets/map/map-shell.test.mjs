import { test } from "bun:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { installMapTestI18n } from "./test-i18n.js";

installMapTestI18n();

const shellHtml = readFileSync(new URL("./map-shell.html", import.meta.url), "utf8");

test("map shell windows are Datastar-driven for open and collapsed state", () => {
  assert.match(
    shellHtml,
    /settings: \{ open: false, collapsed: false, x: null, y: null, autoAdjustView: true, normalizeRates: true \}/,
  );
  assert.match(shellHtml, /saveMapPresetToken: 0/);
  assert.match(shellHtml, /discardMapPresetToken: 0/);
  assert.match(shellHtml, /_user_presets: \{[\s\S]*'map-presets': \{[\s\S]*canSave: false,[\s\S]*canDiscard: false/);
  assert.match(shellHtml, /layers: \{ open: true, collapsed: false, x: null, y: null \}/);
  assert.match(shellHtml, /layers: \{ expandedLayerIds: \[\], hoverFactsVisibleByLayer: \{\} \}/);
  assert.match(shellHtml, /id="fishymap-search-window"[\s\S]*data-show="\$_map_ui\.windowUi\.search\.open"/);
  assert.match(
    shellHtml,
    /<fishymap-search-panel[\s\S]*id="fishymap-search-panel"[\s\S]*class="not-prose"[\s\S]*data-attr:data-search-state="JSON\.stringify\(\$_map_ui\.search\)"[\s\S]*><\/fishymap-search-panel>/,
  );
  assert.match(shellHtml, /id="fishymap-bookmarks-window"[\s\S]*data-show="\$_map_ui\.windowUi\.bookmarks\.open"/);
  assert.match(shellHtml, /<fishymap-bookmark-panel id="fishymap-bookmark-panel" class="not-prose"><\/fishymap-bookmark-panel>/);
  assert.match(shellHtml, /id="fishymap-zone-info-window"[\s\S]*data-show="\$_map_ui\.windowUi\.zoneInfo\.open"/);
  assert.match(shellHtml, /id="fishymap-zone-info-title"[^>]*data-i18n-text="map\.info\.window_title"[^>]*><\/span>/);
  assert.match(shellHtml, /id="fishymap-layers-window"[\s\S]*data-show="\$_map_ui\.windowUi\.layers\.open"/);
  assert.match(shellHtml, /id="fishymap-panel"[\s\S]*data-show="\$_map_ui\.windowUi\.settings\.open"/);
  assert.match(shellHtml, /<fishymap-hover-tooltip id="fishymap-hover-tooltip" class="card card-border bg-base-100 not-prose" hidden><\/fishymap-hover-tooltip>/);
  assert.match(
    shellHtml,
    /<fishymap-info-panel[\s\S]*id="fishymap-info-panel"[\s\S]*class="space-y-3 not-prose"[\s\S]*data-normalize-rates="true"[\s\S]*data-attr:data-normalize-rates="\$_map_ui\.windowUi\.settings\.normalizeRates \? 'true' : 'false'"[\s\S]*><\/fishymap-info-panel>/,
  );
  assert.match(shellHtml, /<fishymap-layer-panel id="fishymap-layer-panel" class="not-prose"><\/fishymap-layer-panel>/);
  assert.doesNotMatch(shellHtml, /<fishymap-patch-picker\b/);
  assert.match(shellHtml, /<fishymap-window-manager id="fishymap-window-manager" hidden><\/fishymap-window-manager>/);
  assert.match(shellHtml, /id="fishymap-search-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.search\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-bookmarks-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.bookmarks\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-zone-info-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.zoneInfo\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-layers-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.layers\.collapsed"/);
  assert.match(shellHtml, /id="fishymap-panel-body"[\s\S]*data-show="!\$_map_ui\.windowUi\.settings\.collapsed"/);
});

test("map shell toolbar and status affordances derive from Datastar signals", () => {
  assert.match(
    shellHtml,
    /data-init="const shell = el\.closest\('#map-page-shell'\); shell\.__fishymapInitialSignals = \$; shell\.dispatchEvent\(new CustomEvent\('fishymap-live-init', \{ bubbles: true, detail: \$ \}\)\)"/,
  );
  assert.match(
    shellHtml,
    /id="map-page-shell"[\s\S]*data-on-signal-patch-filter="\{include: \/\^_\(\?:map_\[\^\.\]\+\|shared_fish\)\(\?:\\\.\|\$\)\/\}"[\s\S]*data-on-signal-patch="el\.dispatchEvent\(new CustomEvent\('fishymap:signal-patched', \{ bubbles: true, detail: patch \}\)\)"/,
  );
  const mapSignalPatchFilter = /^_(?:map_[^.]+|shared_fish)(?:\.|$)/;
  assert.equal(mapSignalPatchFilter.test("_map_ui.windowUi.settings.normalizeRates"), true);
  assert.equal(mapSignalPatchFilter.test("_map_actions.resetViewToken"), true);
  assert.equal(mapSignalPatchFilter.test("_map_bridged.ui.showPoints"), true);
  assert.equal(mapSignalPatchFilter.test("_shared_fish.caughtIds"), true);
  assert.equal(mapSignalPatchFilter.test("_not_map_ui.windowUi.settings.open"), false);
  assert.doesNotMatch(shellHtml, /data-on:fishymap-signals-patch=/);
  assert.match(shellHtml, /data-window-toggle="search"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.search\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="search"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.search\.open"/);
  assert.match(shellHtml, /data-window-toggle="search"[\s\S]*data-i18n-attr-aria-label="map\.toolbar\.search"/);
  assert.match(shellHtml, /data-window-toggle="bookmarks"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.bookmarks\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="bookmarks"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.bookmarks\.open"/);
  assert.match(shellHtml, /data-window-toggle="bookmarks"[\s\S]*data-i18n-attr-aria-label="map\.toolbar\.bookmarks"/);
  assert.match(shellHtml, /data-window-toggle="settings"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.settings\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="settings"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.settings\.open"/);
  assert.match(shellHtml, /data-window-toggle="settings"[\s\S]*data-i18n-attr-aria-label="map\.toolbar\.settings"/);
  assert.match(shellHtml, /data-window-toggle="zone-info"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.zoneInfo\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="zone-info"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.zoneInfo\.open"/);
  assert.match(shellHtml, /data-window-toggle="zone-info"[\s\S]*#fishy-inspect-fill/);
  assert.match(shellHtml, /id="fishymap-zone-info-title-icon"[\s\S]*#fishy-inspect-fill/);
  assert.match(shellHtml, /data-window-toggle="layers"[\s\S]*data-attr:data-open="\$_map_ui\.windowUi\.layers\.open \? 'true' : 'false'"/);
  assert.match(shellHtml, /data-window-toggle="layers"[\s\S]*data-attr:aria-pressed="\$_map_ui\.windowUi\.layers\.open"/);
  assert.match(shellHtml, /data-window-toggle="layers"[\s\S]*data-i18n-attr-aria-label="map\.toolbar\.layers"/);
  assert.match(shellHtml, /id="fishymap-ready-pill"[\s\S]*data-class:badge-success="\$_map_runtime\.ready"/);
  assert.match(shellHtml, /id="fishymap-diagnostics"[\s\S]*data-attr:open="\$_map_bridged\.ui\.diagnosticsOpen"/);
  assert.match(shellHtml, /id="fishymap-reset-view"[\s\S]*data-on:click="\$_map_actions\.resetViewToken = \(\$_map_actions\.resetViewToken \|\| 0\) \+ 1"/);
  assert.match(shellHtml, /id="fishymap-reset-ui"[\s\S]*data-on:click="\$_map_actions\.resetUiToken = \(\$_map_actions\.resetUiToken \|\| 0\) \+ 1"/);
  assert.match(shellHtml, /<fishy-preset-manager[\s\S]*data-preset-collection="map-presets"[\s\S]*><\/fishy-preset-manager>[\s\S]*id="fishymap-save-preset"/);
  assert.match(shellHtml, /id="fishymap-save-preset"[\s\S]*data-show="\$_user_presets\.collections\['map-presets'\]\.canSave"[\s\S]*data-on:click="\$_map_actions\.saveMapPresetToken = \(\$_map_actions\.saveMapPresetToken \|\| 0\) \+ 1"[\s\S]*data-i18n-text="presets\.button\.save"/);
  assert.match(shellHtml, /id="fishymap-discard-preset"[\s\S]*class="btn btn-warning btn-outline btn-sm"/);
  assert.match(shellHtml, /id="fishymap-discard-preset"[\s\S]*data-show="\$_user_presets\.collections\['map-presets'\]\.canDiscard"[\s\S]*data-on:click="\$_map_actions\.discardMapPresetToken = \(\$_map_actions\.discardMapPresetToken \|\| 0\) \+ 1"[\s\S]*data-i18n-text="presets\.button\.discard"/);
  assert.match(shellHtml, /id="fishymap-normalize-rates"[\s\S]*data-bind="_map_ui\.windowUi\.settings\.normalizeRates"/);
  assert.match(shellHtml, /data-i18n-text="map\.settings\.normalize_rates"/);
  assert.doesNotMatch(shellHtml, /id="fishymap-view-toggle"/);
});
