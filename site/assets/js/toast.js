(function () {
  var ROOT_ID = "fishystuff-toast-root";
  var EVENT_NAME = "fishystuff:toast";
  var DEFAULT_DURATION_MS = 3200;
  var MAX_VISIBLE_TOASTS = 4;
  var runtime = typeof window !== "undefined" ? window : globalThis;
  var requestFrame =
    typeof runtime.requestAnimationFrame === "function"
      ? runtime.requestAnimationFrame.bind(runtime)
      : function (callback) {
          return runtime.setTimeout(callback, 0);
        };

  function currentDocument() {
    return runtime.document || (typeof document !== "undefined" ? document : null);
  }

  function asObject(value) {
    return value && typeof value === "object" ? value : {};
  }

  function normalizeTone(value) {
    var tone = String(value || "").trim().toLowerCase();
    if (tone === "success" || tone === "warning" || tone === "error") {
      return tone;
    }
    return "info";
  }

  function toneClass(tone) {
    if (tone === "success") return "alert-success";
    if (tone === "warning") return "alert-warning";
    if (tone === "error") return "alert-error";
    return "alert-info";
  }

  function toneRole(tone) {
    return tone === "error" || tone === "warning" ? "alert" : "status";
  }

  function defaultDuration(tone) {
    return tone === "error" || tone === "warning" ? 5200 : DEFAULT_DURATION_MS;
  }

  function normalizeOptions(input, overrides) {
    var base =
      typeof input === "string"
        ? { message: input }
        : asObject(input);
    var extra = asObject(overrides);
    var tone = normalizeTone(extra.tone || base.tone);
    var message = String(extra.message || base.message || "").trim();
    var title = String(extra.title || base.title || "").trim();
    var duration = extra.duration;
    if (!Number.isFinite(duration)) {
      duration = base.duration;
    }
    if (duration === Infinity) {
      duration = 0;
    } else if (!Number.isFinite(duration)) {
      duration = defaultDuration(tone);
    } else {
      duration = Math.max(0, duration);
    }
    return {
      tone: tone,
      title: title,
      message: message,
      duration: duration,
    };
  }

  function ensureRoot() {
    var doc = currentDocument();
    if (!doc) {
      return null;
    }
    var existing = doc.getElementById(ROOT_ID);
    if (existing) {
      return existing;
    }
    if (!doc.body) {
      return null;
    }
    var root = doc.createElement("div");
    root.id = ROOT_ID;
    root.className = "toast toast-bottom toast-center pointer-events-none";
    root.setAttribute("aria-live", "polite");
    root.setAttribute("aria-atomic", "false");
    doc.body.appendChild(root);
    return root;
  }

  function finishToastRemoval(node) {
    if (!node || !node.parentNode) {
      return;
    }
    node.parentNode.removeChild(node);
  }

  function closeToast(node) {
    if (!node || node.dataset.state === "closing") {
      return;
    }
    if (node.__fishyToastTimer) {
      runtime.clearTimeout(node.__fishyToastTimer);
      node.__fishyToastTimer = 0;
    }
    node.dataset.state = "closing";
    runtime.setTimeout(function () {
      finishToastRemoval(node);
    }, 180);
  }

  function pruneOldToasts(root) {
    while (root.childElementCount >= MAX_VISIBLE_TOASTS) {
      finishToastRemoval(root.firstElementChild);
    }
  }

  function createToastElement(options) {
    var doc = currentDocument();
    if (!doc) {
      return null;
    }
    var shell = doc.createElement("div");
    shell.className = "fishy-toast pointer-events-auto w-[min(28rem,calc(100vw-1rem))] max-w-full";
    shell.dataset.state = "enter";

    var alert = doc.createElement("div");
    alert.className =
      "alert alert-soft border border-base-300/70 shadow-lg " + toneClass(options.tone);
    alert.setAttribute("role", toneRole(options.tone));
    alert.tabIndex = 0;
    shell.appendChild(alert);

    var content = doc.createElement("div");
    content.className = "min-w-0";
    alert.appendChild(content);

    if (options.title) {
      var title = doc.createElement("div");
      title.className = "fishy-toast-title text-sm font-semibold leading-tight";
      title.textContent = options.title;
      content.appendChild(title);
    }

    var message = doc.createElement("div");
    message.className = "fishy-toast-message text-sm leading-snug";
    message.textContent = options.message;
    content.appendChild(message);

    shell.addEventListener("click", function () {
      closeToast(shell);
    });

    shell.addEventListener("keydown", function (event) {
      if (event.key === "Escape" || event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        closeToast(shell);
      }
    });

    return shell;
  }

  function showToast(input, overrides) {
    var options = normalizeOptions(input, overrides);
    if (!options.message) {
      return null;
    }
    var root = ensureRoot();
    if (!root) {
      currentDocument()?.addEventListener(
        "DOMContentLoaded",
        function () {
          showToast(options);
        },
        { once: true },
      );
      return null;
    }

    pruneOldToasts(root);
    var node = createToastElement(options);
    if (!node) {
      return null;
    }
    root.appendChild(node);
    requestFrame(function () {
      node.dataset.state = "open";
    });
    if (options.duration > 0) {
      node.__fishyToastTimer = runtime.setTimeout(function () {
        closeToast(node);
      }, options.duration);
    }
    return {
      close: function () {
        closeToast(node);
      },
      element: node,
    };
  }

  async function copyText(text) {
    var doc = currentDocument();
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(String(text ?? ""));
      return;
    }
    if (!doc?.createElement || !doc.body || !doc.execCommand) {
      throw new Error("Clipboard API unavailable");
    }
    var probe = doc.createElement("textarea");
    probe.value = String(text ?? "");
    probe.setAttribute("readonly", "");
    probe.style.position = "fixed";
    probe.style.opacity = "0";
    probe.style.pointerEvents = "none";
    doc.body.appendChild(probe);
    probe.select();
    probe.setSelectionRange(0, probe.value.length);
    var copied = doc.execCommand("copy");
    probe.remove();
    if (!copied) {
      throw new Error("Clipboard API unavailable");
    }
  }

  async function toastCopyText(text, options) {
    var config = asObject(options);
    try {
      await copyText(text);
      showToast({
        tone: "success",
        title: config.successTitle,
        message: String(config.success || "Copied to clipboard."),
        duration: config.successDuration,
      });
      return true;
    } catch (_error) {
      showToast({
        tone: "error",
        title: config.errorTitle,
        message: String(config.error || "Clipboard access is unavailable in this browser."),
        duration: config.errorDuration,
      });
      return false;
    }
  }

  function clearToasts() {
    var root = ensureRoot();
    if (!root) {
      return;
    }
    Array.from(root.children).forEach(finishToastRemoval);
  }

  var api = {
    show: showToast,
    info: function (message, options) {
      return showToast(message, Object.assign({}, asObject(options), { tone: "info" }));
    },
    success: function (message, options) {
      return showToast(message, Object.assign({}, asObject(options), { tone: "success" }));
    },
    warning: function (message, options) {
      return showToast(message, Object.assign({}, asObject(options), { tone: "warning" }));
    },
    error: function (message, options) {
      return showToast(message, Object.assign({}, asObject(options), { tone: "error" }));
    },
    copyText: toastCopyText,
    clear: clearToasts,
    close: closeToast,
  };

  runtime.__fishystuffToast = api;
  runtime.fishyToast = api;

  runtime.addEventListener?.(EVENT_NAME, function (event) {
    showToast(event && event.detail ? event.detail : {});
  });

  var readyDocument = currentDocument();
  if (readyDocument && (readyDocument.readyState === "interactive" || readyDocument.readyState === "complete")) {
    ensureRoot();
  } else {
    readyDocument?.addEventListener("DOMContentLoaded", ensureRoot, { once: true });
  }
})();
