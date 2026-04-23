import { DATASTAR_SIGNAL_PATCH_EVENT } from "../datastar-signals.js";

const TAG_NAME = "fishy-calculator-section-stack";
const LANGUAGE_CHANGE_EVENT = "fishystuff:languagechange";
const CARD_SELECTOR = "[data-calculator-section-card]";
const HANDLE_SELECTOR = "[data-calculator-section-drag]";
const UNPINNED_SLOT_HANDLE_SELECTOR = "[data-calculator-unpinned-slot-drag]";
const UNPINNED_SLOT_PROJECTION_SELECTOR = "[data-calculator-unpinned-slot-projection]";
const DROPZONE_SELECTOR = "[data-calculator-pin-dropzone]";
const ROWS_HOST_SELECTOR = "[data-calculator-pinned-rows]";
const ROW_SELECTOR = "[data-calculator-stack-row]";
const COLUMN_SELECTOR = "[data-calculator-pinned-column]";
const DRAG_THRESHOLD_PX = 4;
const DRAG_Z_INDEX = 80;
const DROPZONE_HEADER_GAP_PX = 16;
const DROPZONE_FRAME_PADDING_PX = 16;
const DROPZONE_EMPTY_HEIGHT_PX = 96;
const PLACEHOLDER_MIN_HEIGHT_PX = 120;
const INLINE_PLACEHOLDER_SELECTOR = "[data-calculator-inline-placeholder]";
const DEFAULT_UNPINNED_INSERT_INDEX = Object.freeze([0, 0]);
const SECTION_LAYOUT_META = Object.freeze({
    overview: { kind: "full", basis: "100%", minWidth: "100%", shareable: false },
    zone: { kind: "wide", basis: "34rem", minWidth: "min(100%, 34rem)", shareable: true },
    bite_time: { kind: "wide", basis: "34rem", minWidth: "min(100%, 34rem)", shareable: true },
    catch_time: { kind: "compact", basis: "24rem", minWidth: "min(100%, 24rem)", shareable: true },
    session: { kind: "compact", basis: "24rem", minWidth: "min(100%, 24rem)", shareable: true },
    distribution: { kind: "wide", basis: "36rem", minWidth: "min(100%, 36rem)", shareable: true },
    loot: { kind: "wide", basis: "38rem", minWidth: "min(100%, 38rem)", shareable: true },
    trade: { kind: "compact", basis: "30rem", minWidth: "min(100%, 30rem)", shareable: true },
    gear: { kind: "full", basis: "100%", minWidth: "100%", shareable: false },
    food: { kind: "compact", basis: "24rem", minWidth: "min(100%, 24rem)", shareable: true },
    buffs: { kind: "compact", basis: "26rem", minWidth: "min(100%, 26rem)", shareable: true },
    pets: { kind: "full", basis: "100%", minWidth: "100%", shareable: false },
    overlay: { kind: "wide", basis: "34rem", minWidth: "min(100%, 34rem)", shareable: true },
    debug: { kind: "compact", basis: "26rem", minWidth: "min(100%, 26rem)", shareable: true },
    default: { kind: "wide", basis: "32rem", minWidth: "min(100%, 32rem)", shareable: true },
});
const HTMLElementBase = globalThis.HTMLElement ?? class {};

function trimString(value) {
    return String(value ?? "").trim();
}

function normalizeUniqueSectionIds(sectionIds, availableSectionIds = []) {
    const available = new Set(
        Array.isArray(availableSectionIds)
            ? availableSectionIds.map(trimString).filter(Boolean)
            : [],
    );
    const seen = new Set();
    const normalized = [];
    for (const entry of Array.isArray(sectionIds) ? sectionIds : []) {
        const sectionId = trimString(entry);
        if (!sectionId || seen.has(sectionId)) {
            continue;
        }
        if (available.size && !available.has(sectionId)) {
            continue;
        }
        seen.add(sectionId);
        normalized.push(sectionId);
    }
    return normalized;
}

function normalizeUnpinnedInsertIndex(value, fallback = DEFAULT_UNPINNED_INSERT_INDEX) {
    const fallbackRow = Number.parseInt(fallback?.[0] ?? DEFAULT_UNPINNED_INSERT_INDEX[0], 10);
    const fallbackColumn = Number.parseInt(fallback?.[1] ?? DEFAULT_UNPINNED_INSERT_INDEX[1], 10);
    const rowCandidate = Number.parseInt(Array.isArray(value) ? value[0] : fallbackRow, 10);
    const columnCandidate = Number.parseInt(Array.isArray(value) ? value[1] : fallbackColumn, 10);
    return [
        Math.max(0, Number.isFinite(rowCandidate) ? rowCandidate : fallbackRow),
        Math.max(0, Number.isFinite(columnCandidate) ? columnCandidate : fallbackColumn),
    ];
}

export function flattenPinnedLayout(layout, availableSectionIds = []) {
    const entries = [];
    if (Array.isArray(layout)) {
        for (const row of layout) {
            if (!Array.isArray(row)) {
                continue;
            }
            for (const column of row) {
                if (Array.isArray(column)) {
                    entries.push(...column);
                }
            }
        }
    }
    return normalizeUniqueSectionIds(
        entries,
        availableSectionIds,
    );
}

export function normalizePinnedLayout(layout, availableSectionIds = [], fallbackPinnedSections = []) {
    const fallbackRows = normalizeUniqueSectionIds(fallbackPinnedSections, availableSectionIds)
        .map((sectionId) => [[sectionId]]);
    const rows = Array.isArray(layout) ? layout : fallbackRows;
    const seen = new Set();
    const normalized = [];
    for (const row of rows) {
        if (!Array.isArray(row)) {
            continue;
        }
        const normalizedRow = [];
        for (const column of row) {
            if (!Array.isArray(column)) {
                continue;
            }
            const normalizedColumn = [];
            for (const entry of column) {
                const sectionId = trimString(entry);
                if (!sectionId || seen.has(sectionId)) {
                    continue;
                }
                if (availableSectionIds.length && !availableSectionIds.includes(sectionId)) {
                    continue;
                }
                seen.add(sectionId);
                normalizedColumn.push(sectionId);
            }
            if (normalizedColumn.length) {
                normalizedRow.push(normalizedColumn);
            }
        }
        if (normalizedRow.length) {
            normalized.push(normalizedRow);
        }
    }
    if (Array.isArray(layout)) {
        return normalized;
    }
    return normalized.length ? normalized : fallbackRows;
}

