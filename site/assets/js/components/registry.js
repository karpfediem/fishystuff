import { registerCalculatorSectionStack } from "./calculator-section-stack.js";
import { registerCheckboxGroup } from "./checkbox-group.js";
import { registerDistributionChart, registerTimelineChart } from "./distribution-chart.js";
import { registerLootSankey } from "./loot-sankey.js";
import { registerNoticeDisclosure } from "./notice-disclosure.js";
import { registerPresetManager } from "./preset-manager.js";
import { registerPresetQuickSwitch } from "./preset-quick-switch.js";
import { registerPmfChart } from "./pmf-chart.js";
import { registerSearchableDropdown } from "./searchable-dropdown.js";
import { registerSearchableMultiselect } from "./searchable-multiselect.js";
import { attachStatBreakdownTooltip } from "./stat-breakdown-tooltip.js";

registerCalculatorSectionStack();
registerCheckboxGroup();
registerDistributionChart();
registerLootSankey();
registerNoticeDisclosure();
registerPresetManager();
registerPresetQuickSwitch();
registerPmfChart();
registerTimelineChart();
registerSearchableDropdown();
registerSearchableMultiselect();
if (typeof document !== "undefined") {
    attachStatBreakdownTooltip(document);
}
