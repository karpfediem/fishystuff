export const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";

export function readObjectPath(root, path) {
    return String(path ?? "")
        .split(".")
        .filter(Boolean)
        .reduce((current, key) => {
            if (current && typeof current === "object" && key in current) {
                return current[key];
            }
            return null;
        }, root);
}