export function buildCalculatorSectionRenderOrder(sectionIds, topLevelTab, pinnedSectionsOrLayout) {
    const availableSectionIds = normalizeUniqueSectionIds(sectionIds);
    const pinned = Array.isArray(pinnedSectionsOrLayout?.[0]?.[0])
        ? flattenPinnedLayout(pinnedSectionsOrLayout, availableSectionIds)
        : normalizeUniqueSectionIds(pinnedSectionsOrLayout, availableSectionIds);
    const topLevelSectionId = trimString(topLevelTab);
    const ordered = [...pinned];
    if (topLevelSectionId && !ordered.includes(topLevelSectionId) && availableSectionIds.includes(topLevelSectionId)) {
        ordered.push(topLevelSectionId);
    }
    for (const sectionId of availableSectionIds) {
        if (!ordered.includes(sectionId)) {
            ordered.push(sectionId);
        }
    }
    return ordered;
}

function languageHelper() {
    const helper = globalThis.window?.__fishystuffLanguage;
    return helper && typeof helper.apply === "function" ? helper : null;
}

function applyTranslations(root) {
    languageHelper()?.apply?.(root);
}

function calculatorSignals() {
    return globalThis.window?.__fishystuffCalculator?.signalObject?.() ?? null;
}

function calculatorUi() {
    const current = calculatorSignals()?._calculator_ui;
    return current && typeof current === "object" ? current : {};
}

function patchPinnedLayout(pinnedLayout) {
    if (typeof globalThis.window?.__fishystuffCalculator?.patchSignals !== "function") {
        return;
    }
    const normalizedLayout = normalizePinnedLayout(pinnedLayout);
    globalThis.window.__fishystuffCalculator.patchSignals({
        _calculator_ui: {
            pinned_layout: normalizedLayout.map((row) => row.map((column) => [...column])),
            pinned_sections: flattenPinnedLayout(normalizedLayout),
        },
    });
}

function patchUnpinnedInsertIndex(insertIndex) {
    if (typeof globalThis.window?.__fishystuffCalculator?.patchSignals !== "function") {
        return;
    }
    globalThis.window.__fishystuffCalculator.patchSignals({
        _calculator_ui: {
            unpinned_insert_index: normalizeUnpinnedInsertIndex(insertIndex),
        },
    });
}

function elementDisplayState(element) {
    if (!element) {
        return { display: "none", visibility: "hidden" };
    }
    if (typeof globalThis.getComputedStyle !== "function") {
        return {
            display: element.hidden ? "none" : "",
            visibility: element.hidden ? "hidden" : "",
        };
    }
    const computed = globalThis.getComputedStyle(element);
    return {
        display: computed?.display ?? "",
        visibility: computed?.visibility ?? "",
    };
}

function isElementVisible(element) {
    const state = elementDisplayState(element);
    return state.display !== "none" && state.visibility !== "hidden";
}

function sectionLayoutMeta(sectionId) {
    return SECTION_LAYOUT_META[trimString(sectionId)] ?? SECTION_LAYOUT_META.default;
}

function applySectionLayoutMeta(element, sectionId) {
    const meta = sectionLayoutMeta(sectionId);
    element.style.setProperty("--fishy-calculator-section-basis", meta.basis);
    element.style.setProperty("--fishy-calculator-section-min-width", meta.minWidth);
    element.dataset.calculatorSectionLayout = meta.kind;
    element.dataset.calculatorSectionShareable = meta.shareable ? "true" : "false";
}

function rowAcceptsInline(rowSectionIds, draggedSectionId) {
    const sectionIds = [
        ...normalizeUniqueSectionIds(rowSectionIds),
        ...normalizeUniqueSectionIds([draggedSectionId]),
    ];
    return sectionIds.every((sectionId) => sectionLayoutMeta(sectionId).shareable);
}

function columnLayoutMeta(sectionIds) {
    const metas = normalizeUniqueSectionIds(sectionIds).map(sectionLayoutMeta);
    if (metas.some((meta) => !meta.shareable || meta.kind === "full")) {
        return { basis: "100%", minWidth: "100%", shareable: false };
    }
    const basisRem = metas
        .map((meta) => Number.parseFloat(String(meta.basis).replace("rem", "")))
        .filter(Number.isFinite);
    const widthRem = basisRem.length ? Math.max(...basisRem) : 32;
    return {
        basis: `${widthRem}rem`,
        minWidth: `min(100%, ${widthRem}rem)`,
        shareable: true,
    };
}

function applyColumnLayoutMeta(element, sectionIds) {
    const meta = columnLayoutMeta(sectionIds);
    element.style.setProperty("--fishy-calculator-column-basis", meta.basis);
    element.style.setProperty("--fishy-calculator-column-min-width", meta.minWidth);
    element.dataset.calculatorColumnShareable = meta.shareable ? "true" : "false";
}

function closestRectIndex(rects, pointX, pointY) {
    let bestIndex = -1;
    let bestScore = Number.POSITIVE_INFINITY;
    for (let index = 0; index < rects.length; index += 1) {
        const rect = rects[index];
        const dx = pointX < rect.left ? rect.left - pointX : pointX > rect.right ? pointX - rect.right : 0;
        const dy = pointY < rect.top ? rect.top - pointY : pointY > rect.bottom ? pointY - rect.bottom : 0;
        const score = (dy * 10_000) + dx;
        if (score < bestScore) {
            bestScore = score;
            bestIndex = index;
        }
    }
    return bestIndex;
}

export class FishyCalculatorSectionStack extends HTMLElementBase {
    constructor() {
        super();
        this._frameId = 0;
        this._observer = null;
        this._isSyncing = false;
        this._drag = {
            pointerId: null,
            handle: null,
            item: null,
            sectionId: "",
            mode: "",
            active: false,
            engaged: false,
            wasPinned: false,
            startClientX: 0,
            startClientY: 0,
            offsetX: 0,
            offsetY: 0,
            width: 0,
            height: 0,
            inlinePlaceholder: null,
            rowPlaceholder: null,
        };
        this._handlePointerDown = (event) => this.handlePointerDown(event);
        this._handlePointerMove = (event) => this.handlePointerMove(event);
        this._handlePointerUp = (event) => this.handlePointerUp(event);
        this._handlePointerCancel = (event) => this.handlePointerCancel(event);
        this._handleSignalPatch = () => this.scheduleSync();
        this._handleLanguageChange = () => applyTranslations(this);
    }

