(function () {
    var KEY = "fishystuff.ui.settings.v1";
    var EVENT = "fishystuff:uisettingschange";

    function isPlainObject(value) {
        if (!value || typeof value !== "object") {
            return false;
        }
        var prototype = Object.getPrototypeOf(value);
        return prototype === Object.prototype || prototype === null;
    }

    function normalizeSettings(value) {
        return isPlainObject(value) ? value : {};
    }

    function readFromStorage() {
        try {
            return normalizeSettings(JSON.parse(localStorage.getItem(KEY) || "{}"));
        } catch (_error) {
            return {};
        }
    }

    function normalizePath(path) {
        if (Array.isArray(path)) {
            return path
                .map(function (part) { return String(part || "").trim(); })
                .filter(Boolean);
        }
        return String(path || "")
            .split(".")
            .map(function (part) { return part.trim(); })
            .filter(Boolean);
    }

    function getAtPath(root, pathParts, fallback) {
        var current = root;
        for (var index = 0; index < pathParts.length; index += 1) {
            if (!isPlainObject(current) || !(pathParts[index] in current)) {
                return fallback;
            }
            current = current[pathParts[index]];
        }
        return current === undefined ? fallback : current;
    }

    function setAtPath(root, pathParts, value) {
        if (!pathParts.length) {
            return normalizeSettings(value);
        }

        var nextRoot = normalizeSettings(root);
        nextRoot = Object.assign({}, nextRoot);
        var cursor = nextRoot;
        for (var index = 0; index < pathParts.length - 1; index += 1) {
            var part = pathParts[index];
            var existing = isPlainObject(cursor[part]) ? cursor[part] : {};
            cursor[part] = Object.assign({}, existing);
            cursor = cursor[part];
        }
        cursor[pathParts[pathParts.length - 1]] = value;
        return nextRoot;
    }

    function dispatchChange(detail) {
        window.dispatchEvent(new CustomEvent(EVENT, { detail: detail }));
    }

    var cache = readFromStorage();

    function persist(nextSettings, source, changedPath) {
        cache = normalizeSettings(nextSettings);
        try {
            localStorage.setItem(KEY, JSON.stringify(cache));
        } catch (_error) {
        }
        dispatchChange({
            key: KEY,
            path: changedPath || null,
            settings: cache,
            source: source || "local",
        });
        return cache;
    }

    function get(path, fallback) {
        var parts = normalizePath(path);
        if (!parts.length) {
            return cache;
        }
        return getAtPath(cache, parts, fallback);
    }

    function set(path, value) {
        var parts = normalizePath(path);
        return persist(setAtPath(cache, parts, value), "local", parts.join("."));
    }

    function update(path, updater) {
        var parts = normalizePath(path);
        var current = getAtPath(cache, parts, undefined);
        var nextValue = typeof updater === "function" ? updater(current) : updater;
        return persist(setAtPath(cache, parts, nextValue), "local", parts.join("."));
    }

    function subscribe(listener) {
        if (typeof listener !== "function") {
            return function () {};
        }
        function handle(event) {
            listener(event.detail || {});
        }
        window.addEventListener(EVENT, handle);
        return function () {
            window.removeEventListener(EVENT, handle);
        };
    }

    window.addEventListener("storage", function (event) {
        if (event.key !== KEY) {
            return;
        }
        cache = normalizeSettings((function () {
            try {
                return JSON.parse(event.newValue || "{}");
            } catch (_error) {
                return {};
            }
        })());
        dispatchChange({
            key: KEY,
            path: null,
            settings: cache,
            source: "storage",
        });
    });

    window.__fishystuffUiSettings = Object.freeze({
        key: KEY,
        event: EVENT,
        get: get,
        set: set,
        update: update,
        subscribe: subscribe,
        snapshot: function () { return cache; },
    });
})();
