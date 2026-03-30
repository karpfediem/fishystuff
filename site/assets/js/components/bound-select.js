import { findPropertyDescriptor } from "./searchable-dropdown.js";

export function resolveBoundSelectElement(host, boundSelectId) {
    const id = String(boundSelectId ?? "").trim();
    if (id) {
        const root = host.ownerDocument?.getElementById(id) ?? null;
        return root?.querySelector('select[data-role="bound-select"]') ?? null;
    }
    return host.querySelector('select[data-role="bound-select"]');
}

export function boundSelectOptions(select) {
    if (!(select instanceof HTMLSelectElement)) {
        return [];
    }
    return Array.from(select.options).filter((element) => element instanceof HTMLOptionElement);
}

export function findBoundSelectOption(select, value) {
    const normalized = String(value ?? "");
    return boundSelectOptions(select).find((option) => option.value === normalized) ?? null;
}

export function bindBoundSelect(select, onInput) {
    if (!(select instanceof HTMLSelectElement) || typeof onInput !== "function") {
        return () => {};
    }

    const releases = [];
    select.addEventListener("input", onInput);
    select.addEventListener("change", onInput);
    releases.push(() => {
        select.removeEventListener("input", onInput);
        select.removeEventListener("change", onInput);
    });

    for (const option of boundSelectOptions(select)) {
        let releaseSelectedObserver = () => {};
        if (!Object.prototype.hasOwnProperty.call(option, "selected")) {
            const descriptor = findPropertyDescriptor(option, "selected");
            if (descriptor?.get && descriptor?.set) {
                Object.defineProperty(option, "selected", {
                    configurable: true,
                    enumerable: descriptor.enumerable ?? true,
                    get() {
                        return descriptor.get.call(this);
                    },
                    set(nextValue) {
                        const previousValue = descriptor.get.call(option);
                        descriptor.set.call(option, nextValue);
                        const currentValue = descriptor.get.call(option);
                        if (currentValue === previousValue) {
                            return;
                        }
                        onInput();
                    },
                });
                releaseSelectedObserver = () => {
                    delete option.selected;
                };
            }
        }
        releases.push(releaseSelectedObserver);
    }

    return () => {
        while (releases.length) {
            const release = releases.pop();
            if (typeof release === "function") {
                release();
            }
        }
    };
}