    connectedCallback() {
        this.addEventListener("pointerdown", this._handlePointerDown);
        globalThis.addEventListener?.("pointermove", this._handlePointerMove);
        globalThis.addEventListener?.("pointerup", this._handlePointerUp);
        globalThis.addEventListener?.("pointercancel", this._handlePointerCancel);
        globalThis.addEventListener?.("resize", this._handleSignalPatch);
        globalThis.window?.addEventListener?.(LANGUAGE_CHANGE_EVENT, this._handleLanguageChange);
        globalThis.document?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatch);
        this._observer = new MutationObserver(() => {
            if (this._isSyncing) {
                return;
            }
            this.scheduleSync();
        });
        this._observer.observe(this, {
            childList: true,
        });
        applyTranslations(this);
        this.scheduleSync();
    }

    disconnectedCallback() {
        this._observer?.disconnect?.();
        this._observer = null;
        if (this._frameId && typeof globalThis.cancelAnimationFrame === "function") {
            globalThis.cancelAnimationFrame(this._frameId);
            this._frameId = 0;
        }
        this.removeEventListener("pointerdown", this._handlePointerDown);
        globalThis.removeEventListener?.("pointermove", this._handlePointerMove);
        globalThis.removeEventListener?.("pointerup", this._handlePointerUp);
        globalThis.removeEventListener?.("pointercancel", this._handlePointerCancel);
        globalThis.removeEventListener?.("resize", this._handleSignalPatch);
        globalThis.window?.removeEventListener?.(LANGUAGE_CHANGE_EVENT, this._handleLanguageChange);
        globalThis.document?.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatch);
        this.finishDrag({ commit: false });
    }

    scheduleSync() {
        if (this._drag.active) {
            return;
        }
        if (this._frameId && typeof globalThis.cancelAnimationFrame === "function") {
            globalThis.cancelAnimationFrame(this._frameId);
        }
        if (typeof globalThis.requestAnimationFrame !== "function") {
            this.syncOrderFromSignals();
            return;
        }
        this._frameId = globalThis.requestAnimationFrame(() => {
            this._frameId = 0;
            this.syncOrderFromSignals();
        });
    }

    sectionCards() {
        return Array.from(this.querySelectorAll(CARD_SELECTOR));
    }

    sectionCardById(sectionId) {
        const normalizedSectionId = trimString(sectionId);
        return this.sectionCards().find((card) => trimString(card.dataset.calculatorSectionId) === normalizedSectionId) || null;
    }

    availableSectionIds() {
        return this.sectionCards()
            .map((card) => trimString(card.dataset.calculatorSectionId))
            .filter(Boolean);
    }

    pinnedLayout() {
        const availableSectionIds = this.availableSectionIds();
        return normalizePinnedLayout(
            calculatorUi().pinned_layout,
            availableSectionIds,
            calculatorUi().pinned_sections,
        );
    }

    pinnedSectionIds() {
        return flattenPinnedLayout(this.pinnedLayout(), this.availableSectionIds());
    }

    unpinnedInsertIndex() {
        return normalizeUnpinnedInsertIndex(calculatorUi().unpinned_insert_index);
    }

    activeUnpinnedSectionId(pinnedSectionIds = this.pinnedSectionIds()) {
        const sectionId = trimString(calculatorUi().top_level_tab);
        if (!sectionId || pinnedSectionIds.includes(sectionId)) {
            return "";
        }
        return sectionId;
    }

    dropzoneElement() {
        return this.querySelector(DROPZONE_SELECTOR);
    }

    dropzoneBodyElement() {
        return this.querySelector(".fishy-calculator-pin-dropzone__body");
    }

    unpinnedSlotHandleElement() {
        return this.querySelector(UNPINNED_SLOT_HANDLE_SELECTOR);
    }

    unpinnedSlotProjectionElement() {
        return this.querySelector(UNPINNED_SLOT_PROJECTION_SELECTOR);
    }

    rowsHost() {
        return this.querySelector(ROWS_HOST_SELECTOR);
    }

    ensureRowsHost() {
        let host = this.rowsHost();
        if (host) {
            return host;
        }
        host = globalThis.document?.createElement?.("div") ?? null;
        if (!host) {
            return null;
        }
        host.className = "fishy-calculator-section-stack__rows";
        host.setAttribute("data-calculator-pinned-rows", "");
        const dropzone = this.dropzoneElement();
        if (dropzone?.nextSibling) {
            this.insertBefore(host, dropzone.nextSibling);
        } else if (dropzone) {
            this.appendChild(host);
        } else {
            this.prepend(host);
        }
        return host;
    }

    rowElements({ includePlaceholder = false, includeUnpinned = true } = {}) {
        return Array.from(this.rowsHost()?.querySelectorAll(ROW_SELECTOR) ?? [])
            .filter((row) => includePlaceholder || !row.hasAttribute("data-calculator-row-placeholder"))
            .filter((row) => includeUnpinned || !row.hasAttribute("data-calculator-unpinned-row"));
    }

    columnElements(row = null, { includePlaceholder = false } = {}) {
        const root = row ?? this.rowsHost();
        return Array.from(root?.children ?? [])
            .filter((child) => child.matches?.(COLUMN_SELECTOR))
            .filter((column) => includePlaceholder || !column.hasAttribute("data-calculator-column-placeholder"));
    }

    syncSectionLayoutMeta(card) {
        const sectionId = trimString(card.dataset.calculatorSectionId);
        applySectionLayoutMeta(card, sectionId);
    }

    syncCardLayoutMeta() {
        for (const card of this.sectionCards()) {
            this.syncSectionLayoutMeta(card);
        }
    }

    createRowElement() {
        const row = globalThis.document?.createElement?.("div");
        if (!row) {
            return null;
        }
        row.className = "fishy-calculator-section-stack__row";
        row.setAttribute("data-calculator-stack-row", "");
        row.setAttribute("data-calculator-pinned-row", "");
        return row;
    }

    createUnpinnedRowElement() {
        const row = globalThis.document?.createElement?.("div");
        if (!row) {
            return null;
        }
        row.className = "fishy-calculator-section-stack__row fishy-calculator-section-stack__row--unpinned";
        row.setAttribute("data-calculator-stack-row", "");
        row.setAttribute("data-calculator-unpinned-row", "");
        return row;
    }

    createColumnElement(sectionIds = []) {
        const column = globalThis.document?.createElement?.("div");
        if (!column) {
            return null;
        }
        column.className = "fishy-calculator-section-stack__column";
        column.setAttribute("data-calculator-pinned-column", "");
        applyColumnLayoutMeta(column, sectionIds);
        return column;
    }

    prepareInlinePlaceholder(mode) {
        const placeholder = globalThis.document?.createElement?.("div");
        if (!placeholder) {
            return null;
        }
        placeholder.className = "fishy-calculator-section-slot-placeholder fishy-calculator-section-slot-placeholder--inline";
        placeholder.setAttribute("data-calculator-inline-placeholder", "");
        placeholder.setAttribute("aria-hidden", "true");
        applySectionLayoutMeta(placeholder, this._drag.sectionId);
        placeholder.style.setProperty("min-height", `${Math.max(this._drag.height || 0, PLACEHOLDER_MIN_HEIGHT_PX)}px`);
        this.setInlinePlaceholderMode(placeholder, mode);
        return placeholder;
    }

    setInlinePlaceholderMode(placeholder, mode) {
        const normalizedMode = mode === "stack" ? "stack" : "column";
        placeholder.dataset.calculatorInlinePlaceholder = normalizedMode;
        placeholder.classList.toggle("fishy-calculator-section-slot-placeholder--column", normalizedMode === "column");
        placeholder.classList.toggle("fishy-calculator-section-slot-placeholder--stack", normalizedMode === "stack");
    }

    createRowPlaceholder() {
        const row = this.createRowElement();
        if (!row) {
            return null;
        }
        row.classList.add("fishy-calculator-section-stack__row--placeholder");
        row.setAttribute("data-calculator-row-placeholder", "");
        const marker = globalThis.document?.createElement?.("div");
        if (!marker) {
            return null;
        }
        marker.className = "fishy-calculator-section-slot-placeholder fishy-calculator-section-slot-placeholder--row";
        marker.setAttribute("aria-hidden", "true");
        marker.style.setProperty("min-height", `${Math.max(this._drag.height || 0, PLACEHOLDER_MIN_HEIGHT_PX)}px`);
        row.appendChild(marker);
        return row;
    }

    syncUnpinnedRow(rowsHost, cardsById, pinnedSectionIds) {
        const handle = this.unpinnedSlotHandleElement();
        const projection = this.unpinnedSlotProjectionElement();
        const sectionId = this.activeUnpinnedSectionId(pinnedSectionIds);
        if (!sectionId) {
            handle?.setAttribute("hidden", "");
            projection?.setAttribute("hidden", "");
            return;
        }
        handle?.removeAttribute("hidden");
        const card = cardsById.get(sectionId);
        if (!card) {
            return;
        }
        const row = this.createUnpinnedRowElement();
        const column = this.createColumnElement([sectionId]);
        if (!row || !column) {
            return;
        }
        column.appendChild(card);
        row.appendChild(column);
        const [rowIndex] = this.unpinnedInsertIndex();
        const pinnedRows = this.rowElements({ includePlaceholder: false, includeUnpinned: false });
        const insertionIndex = Math.max(0, Math.min(rowIndex, pinnedRows.length));
        const reference = pinnedRows[insertionIndex] ?? null;
        rowsHost.insertBefore(row, reference);
    }

    currentUnpinnedSlotAnchor() {
        if (this._drag.mode === "slot" && this._drag.active && this._drag.item?.isConnected) {
            return this._drag.item;
        }
        if (this._drag.mode === "slot" && this._drag.rowPlaceholder?.isConnected) {
            return this._drag.rowPlaceholder;
        }
        return this.rowElements({ includePlaceholder: false, includeUnpinned: true })
            .find((row) => row.hasAttribute("data-calculator-unpinned-row")) ?? null;
    }

    updateUnpinnedSlotHandlePosition() {
        const handle = this.unpinnedSlotHandleElement();
        const projection = this.unpinnedSlotProjectionElement();
        if (!handle || handle.hasAttribute("hidden")) {
            this.style.removeProperty("--fishy-calculator-unpinned-slot-top");
            projection?.setAttribute("hidden", "");
            this.style.removeProperty("--fishy-calculator-unpinned-slot-projection-top");
            return;
        }
        const anchor = this.currentUnpinnedSlotAnchor();
        if (!anchor) {
            if (!this._drag.active) {
                this.style.removeProperty("--fishy-calculator-unpinned-slot-top");
            }
        } else {
            const stackRect = this.getBoundingClientRect();
            const anchorRect = anchor.getBoundingClientRect();
            const top = Math.max(0, Math.round(anchorRect.top - stackRect.top));
            this.style.setProperty("--fishy-calculator-unpinned-slot-top", `${top}px`);
        }
        const projectionAnchor = this._drag.mode === "slot" && this._drag.rowPlaceholder?.isConnected
            ? this._drag.rowPlaceholder
            : null;
        if (!projection || !projectionAnchor) {
            projection?.setAttribute("hidden", "");
            this.style.removeProperty("--fishy-calculator-unpinned-slot-projection-top");
            return;
        }
        const stackRect = this.getBoundingClientRect();
        const projectionRect = projectionAnchor.getBoundingClientRect();
        const projectionTop = Math.max(0, Math.round(projectionRect.top - stackRect.top));
        projection.removeAttribute("hidden");
        this.style.setProperty("--fishy-calculator-unpinned-slot-projection-top", `${projectionTop}px`);
    }

    syncOrderFromSignals() {
        this._isSyncing = true;
        try {
            const slotHandle = this.unpinnedSlotHandleElement();
            const slotProjection = this.unpinnedSlotProjectionElement();
            if (slotHandle && this.firstElementChild !== slotHandle) {
                this.prepend(slotHandle);
            }
            if (slotProjection) {
                const expectedReference = slotHandle?.nextSibling ?? this.firstChild;
                if (slotProjection !== expectedReference) {
                    this.insertBefore(slotProjection, expectedReference);
                }
            }
            const dropzone = this.dropzoneElement();
            if (dropzone) {
                const expectedReference = slotProjection?.nextSibling ?? slotHandle?.nextSibling ?? this.firstChild;
                if (dropzone !== expectedReference) {
                    this.insertBefore(dropzone, expectedReference);
                }
            }
            const rowsHost = this.ensureRowsHost();
            if (dropzone && rowsHost && dropzone.nextElementSibling !== rowsHost) {
                this.insertBefore(rowsHost, dropzone.nextSibling);
            }
            this.syncCardLayoutMeta();
            const cardsById = new Map(
                this.sectionCards().map((card) => [
                    trimString(card.dataset.calculatorSectionId),
                    card,
                ]),
            );
            const availableSectionIds = Array.from(cardsById.keys());
            const pinnedLayout = normalizePinnedLayout(
                calculatorUi().pinned_layout,
                availableSectionIds,
                calculatorUi().pinned_sections,
            );
            const pinnedSectionIds = flattenPinnedLayout(pinnedLayout, availableSectionIds);
            if (rowsHost) {
                const fragment = globalThis.document?.createDocumentFragment?.();
                for (const rowColumns of pinnedLayout) {
                    const row = this.createRowElement();
                    if (!row) {
                        continue;
                    }
                    for (const columnSectionIds of rowColumns) {
                        const column = this.createColumnElement(columnSectionIds);
                        if (!column) {
                            continue;
                        }
                        for (const sectionId of columnSectionIds) {
                            const card = cardsById.get(sectionId);
                            if (card) {
                                column.appendChild(card);
                            }
                        }
                        if (column.childElementCount) {
                            applyColumnLayoutMeta(
                                column,
                                Array.from(column.querySelectorAll(CARD_SELECTOR))
                                    .map((card) => trimString(card.dataset.calculatorSectionId)),
                            );
                            row.appendChild(column);
                        }
                    }
                    if (row.childElementCount) {
                        if (fragment) {
                            fragment.appendChild(row);
                        } else {
                            rowsHost.appendChild(row);
                        }
                    }
                }
                if (fragment) {
                    rowsHost.replaceChildren(fragment);
                } else {
                    rowsHost.replaceChildren();
                }
                this.syncUnpinnedRow(rowsHost, cardsById, pinnedSectionIds);
            }
            const pinnedSet = new Set(pinnedSectionIds);
            const activeUnpinnedSectionId = this.activeUnpinnedSectionId(pinnedSectionIds);
            const order = buildCalculatorSectionRenderOrder(
                availableSectionIds,
                calculatorUi().top_level_tab,
                pinnedLayout,
            );
            let reference = rowsHost?.nextElementSibling ?? this.firstElementChild;
            for (const sectionId of order) {
                if (pinnedSet.has(sectionId) || sectionId === activeUnpinnedSectionId) {
                    continue;
                }
                const card = cardsById.get(sectionId);
                if (!card) {
                    continue;
                }
                if (card !== reference) {
                    this.insertBefore(card, reference);
                }
                reference = card.nextElementSibling;
            }
            this.updateDropzoneFrame();
        } finally {
            this._isSyncing = false;
        }
    }

    handlePointerDown(event) {
        const slotHandle = event.target?.closest?.(UNPINNED_SLOT_HANDLE_SELECTOR);
        if (slotHandle && this.contains(slotHandle) && event.button === 0) {
            const sectionId = this.activeUnpinnedSectionId();
            const item = this.sectionCardById(sectionId);
            if (!sectionId || !item || !isElementVisible(item)) {
                return;
            }
            event.preventDefault();
            this._drag.pointerId = event.pointerId;
            this._drag.handle = slotHandle;
            this._drag.item = item;
            this._drag.sectionId = sectionId;
            this._drag.mode = "slot";
            this._drag.active = false;
            this._drag.engaged = false;
            this._drag.wasPinned = false;
            this._drag.startClientX = event.clientX;
            this._drag.startClientY = event.clientY;
            this._drag.offsetX = 0;
            this._drag.offsetY = 0;
            this._drag.width = 0;
            this._drag.height = 0;
            this._drag.inlinePlaceholder = null;
            this._drag.rowPlaceholder = null;
            slotHandle.setPointerCapture?.(event.pointerId);
            return;
        }
        const handle = event.target?.closest?.(HANDLE_SELECTOR);
        if (!handle || !this.contains(handle) || event.button !== 0) {
            return;
        }
        const sectionId = trimString(handle.dataset.calculatorSectionId);
        const item = this.sectionCardById(sectionId);
        if (!item || !isElementVisible(item)) {
            return;
        }
        event.preventDefault();
        this._drag.pointerId = event.pointerId;
        this._drag.handle = handle;
        this._drag.item = item;
        this._drag.sectionId = sectionId;
        this._drag.mode = "card";
        this._drag.active = false;
        this._drag.engaged = false;
        this._drag.wasPinned = this.pinnedSectionIds().includes(sectionId);
        this._drag.startClientX = event.clientX;
        this._drag.startClientY = event.clientY;
        this._drag.offsetX = 0;
        this._drag.offsetY = 0;
        this._drag.width = 0;
        this._drag.height = 0;
        this._drag.inlinePlaceholder = null;
        this._drag.rowPlaceholder = null;
        handle.setPointerCapture?.(event.pointerId);
    }

    activateDrag(event) {
        if (this._drag.active || !this._drag.item) {
            return;
        }
        const rect = this._drag.item.getBoundingClientRect();
        if (this._drag.mode === "card" && this._drag.wasPinned) {
            this._drag.inlinePlaceholder = this.prepareInlinePlaceholder("stack");
            const sourceColumn = this._drag.item.closest(COLUMN_SELECTOR);
            if (this._drag.inlinePlaceholder && sourceColumn) {
                sourceColumn.insertBefore(this._drag.inlinePlaceholder, this._drag.item);
            }
        }
        this.appendChild(this._drag.item);
        this._drag.item.classList.add("fishy-calculator-section-card--dragging");
        this._drag.item.style.position = "fixed";
        this._drag.item.style.left = `${rect.left}px`;
        this._drag.item.style.top = `${rect.top}px`;
        this._drag.item.style.width = `${rect.width}px`;
        this._drag.item.style.height = `${rect.height}px`;
        this._drag.item.style.zIndex = String(DRAG_Z_INDEX);
        this._drag.item.style.pointerEvents = "none";
        this._drag.offsetX = event.clientX - rect.left;
        this._drag.offsetY = event.clientY - rect.top;
        this._drag.width = rect.width;
        this._drag.height = rect.height;
        this._drag.active = true;
        this.classList.add("fishy-calculator-section-stack--dragging");
        this.classList.toggle("fishy-calculator-section-stack--slot-dragging", this._drag.mode === "slot");
        this.pruneEmptyRows();
        this.updateDropzoneFrame();
        this.projectDrag(event);
    }

    handlePointerMove(event) {
        if (this._drag.pointerId !== event.pointerId || !this._drag.item) {
            return;
        }
        if (!this._drag.active) {
            const deltaX = event.clientX - this._drag.startClientX;
            const deltaY = event.clientY - this._drag.startClientY;
            if (Math.abs(deltaX) < DRAG_THRESHOLD_PX && Math.abs(deltaY) < DRAG_THRESHOLD_PX) {
                return;
            }
            this.activateDrag(event);
            if (!this._drag.active) {
                return;
            }
        }
        event.preventDefault();
        this.projectDrag(event);
    }

    rowInfos({ includeUnpinned = false } = {}) {
        return this.rowElements({ includePlaceholder: false, includeUnpinned }).map((row, index) => ({
            index,
            row,
            rect: row.getBoundingClientRect(),
            columns: this.columnElements(row).map((column, columnIndex) => {
                const cards = Array.from(column.querySelectorAll(CARD_SELECTOR))
                    .filter((card) => card !== this._drag.item && isElementVisible(card))
                    .map((card, cardIndex) => ({
                        card,
                        cardIndex,
                        rect: card.getBoundingClientRect(),
                        sectionId: trimString(card.dataset.calculatorSectionId),
                    }))
                    .filter((cardInfo) => cardInfo.sectionId && cardInfo.sectionId !== this._drag.sectionId);
                return {
                    column,
                    columnIndex,
                    rect: column.getBoundingClientRect(),
                    cards,
                    sectionIds: cards.map((cardInfo) => cardInfo.sectionId),
                };
            }),
            sectionIds: this.columnElements(row)
                .flatMap((column) => Array.from(column.querySelectorAll(CARD_SELECTOR)))
                .map((card) => trimString(card.dataset.calculatorSectionId))
                .filter((sectionId) => sectionId && sectionId !== this._drag.sectionId),
        }));
    }

    resolveMosaicDropTarget(rowInfo, pointX, pointY) {
        if (!rowInfo.columns.length) {
            return { kind: "column", rowIndex: rowInfo.index, columnIndex: 0 };
        }
        const columnIndex = Math.max(0, closestRectIndex(
            rowInfo.columns.map((columnInfo) => columnInfo.rect),
            pointX,
            pointY,
        ));
        const columnInfo = rowInfo.columns[columnIndex];
        if (!columnInfo?.cards.length) {
            return { kind: "stack", rowIndex: rowInfo.index, columnIndex, itemIndex: 0 };
        }
        const cardIndex = Math.max(0, closestRectIndex(
            columnInfo.cards.map((cardInfo) => cardInfo.rect),
            pointX,
            pointY,
        ));
        const cardInfo = columnInfo.cards[cardIndex];
        const rect = cardInfo.rect;
        const sideGutterWidth = Math.min(96, Math.max(32, rect.width * 0.25));
        if (pointX <= rect.left + sideGutterWidth) {
            return { kind: "column", rowIndex: rowInfo.index, columnIndex };
        }
        if (pointX >= rect.right - sideGutterWidth) {
            return { kind: "column", rowIndex: rowInfo.index, columnIndex: columnIndex + 1 };
        }
        const itemIndex = pointY <= rect.top + (rect.height / 2) ? cardIndex : cardIndex + 1;
        return { kind: "stack", rowIndex: rowInfo.index, columnIndex, itemIndex };
    }

    resolveDropTarget(pointX, pointY) {
        const rows = this.rowInfos({ includeUnpinned: false });
        if (!rows.length) {
            return { kind: "row", rowIndex: 0 };
        }
        if (pointY < rows[0].rect.top) {
            return { kind: "row", rowIndex: 0 };
        }
        for (let index = 0; index < rows.length; index += 1) {
            const rowInfo = rows[index];
            const previousRect = rows[index - 1]?.rect ?? null;
            const nextRect = rows[index + 1]?.rect ?? null;
            const beforeBoundary = previousRect ? (previousRect.bottom + rowInfo.rect.top) / 2 : rowInfo.rect.top;
            const afterBoundary = nextRect ? (rowInfo.rect.bottom + nextRect.top) / 2 : rowInfo.rect.bottom;
            if (pointY >= beforeBoundary && pointY < rowInfo.rect.top) {
                return { kind: "row", rowIndex: index };
            }
            if (pointY >= rowInfo.rect.top && pointY <= rowInfo.rect.bottom) {
                if (rowAcceptsInline(rowInfo.sectionIds, this._drag.sectionId)) {
                    return this.resolveMosaicDropTarget(rowInfo, pointX, pointY);
                }
                return {
                    kind: "row",
                    rowIndex: pointY <= rowInfo.rect.top + (rowInfo.rect.height / 2) ? index : index + 1,
                };
            }
            if (pointY > rowInfo.rect.bottom && pointY <= afterBoundary) {
                return { kind: "row", rowIndex: index + 1 };
            }
        }
        return { kind: "row", rowIndex: rows.length };
    }

    resolveRowDropTarget(pointY) {
        const rows = this.rowInfos({ includeUnpinned: false });
        if (!rows.length) {
            return { kind: "row", rowIndex: 0 };
        }
        if (pointY < rows[0].rect.top) {
            return { kind: "row", rowIndex: 0 };
        }
        for (let index = 0; index < rows.length; index += 1) {
            const rowInfo = rows[index];
            const previousRect = rows[index - 1]?.rect ?? null;
            const nextRect = rows[index + 1]?.rect ?? null;
            const beforeBoundary = previousRect ? (previousRect.bottom + rowInfo.rect.top) / 2 : rowInfo.rect.top;
            const afterBoundary = nextRect ? (rowInfo.rect.bottom + nextRect.top) / 2 : rowInfo.rect.bottom;
            if (pointY >= beforeBoundary && pointY < rowInfo.rect.top) {
                return { kind: "row", rowIndex: index };
            }
            if (pointY >= rowInfo.rect.top && pointY <= rowInfo.rect.bottom) {
                return {
                    kind: "row",
                    rowIndex: pointY <= rowInfo.rect.top + (rowInfo.rect.height / 2) ? index : index + 1,
                };
            }
            if (pointY > rowInfo.rect.bottom && pointY <= afterBoundary) {
                return { kind: "row", rowIndex: index + 1 };
            }
        }
        return { kind: "row", rowIndex: rows.length };
    }

    inlinePlaceholder(mode) {
        const placeholder = this._drag.inlinePlaceholder ?? this.prepareInlinePlaceholder(mode);
        if (!placeholder) {
            return null;
        }
        this._drag.inlinePlaceholder = placeholder;
        this.setInlinePlaceholderMode(placeholder, mode);
        if (mode === "column") {
            placeholder.setAttribute("data-calculator-column-placeholder", "");
            applyColumnLayoutMeta(placeholder, [this._drag.sectionId]);
        } else {
            placeholder.removeAttribute("data-calculator-column-placeholder");
        }
        return placeholder;
    }

    moveColumnPlaceholder(rowIndex, columnIndex) {
        const rows = this.rowElements({ includePlaceholder: false, includeUnpinned: false });
        const row = rows[Math.max(0, Math.min(rowIndex, rows.length - 1))];
        const placeholder = this.inlinePlaceholder("column");
        if (!row || !placeholder) {
            return;
        }
        if (this._drag.rowPlaceholder?.parentNode) {
            this._drag.rowPlaceholder.remove();
        }
        const siblings = this.columnElements(row).filter((child) => child !== placeholder);
        const insertionIndex = Math.max(0, Math.min(columnIndex, siblings.length));
        const reference = siblings[insertionIndex] ?? null;
        row.insertBefore(placeholder, reference);
        this.pruneEmptyRows();
    }

    moveStackPlaceholder(rowIndex, columnIndex, itemIndex) {
        const rows = this.rowElements({ includePlaceholder: false, includeUnpinned: false });
        const row = rows[Math.max(0, Math.min(rowIndex, rows.length - 1))];
        const column = this.columnElements(row)[Math.max(0, columnIndex)];
        const placeholder = this.inlinePlaceholder("stack");
        if (!column || !placeholder) {
            return;
        }
        if (this._drag.rowPlaceholder?.parentNode) {
            this._drag.rowPlaceholder.remove();
        }
        const siblings = Array.from(column.children).filter((child) => child !== placeholder);
        const insertionIndex = Math.max(0, Math.min(itemIndex, siblings.length));
        const reference = siblings[insertionIndex] ?? null;
        column.insertBefore(placeholder, reference);
        this.pruneEmptyRows();
    }

    moveRowPlaceholder(rowIndex) {
        const rowsHost = this.ensureRowsHost();
        if (!rowsHost) {
            return;
        }
        if (this._drag.inlinePlaceholder?.parentNode) {
            this._drag.inlinePlaceholder.remove();
        }
        const placeholderRow = this._drag.rowPlaceholder ?? this.createRowPlaceholder();
        if (!placeholderRow) {
            return;
        }
        this._drag.rowPlaceholder = placeholderRow;
        const rows = this.rowElements({ includePlaceholder: false, includeUnpinned: false });
        const insertionIndex = Math.max(0, Math.min(rowIndex, rows.length));
        const reference = rows[insertionIndex] ?? null;
        rowsHost.insertBefore(placeholderRow, reference);
        this.pruneEmptyRows();
    }

    projectDrag(event) {
        const item = this._drag.item;
        if (!this._drag.active || !item) {
            return;
        }
        const nextLeft = event.clientX - this._drag.offsetX;
        const nextTop = event.clientY - this._drag.offsetY;
        item.style.left = `${nextLeft}px`;
        item.style.top = `${nextTop}px`;
        const probeX = event.clientX;
        const probeY = event.clientY;
        const dropzone = this.dropzoneElement();
        const dropzoneMetrics = this.dropzoneMetrics();
        const dropzoneActive = Boolean(
            dropzoneMetrics
            && probeX >= dropzoneMetrics.left
            && probeX <= dropzoneMetrics.right
            && probeY >= dropzoneMetrics.top
            && probeY <= dropzoneMetrics.bottom,
        );
        this._drag.engaged = dropzoneActive;
        if (dropzoneActive) {
            const target = this._drag.mode === "slot"
                ? this.resolveRowDropTarget(probeY)
                : this.resolveDropTarget(probeX, probeY);
            if (target.kind === "column") {
                this.moveColumnPlaceholder(target.rowIndex, target.columnIndex);
            } else if (target.kind === "stack") {
                this.moveStackPlaceholder(target.rowIndex, target.columnIndex, target.itemIndex);
            } else {
                this.moveRowPlaceholder(target.rowIndex);
            }
        }
        this.updateDropzoneFrame();
        dropzone?.classList.toggle("fishy-calculator-pin-dropzone--active", dropzoneActive);
        dropzone?.classList.toggle("fishy-calculator-pin-dropzone--slot-mode", this._drag.mode === "slot");
    }

    pruneEmptyRows() {
        for (const row of this.rowElements({ includePlaceholder: true })) {
            for (const column of this.columnElements(row, { includePlaceholder: true })) {
                if (!column.querySelector(CARD_SELECTOR) && !column.querySelector(INLINE_PLACEHOLDER_SELECTOR)) {
                    column.remove();
                }
            }
        }
        for (const row of this.rowElements({ includePlaceholder: true })) {
            if (!row.querySelector(CARD_SELECTOR)
                && !row.querySelector(INLINE_PLACEHOLDER_SELECTOR)
                && !row.hasAttribute("data-calculator-row-placeholder")) {
                row.remove();
            }
        }
    }

    dropzoneMetrics() {
        const dropzone = this.dropzoneElement();
        const dropzoneBody = this.dropzoneBodyElement();
        if (!dropzone || !dropzoneBody) {
            return null;
        }
        const stackRect = this.getBoundingClientRect();
        const bodyRect = dropzoneBody.getBoundingClientRect();
        const headerHeight = Math.max(0, Math.ceil(bodyRect.height));
        const contentTopOffset = headerHeight + DROPZONE_HEADER_GAP_PX;
        const rows = this.rowElements({ includePlaceholder: true, includeUnpinned: false });
        const frameBottomOffset = rows.length
            ? Math.max(
                contentTopOffset,
                ...rows.map((row) => row.getBoundingClientRect().bottom - stackRect.top),
            )
            : contentTopOffset + DROPZONE_EMPTY_HEIGHT_PX;
        return {
            left: stackRect.left,
            right: stackRect.right,
            top: stackRect.top,
            bottom: stackRect.top + frameBottomOffset,
            contentTopOffset,
            height: Math.ceil(frameBottomOffset + DROPZONE_FRAME_PADDING_PX),
        };
    }

    updateDropzoneFrame() {
        const dropzone = this.dropzoneElement();
        if (!this._drag.active) {
            this.style.removeProperty("padding-top");
            dropzone?.style.removeProperty("height");
            this.updateUnpinnedSlotHandlePosition();
            return;
        }
        const metrics = this.dropzoneMetrics();
        if (!dropzone || !metrics) {
            this.updateUnpinnedSlotHandlePosition();
            return;
        }
        this.style.setProperty("padding-top", `${metrics.contentTopOffset}px`);
        dropzone.style.setProperty("height", `${metrics.height}px`);
        this.updateUnpinnedSlotHandlePosition();
    }

    layoutFromPlaceholders() {
        const availableSectionIds = this.availableSectionIds();
        const layout = [];
        for (const row of this.rowElements({ includePlaceholder: true, includeUnpinned: false })) {
            if (row.hasAttribute("data-calculator-row-placeholder")) {
                layout.push([[this._drag.sectionId]]);
                continue;
            }
            const rowColumns = [];
            for (const child of Array.from(row.children)) {
                if (child === this._drag.inlinePlaceholder && child.dataset.calculatorInlinePlaceholder === "column") {
                    rowColumns.push([this._drag.sectionId]);
                    continue;
                }
                if (!child.matches?.(COLUMN_SELECTOR)) {
                    continue;
                }
                const columnEntries = [];
                for (const columnChild of Array.from(child.children)) {
                    if (columnChild === this._drag.inlinePlaceholder && columnChild.dataset.calculatorInlinePlaceholder === "stack") {
                        columnEntries.push(this._drag.sectionId);
                        continue;
                    }
                    if (!columnChild.matches?.(CARD_SELECTOR)) {
                        continue;
                    }
                    const sectionId = trimString(columnChild.dataset.calculatorSectionId);
                    if (sectionId && sectionId !== this._drag.sectionId) {
                        columnEntries.push(sectionId);
                    }
                }
                if (columnEntries.length) {
                    rowColumns.push(columnEntries);
                }
            }
            if (rowColumns.length) {
                layout.push(rowColumns);
            }
        }
        return normalizePinnedLayout(layout, availableSectionIds, []);
    }

    unpinnedInsertIndexFromPlaceholder() {
        const rows = this.rowElements({ includePlaceholder: true, includeUnpinned: false });
        const rowIndex = Math.max(0, rows.indexOf(this._drag.rowPlaceholder));
        return [rowIndex, this.unpinnedInsertIndex()[1]];
    }

    handlePointerUp(event) {
        if (this._drag.pointerId !== event.pointerId) {
            return;
        }
        this.finishDrag({ commit: true });
    }

    handlePointerCancel(event) {
        if (this._drag.pointerId !== event.pointerId) {
            return;
        }
        this.finishDrag({ commit: false });
    }

    finishDrag({ commit }) {
        const wasActive = this._drag.active;
        const handle = this._drag.handle;
        const pointerId = this._drag.pointerId;
        if (handle && pointerId != null && handle.hasPointerCapture?.(pointerId)) {
            handle.releasePointerCapture(pointerId);
        }
        const item = this._drag.item;
        const shouldPatchLayout = Boolean(
            commit
            && wasActive
            && this._drag.engaged
            && this._drag.mode === "card",
        );
        const shouldPatchInsertIndex = Boolean(
            commit
            && wasActive
            && this._drag.engaged
            && this._drag.mode === "slot",
        );
        const nextLayout = shouldPatchLayout ? this.layoutFromPlaceholders() : null;
        const nextUnpinnedInsertIndex = shouldPatchInsertIndex
            ? this.unpinnedInsertIndexFromPlaceholder()
            : null;
        this._drag.inlinePlaceholder?.remove?.();
        this._drag.rowPlaceholder?.remove?.();
        if (item) {
            item.classList.remove("fishy-calculator-section-card--dragging");
            item.style.removeProperty("position");
            item.style.removeProperty("left");
            item.style.removeProperty("top");
            item.style.removeProperty("width");
            item.style.removeProperty("height");
            item.style.removeProperty("z-index");
            item.style.removeProperty("pointer-events");
        }
        this.classList.remove("fishy-calculator-section-stack--dragging");
        this.classList.remove("fishy-calculator-section-stack--slot-dragging");
        this.dropzoneElement()?.classList.remove(
            "fishy-calculator-pin-dropzone--active",
            "fishy-calculator-pin-dropzone--slot-mode",
        );
        this._drag.pointerId = null;
        this._drag.handle = null;
        this._drag.item = null;
        this._drag.sectionId = "";
        this._drag.mode = "";
        this._drag.active = false;
        this._drag.engaged = false;
        this._drag.wasPinned = false;
        this._drag.startClientX = 0;
        this._drag.startClientY = 0;
        this._drag.offsetX = 0;
        this._drag.offsetY = 0;
        this._drag.width = 0;
        this._drag.height = 0;
        this._drag.inlinePlaceholder = null;
        this._drag.rowPlaceholder = null;
        this.style.removeProperty("padding-top");
        this.dropzoneElement()?.style.removeProperty("height");
        if (shouldPatchLayout && nextLayout) {
            patchPinnedLayout(nextLayout);
            return;
        }
        if (shouldPatchInsertIndex && nextUnpinnedInsertIndex) {
            patchUnpinnedInsertIndex(nextUnpinnedInsertIndex);
            return;
        }
        this.scheduleSync();
    }
}

export function registerCalculatorSectionStack(registry = globalThis.customElements) {
    if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
        return false;
    }
    if (!registry.get(TAG_NAME)) {
        registry.define(TAG_NAME, FishyCalculatorSectionStack);
    }
    return true;
}

registerCalculatorSectionStack();
