import { registerCheckboxGroup } from "./checkbox-group.js";
import { registerDistributionChart } from "./distribution-chart.js";
import { registerLootSankey } from "./loot-sankey.js";
import { registerPmfChart } from "./pmf-chart.js";
import { registerSearchableDropdown } from "./searchable-dropdown.js";
import { registerSearchableMultiselect } from "./searchable-multiselect.js";
import { attachStatBreakdownTooltip } from "./stat-breakdown-tooltip.js";

registerCheckboxGroup();
registerDistributionChart();
registerLootSankey();
registerPmfChart();
registerSearchableDropdown();
registerSearchableMultiselect();
if (typeof document !== "undefined") {
    attachStatBreakdownTooltip(document);
}
