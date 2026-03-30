(function () {
    var LEGACY_KEY = 'theme';
    var UI_SETTINGS_KEY = 'fishystuff.ui.settings.v1';
    var THEME_PATH = ['app', 'theme', 'selected'];
    var EVENT = 'fishystuff:themechange';
    var PROBE_ID = 'fishystuff-theme-probe';

    function isPlainObject(value) {
        if (!value || typeof value !== 'object') {
            return false;
        }
        var prototype = Object.getPrototypeOf(value);
        return prototype === Object.prototype || prototype === null;
    }

    function readJsonStorage(key, fallback) {
        try {
            var raw = localStorage.getItem(key);
            if (!raw) return fallback;
            var parsed = JSON.parse(raw);
            return parsed === undefined ? fallback : parsed;
        } catch (_error) {
            return fallback;
        }
    }

    function readThemeFromSettingsObject(root) {
        var current = isPlainObject(root) ? root : {};
        for (var index = 0; index < THEME_PATH.length; index += 1) {
            var part = THEME_PATH[index];
            if (!isPlainObject(current) || !(part in current)) {
                return '';
            }
            current = current[part];
        }
        return typeof current === 'string' ? current.trim() : '';
    }

    function readSharedSettings() {
        return readJsonStorage(UI_SETTINGS_KEY, {});
    }

    function writeThemeToSettingsObject(root, theme) {
        var nextRoot = isPlainObject(root) ? Object.assign({}, root) : {};
        var cursor = nextRoot;
        for (var index = 0; index < THEME_PATH.length - 1; index += 1) {
            var part = THEME_PATH[index];
            var existing = isPlainObject(cursor[part]) ? cursor[part] : {};
            cursor[part] = Object.assign({}, existing);
            cursor = cursor[part];
        }
        cursor[THEME_PATH[THEME_PATH.length - 1]] = theme;
        return nextRoot;
    }

    function sharedUiSettingsStore() {
        var store = window.__fishystuffUiSettings;
        return store && typeof store.get === 'function' && typeof store.set === 'function'
            ? store
            : null;
    }

    function persistThemePreference(theme) {
        var store = sharedUiSettingsStore();
        if (store) {
            store.set(THEME_PATH, theme);
        } else {
            try {
                var nextSettings = writeThemeToSettingsObject(readSharedSettings(), theme);
                localStorage.setItem(UI_SETTINGS_KEY, JSON.stringify(nextSettings));
            } catch (_error) {}
        }
        try {
            localStorage.removeItem(LEGACY_KEY);
        } catch (_error) {}
    }

    function migrateLegacyThemePreference() {
        var sharedTheme = readThemeFromSettingsObject(readSharedSettings());
        if (sharedTheme) {
            return sharedTheme;
        }
        var legacyTheme = '';
        try {
            legacyTheme = String(localStorage.getItem(LEGACY_KEY) || '').trim();
        } catch (_error) {
            legacyTheme = '';
        }
        if (!legacyTheme) {
            return '';
        }
        persistThemePreference(legacyTheme);
        return legacyTheme;
    }

    function ensureProbe() {
        if (!document.body) return null;
        var probe = document.getElementById(PROBE_ID);
        if (probe) return probe;

        probe = document.createElement('div');
        probe.id = PROBE_ID;
        probe.setAttribute('aria-hidden', 'true');
        probe.style.position = 'fixed';
        probe.style.width = '0';
        probe.style.height = '0';
        probe.style.overflow = 'hidden';
        probe.style.opacity = '0';
        probe.style.pointerEvents = 'none';
        probe.innerHTML = [
            '<div data-role="base" class="bg-base-100 text-base-content"></div>',
            '<div data-role="surface" class="bg-base-200 border border-base-300"></div>',
            '<div data-role="primary" class="bg-primary text-primary-content"></div>',
            '<div data-role="secondary" class="bg-secondary text-secondary-content"></div>',
            '<div data-role="accent" class="bg-accent text-accent-content"></div>',
            '<div data-role="neutral" class="bg-neutral text-neutral-content"></div>',
            '<div data-role="info" class="bg-info text-info-content"></div>',
            '<div data-role="success" class="bg-success text-success-content"></div>',
            '<div data-role="warning" class="bg-warning text-warning-content"></div>',
            '<div data-role="error" class="bg-error text-error-content"></div>'
        ].join('');
        document.body.appendChild(probe);
        return probe;
    }

    function readColor(probe, selector, property) {
        var node = probe && probe.querySelector(selector);
        if (!node) return '';
        return window.getComputedStyle(node).getPropertyValue(property).trim();
    }

    function snapshotTheme() {
        var probe = ensureProbe();
        var resolvedTheme = document.documentElement.getAttribute('data-theme') || '';
        return {
            theme: getTheme(),
            resolvedTheme: resolvedTheme,
            colors: {
                base100: readColor(probe, '[data-role="base"]', 'background-color'),
                baseContent: readColor(probe, '[data-role="base"]', 'color'),
                base200: readColor(probe, '[data-role="surface"]', 'background-color'),
                base300: readColor(probe, '[data-role="surface"]', 'border-top-color'),
                primary: readColor(probe, '[data-role="primary"]', 'background-color'),
                primaryContent: readColor(probe, '[data-role="primary"]', 'color'),
                secondary: readColor(probe, '[data-role="secondary"]', 'background-color'),
                secondaryContent: readColor(probe, '[data-role="secondary"]', 'color'),
                accent: readColor(probe, '[data-role="accent"]', 'background-color'),
                accentContent: readColor(probe, '[data-role="accent"]', 'color'),
                neutral: readColor(probe, '[data-role="neutral"]', 'background-color'),
                neutralContent: readColor(probe, '[data-role="neutral"]', 'color'),
                info: readColor(probe, '[data-role="info"]', 'background-color'),
                infoContent: readColor(probe, '[data-role="info"]', 'color'),
                success: readColor(probe, '[data-role="success"]', 'background-color'),
                successContent: readColor(probe, '[data-role="success"]', 'color'),
                warning: readColor(probe, '[data-role="warning"]', 'background-color'),
                warningContent: readColor(probe, '[data-role="warning"]', 'color'),
                error: readColor(probe, '[data-role="error"]', 'background-color'),
                errorContent: readColor(probe, '[data-role="error"]', 'color')
            }
        };
    }

    function publishThemeSnapshot() {
        if (!document.body) return;
        var detail = snapshotTheme();
        window.__fishystuffTheme = detail;
        window.dispatchEvent(new CustomEvent(EVENT, { detail: detail }));
    }

    function queuePublishThemeSnapshot() {
        if (!document.body) return;
        window.requestAnimationFrame(publishThemeSnapshot);
    }

    function resolve(theme) {
        if (theme === 'system') {
            var mq = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)');
            return (mq && mq.matches) ? 'fishy' : 'light';
        }
        return theme;
    }

    function apply(theme) {
        var active = resolve(theme);
        document.documentElement.setAttribute('data-theme', active);
        queuePublishThemeSnapshot();
    }

    function setTheme(theme) {
        persistThemePreference(theme);
        apply(theme);
        if (theme === 'system') {
            try {
                var mq = window.matchMedia('(prefers-color-scheme: dark)');
                mq.addEventListener('change', function () { apply('system'); });
            } catch (e) {}
        }
    }

    function getTheme() {
        var sharedTheme = readThemeFromSettingsObject(readSharedSettings());
        if (sharedTheme) {
            return sharedTheme;
        }
        return migrateLegacyThemePreference() || 'system';
    }

    window.__theme = {
        key: UI_SETTINGS_KEY,
        set: setTheme,
        get: getTheme,
        apply: apply,
        resolve: resolve,
        snapshot: snapshotTheme,
        publish: publishThemeSnapshot
    };

    document.addEventListener('DOMContentLoaded', function () {
        var saved = getTheme();
        apply(saved);
        ensureProbe();
        publishThemeSnapshot();

        if ('MutationObserver' in window) {
            var observer = new MutationObserver(function () {
                publishThemeSnapshot();
            });
            observer.observe(document.documentElement, {
                attributes: true,
                attributeFilter: ['data-theme']
            });
        }

        var inputs = document.querySelectorAll('#theme-switcher input[name="theme"]');
        inputs.forEach(function (input) {
            input.checked = (input.value === saved);
            input.addEventListener('change', function (e) { setTheme(e.target.value); });
        });
    });
})();
