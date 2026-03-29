class FishyDistributionChart extends HTMLElement {
    #resizeObserver = null;
    #rafId = 0;

    connectedCallback() {
        this.#scheduleLayout();
        this.#resizeObserver = new ResizeObserver(() => {
            this.#scheduleLayout();
        });
        this.#resizeObserver.observe(this);
    }

    disconnectedCallback() {
        if (this.#resizeObserver) {
            this.#resizeObserver.disconnect();
            this.#resizeObserver = null;
        }
        if (this.#rafId) {
            cancelAnimationFrame(this.#rafId);
            this.#rafId = 0;
        }
    }

    #scheduleLayout() {
        if (this.#rafId) {
            cancelAnimationFrame(this.#rafId);
        }
        this.#rafId = requestAnimationFrame(() => {
            this.#rafId = 0;
            this.#layoutConnectors();
        });
    }

    #layoutConnectors() {
        const graphic = this.querySelector(".distribution-chart-graphic");
        const track = this.querySelector(".distribution-chart-track");
        if (!(graphic instanceof HTMLElement) || !(track instanceof HTMLElement)) {
            return;
        }

        const items = Array.from(
            this.querySelectorAll(".distribution-chart-item"),
        ).filter((node) => node instanceof HTMLElement);
        const segments = Array.from(
            this.querySelectorAll(".distribution-chart-track-segment"),
        ).filter((node) => node instanceof HTMLElement);

        if (items.length !== segments.length) {
            return;
        }

        const trackRect = track.getBoundingClientRect();
        const trackNeutralLeft = trackRect.left + trackRect.height / 2;
        const trackNeutralRight = trackRect.right - trackRect.height / 2;

        for (const [index, item] of items.entries()) {
            const connector = item.querySelector(".distribution-chart-connector");
            const callout = item.querySelector(".distribution-chart-callout");
            const segment = segments[index];
            if (
                !(connector instanceof HTMLElement) ||
                !(callout instanceof HTMLElement) ||
                !(segment instanceof HTMLElement)
            ) {
                continue;
            }

            const itemRect = item.getBoundingClientRect();
            const calloutRect = callout.getBoundingClientRect();
            const segmentRect = segment.getBoundingClientRect();
            const connectorTop = Math.max(0, calloutRect.bottom - itemRect.top - 1);
            const connectorBottom = Math.max(connectorTop, trackRect.top - itemRect.top + 1);
            const connectorHeight = Math.max(0, connectorBottom - connectorTop);

            const calloutStyle = getComputedStyle(callout);
            const calloutLeftRadius = parseFloat(calloutStyle.borderBottomLeftRadius) || 0;
            const calloutRightRadius = parseFloat(calloutStyle.borderBottomRightRadius) || 0;
            const [calloutNeutralLeft, calloutNeutralRight] = neutralEdgePoints(
                calloutRect.left,
                calloutRect.right,
                calloutLeftRadius,
                calloutRightRadius,
            );
            const [segmentNeutralLeft, segmentNeutralRight] = neutralEdgePoints(
                Math.max(segmentRect.left, trackNeutralLeft),
                Math.min(segmentRect.right, trackNeutralRight),
                0,
                0,
                segmentRect.left,
                segmentRect.right,
            );

            connector.style.top = `${connectorTop}px`;
            connector.style.bottom = "auto";
            connector.style.height = `${connectorHeight}px`;
            connector.style.clipPath = [
                "polygon(",
                `${segmentNeutralLeft - itemRect.left}px 100%, `,
                `${segmentNeutralRight - itemRect.left}px 100%, `,
                `${calloutNeutralRight - itemRect.left}px 0, `,
                `${calloutNeutralLeft - itemRect.left}px 0, `,
                `${segmentNeutralLeft - itemRect.left}px 100%`,
                ")",
            ].join("");
        }
    }
}

function neutralEdgePoints(
    start,
    end,
    startInset,
    endInset,
    fallbackStart = start,
    fallbackEnd = end,
) {
    const left = start + startInset;
    const right = end - endInset;
    if (left <= right) {
        return [left, right];
    }

    const clampedStart = Math.min(fallbackStart, fallbackEnd);
    const clampedEnd = Math.max(fallbackStart, fallbackEnd);
    const center = (clampedStart + clampedEnd) / 2;
    return [center, center];
}

export function registerDistributionChart() {
    if (window.customElements.get("fishy-distribution-chart")) {
        return;
    }

    window.customElements.define(
        "fishy-distribution-chart",
        FishyDistributionChart,
    );
}
