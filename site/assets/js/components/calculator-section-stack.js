import { DATASTAR_SIGNAL_PATCH_EVENT } from "../datastar-signals.js";

const TAG_NAME = "fishy-calculator-section-stack";
const LANGUAGE_CHANGE_EVENT = "fishystuff:languagechange";
const CARD_SELECTOR = "[data-calculator-section-card]";
const HANDLE_SELECTOR = "[data-calculator-section-drag]";
const DROPZONE_SELECTOR = "[data-calculator-pin-dropzone]";
const DRAG_THRESHOLD_PX = 4;
const DRAG_Z_INDEX = 80;
const DROPZONE_HEADER_GAP_PX = 16;
const DROPZONE_FRAME_PADDING_PX = 16;
const DROPZONE_SLOT_HEIGHT_PX = 88;
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

export function buildCalculatorSectionRenderOrder(sectionIds, topLevelTab, pinnedSections) {
    const availableSectionIds = normalizeUniqueSectionIds(sectionIds);
    const pinned = normalizeUniqueSectionIds(pinnedSections, availableSectionIds);
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

export function projectPinnedSlotIndex(slotMidpoints, centerY) {
    const normalizedCenterY = Number(centerY);
    if (!Number.isFinite(normalizedCenterY)) {
        return 0;
    }
    const normalizedMidpoints = Array.isArray(slotMidpoints)
        ? slotMidpoints
            .map((value) => {
                if (value && typeof value === "object") {
                    return Number(value.thresholdY);
                }
                return Number(value);
            })
            .filter(Number.isFinite)
        : [];
    let insertionIndex = 0;
    while (
        insertionIndex < normalizedMidpoints.length
        && normalizedCenterY > normalizedMidpoints[insertionIndex]
    ) {
        insertionIndex += 1;
    }
    return insertionIndex;
}

export function buildPinnedSlots(cardRects) {
    const normalizedRects = Array.isArray(cardRects)
        ? cardRects
            .map((rect) => {
                if (!rect || typeof rect !== "object") {
                    return null;
                }
                const top = Number(rect.top);
                const height = Number(rect.height);
                if (!Number.isFinite(top) || !Number.isFinite(height)) {
                    return null;
                }
                return { top, height };
            })
            .filter(Boolean)
        : [];
    return normalizedRects.map((rect, index) => ({
        index,
        thresholdY: rect.top + (rect.height / 2),
    }));
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

function patchPinnedSections(pinnedSections) {
    if (typeof globalThis.window?.__fishystuffCalculator?.patchSignals !== "function") {
        return;
    }
    globalThis.window.__fishystuffCalculator.patchSignals({
        _calculator_ui: {
            pinned_sections: [...pinnedSections],
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
            active: false,
            engaged: false,
            wasPinned: false,
            insertionIndex: 0,
            startClientX: 0,
            startClientY: 0,
            offsetX: 0,
            offsetY: 0,
            width: 0,
            height: 0,
            placeholder: null,
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

    visibleSectionCards(excludeSectionId = "") {
        const ignoredSectionId = trimString(excludeSectionId);
        return this.sectionCards().filter((card) => {
            const sectionId = trimString(card.dataset.calculatorSectionId);
            if (!sectionId || sectionId === ignoredSectionId) {
                return false;
            }
            return isElementVisible(card);
        });
    }

    availableSectionIds() {
        return this.sectionCards()
            .map((card) => trimString(card.dataset.calculatorSectionId))
            .filter(Boolean);
    }

    pinnedSectionIds() {
        return normalizeUniqueSectionIds(
            calculatorUi().pinned_sections,
            this.availableSectionIds(),
        );
    }

    dropzoneElement() {
        return this.querySelector(DROPZONE_SELECTOR);
    }

    dropzoneBodyElement() {
        return this.querySelector(".fishy-calculator-pin-dropzone__body");
    }

    dragPlaceholderHeight() {
        return DROPZONE_SLOT_HEIGHT_PX;
    }

    pinnedCardsForDrag() {
        return this.pinnedSectionIds()
            .filter((sectionId) => sectionId !== this._drag.sectionId)
            .map((sectionId) => this.sectionCardById(sectionId))
            .filter((card) => card && isElementVisible(card));
    }

    dropzoneMetrics() {
        const dropzone = this.dropzoneElement();
        const dropzoneBody = this.dropzoneBodyElement();
        if (!dropzone || !dropzoneBody) {
            return null;
        }
        const pinnedCards = this.pinnedCardsForDrag();
        const placeholder = this._drag.placeholder;
        const stackRect = this.getBoundingClientRect();
        const bodyRect = dropzoneBody.getBoundingClientRect();
        const headerHeight = Math.max(0, Math.ceil(bodyRect.height));
        const contentTopOffset = headerHeight + DROPZONE_HEADER_GAP_PX;
        let pinnedBottomOffset = contentTopOffset;
        for (const card of pinnedCards) {
            const rect = card.getBoundingClientRect();
            pinnedBottomOffset = Math.max(pinnedBottomOffset, rect.bottom - stackRect.top);
        }
        const placeholderBottomOffset = placeholder?.parentNode === this
            ? placeholder.getBoundingClientRect().bottom - stackRect.top
            : Number.NEGATIVE_INFINITY;
        const placeholderExtendsTail = Boolean(
            this._drag.engaged
            && placeholder?.parentNode === this
            && this._drag.insertionIndex >= pinnedCards.length,
        );
        const frameBottomOffset = Math.max(
            pinnedCards.length ? pinnedBottomOffset : (contentTopOffset + this.dragPlaceholderHeight()),
            placeholderExtendsTail ? placeholderBottomOffset : Number.NEGATIVE_INFINITY,
        );
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
            return;
        }
        const metrics = this.dropzoneMetrics();
        if (!dropzone || !metrics) {
            return;
        }
        this.style.setProperty("padding-top", `${metrics.contentTopOffset}px`);
        dropzone.style.setProperty("height", `${metrics.height}px`);
    }

    syncOrderFromSignals() {
        this._isSyncing = true;
        const dropzone = this.dropzoneElement();
        try {
            if (dropzone && this.firstElementChild !== dropzone) {
                this.prepend(dropzone);
            }
            const cardsById = new Map(
                this.sectionCards().map((card) => [
                    trimString(card.dataset.calculatorSectionId),
                    card,
                ]),
            );
            const order = buildCalculatorSectionRenderOrder(
                this.availableSectionIds(),
                calculatorUi().top_level_tab,
                this.pinnedSectionIds(),
            );
            let reference = dropzone ? dropzone.nextElementSibling : this.firstElementChild;
            for (const sectionId of order) {
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
        this._drag.active = false;
        this._drag.engaged = false;
        this._drag.wasPinned = this.pinnedSectionIds().includes(sectionId);
        this._drag.insertionIndex = Math.max(0, this.pinnedSectionIds().indexOf(sectionId));
        this._drag.startClientX = event.clientX;
        this._drag.startClientY = event.clientY;
        this._drag.offsetX = 0;
        this._drag.offsetY = 0;
        this._drag.width = 0;
        this._drag.height = 0;
        this._drag.placeholder = null;
        handle.setPointerCapture?.(event.pointerId);
    }

    activateDrag(event) {
        if (this._drag.active || !this._drag.item) {
            return;
        }
        const rect = this._drag.item.getBoundingClientRect();
        const placeholder = globalThis.document?.createElement?.("div");
        if (!placeholder) {
            return;
        }
        placeholder.className = "fishy-calculator-section-slot-placeholder";
        placeholder.style.height = `${this.dragPlaceholderHeight()}px`;
        placeholder.setAttribute("aria-hidden", "true");
        this._drag.item.before(placeholder);
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
        this._drag.placeholder = placeholder;
        this._drag.active = true;
        this.classList.add("fishy-calculator-section-stack--dragging");
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

    projectDrag(event) {
        const item = this._drag.item;
        if (!this._drag.active || !item) {
            return;
        }
        const nextLeft = event.clientX - this._drag.offsetX;
        const nextTop = event.clientY - this._drag.offsetY;
        item.style.left = `${nextLeft}px`;
        item.style.top = `${nextTop}px`;
        const probeY = event.clientY;
        const probeX = event.clientX;
        const dropzone = this.dropzoneElement();
        const dropzoneMetrics = this.dropzoneMetrics();
        const pinnedCards = this.pinnedCardsForDrag();
        const pinnedSlots = buildPinnedSlots(
            pinnedCards.map((card) => card.getBoundingClientRect()),
        );
        const dropzoneActive = Boolean(
            dropzoneMetrics
            && probeX >= dropzoneMetrics.left
            && probeX <= dropzoneMetrics.right
            && probeY >= dropzoneMetrics.top
            && probeY <= dropzoneMetrics.bottom,
        );
        this._drag.engaged = dropzoneActive;
        if (dropzoneActive) {
            this._drag.insertionIndex = projectPinnedSlotIndex(pinnedSlots, probeY);
            this.movePlaceholderToPinnedIndex(pinnedCards, this._drag.insertionIndex);
        }
        this.updateDropzoneFrame();
        dropzone?.classList.toggle("fishy-calculator-pin-dropzone--active", dropzoneActive);
    }

    firstCardAfterPinnedArea() {
        const pinnedIds = new Set(this.pinnedSectionIds().filter((sectionId) => sectionId !== this._drag.sectionId));
        const order = buildCalculatorSectionRenderOrder(
            this.availableSectionIds(),
            calculatorUi().top_level_tab,
            this.pinnedSectionIds(),
        );
        for (const sectionId of order) {
            if (sectionId === this._drag.sectionId || pinnedIds.has(sectionId)) {
                continue;
            }
            const card = this.sectionCardById(sectionId);
            if (card) {
                return card;
            }
        }
        return null;
    }

    movePlaceholderToPinnedIndex(pinnedCards, insertionIndex) {
        const placeholder = this._drag.placeholder;
        if (!placeholder) {
            return;
        }
        const clampedIndex = Math.max(0, Math.min(Number(insertionIndex) || 0, pinnedCards.length));
        if (clampedIndex < pinnedCards.length) {
            this.insertBefore(placeholder, pinnedCards[clampedIndex]);
            return;
        }
        const anchor = this.firstCardAfterPinnedArea();
        if (anchor) {
            this.insertBefore(placeholder, anchor);
            return;
        }
        this.appendChild(placeholder);
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
        const placeholder = this._drag.placeholder;
        const pinnedIds = this.pinnedSectionIds().filter((sectionId) => sectionId !== this._drag.sectionId);
        const shouldPatch = Boolean(
            commit
            && wasActive
            && item
            && (this._drag.wasPinned || this._drag.engaged),
        );
        const nextPinnedSections = shouldPatch
            ? (() => {
                const next = [...pinnedIds];
                const insertionIndex = Math.max(0, Math.min(this._drag.insertionIndex, next.length));
                next.splice(insertionIndex, 0, this._drag.sectionId);
                return next;
            })()
            : null;
        if (item && placeholder?.parentNode === this) {
            this.insertBefore(item, placeholder);
        }
        placeholder?.remove?.();
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
        this.dropzoneElement()?.classList.remove("fishy-calculator-pin-dropzone--active");
        this._drag.pointerId = null;
        this._drag.handle = null;
        this._drag.item = null;
        this._drag.sectionId = "";
        this._drag.active = false;
        this._drag.engaged = false;
        this._drag.wasPinned = false;
        this._drag.insertionIndex = 0;
        this._drag.startClientX = 0;
        this._drag.startClientY = 0;
        this._drag.offsetX = 0;
        this._drag.offsetY = 0;
        this._drag.width = 0;
        this._drag.height = 0;
        this._drag.placeholder = null;
        this.style.removeProperty("padding-top");
        this.dropzoneElement()?.style.removeProperty("height");
        if (shouldPatch && nextPinnedSections) {
            patchPinnedSections(nextPinnedSections);
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
