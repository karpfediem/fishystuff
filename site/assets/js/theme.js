(function () {
    var KEY = 'theme';

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
    }

    function setTheme(theme) {
        localStorage.setItem(KEY, theme);
        apply(theme);
        if (theme === 'system') {
            try {
                var mq = window.matchMedia('(prefers-color-scheme: dark)');
                mq.addEventListener('change', function () { apply('system'); });
            } catch (e) {}
        }
    }

    function getTheme() { return localStorage.getItem(KEY) || 'system'; }

    window.__theme = { set: setTheme, get: getTheme, apply: apply, resolve: resolve };

    document.addEventListener('DOMContentLoaded', function () {
        var saved = getTheme();
        apply(saved);
        var inputs = document.querySelectorAll('#theme-switcher input[name="theme"]');
        inputs.forEach(function (input) {
            input.checked = (input.value === saved);
            input.addEventListener('change', function (e) { setTheme(e.target.value); });
        });
    });
})();
