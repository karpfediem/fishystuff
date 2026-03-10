import init from "./fishystuff_ui_bevy.js";

function resolveApiBaseUrl() {
  const hostname = window.location.hostname.toLowerCase();
  if (
    hostname === "localhost" ||
    hostname === "127.0.0.1" ||
    hostname === "::1" ||
    hostname.endsWith(".localhost")
  ) {
    return "http://localhost:8080";
  }
  return "https://api.fishystuff.fish";
}

function shouldRewriteToApi(url) {
  return (
    url.pathname.startsWith("/api/") ||
    url.pathname.startsWith("/images/") ||
    url.pathname.startsWith("/terrain/") ||
    url.pathname.startsWith("/terrain_drape/") ||
    url.pathname.startsWith("/tiles/")
  );
}

function rewriteApiUrl(input, apiBaseUrl) {
  try {
    const url = new URL(String(input), window.location.href);
    if (url.origin !== window.location.origin || !shouldRewriteToApi(url)) {
      return String(input);
    }
    return `${apiBaseUrl}${url.pathname}${url.search}`;
  } catch (_) {
    return String(input);
  }
}

function installApiFetchShim() {
  const apiBaseUrl = resolveApiBaseUrl();
  const nativeFetch = window.fetch.bind(window);

  window.fetch = function patchedFetch(input, init) {
    if (typeof input === "string" || input instanceof URL) {
      return nativeFetch(rewriteApiUrl(input, apiBaseUrl), init);
    }

    if (input instanceof Request) {
      const rewrittenUrl = rewriteApiUrl(input.url, apiBaseUrl);
      if (rewrittenUrl !== input.url) {
        return nativeFetch(new Request(rewrittenUrl, input), init);
      }
    }

    return nativeFetch(input, init);
  };
}

function syncCanvasSize() {
  const canvas = document.getElementById("bevy");
  if (!canvas) {
    return;
  }
  const rect = canvas.getBoundingClientRect();
  const logicalWidth = Math.max(1, Math.round(rect.width || canvas.clientWidth || 0));
  const logicalHeight = Math.max(1, Math.round(rect.height || canvas.clientHeight || 0));
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const physicalWidth = Math.max(1, Math.round(logicalWidth * dpr));
  const physicalHeight = Math.max(1, Math.round(logicalHeight * dpr));

  const cssWidth = `${logicalWidth}px`;
  const cssHeight = `${logicalHeight}px`;
  if (canvas.style.width !== cssWidth) {
    canvas.style.width = cssWidth;
  }
  if (canvas.style.height !== cssHeight) {
    canvas.style.height = cssHeight;
  }

  // Keep the WebGL backbuffer in sync with CSS size to avoid temporary stretching.
  if (canvas.width !== physicalWidth) {
    canvas.width = physicalWidth;
  }
  if (canvas.height !== physicalHeight) {
    canvas.height = physicalHeight;
  }
}

async function main() {
  const canvas = document.getElementById("bevy");
  installApiFetchShim();
  syncCanvasSize();
  window.addEventListener("resize", syncCanvasSize, { passive: true });
  if (canvas && canvas.parentElement && "ResizeObserver" in window) {
    const observer = new ResizeObserver(syncCanvasSize);
    observer.observe(canvas.parentElement);
  }

  try {
    await init();
  } catch (err) {
    const msg =
      err && typeof err === "object" && "message" in err
        ? err.message
        : String(err);
    if (msg.includes("Using exceptions for control flow")) {
      return;
    }
    console.error("Failed to init Bevy wasm", err);
  }
}

main();
