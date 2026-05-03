import { test } from "bun:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const RUN_E2E = process.env.FISHYSTUFF_E2E === "1";
const BASE_URL = (process.env.FISHYSTUFF_E2E_BASE_URL || "http://localhost:1990").replace(/\/+$/, "");
const CALCULATOR_URL = `${BASE_URL}/calculator/`;
const DEFAULT_ZONE = "240,74,74";
const AHRMO_ZONE = "143,190,212";

const e2eTest = RUN_E2E ? test : test.skip;

function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor(predicate, { timeoutMs = 10_000, intervalMs = 50, label = "condition" } = {}) {
  const startedAt = Date.now();
  let lastError = null;
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const value = await predicate();
      if (value) {
        return value;
      }
    } catch (error) {
      lastError = error;
    }
    await wait(intervalMs);
  }
  throw new Error(`Timed out waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

class CdpClient {
  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = new Map();
    this.ws = new WebSocket(wsUrl);
    this.opened = new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", () => reject(new Error(`CDP websocket failed: ${wsUrl}`)), {
        once: true,
      });
    });
    this.ws.addEventListener("message", (event) => this.handleMessage(event));
    this.ws.addEventListener("close", () => {
      for (const { reject } of this.pending.values()) {
        reject(new Error(`CDP websocket closed: ${wsUrl}`));
      }
      this.pending.clear();
    });
  }

  static async connect(wsUrl) {
    const client = new CdpClient(wsUrl);
    await client.opened;
    return client;
  }

  handleMessage(event) {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const pending = this.pending.get(message.id);
      if (!pending) {
        return;
      }
      this.pending.delete(message.id);
      if (message.error) {
        pending.reject(new Error(`${message.error.message}: ${message.error.data || ""}`));
      } else {
        pending.resolve(message.result || {});
      }
      return;
    }
    const listeners = this.listeners.get(message.method) || [];
    for (const listener of listeners) {
      listener(message.params || {});
    }
  }

  on(method, listener) {
    if (!this.listeners.has(method)) {
      this.listeners.set(method, []);
    }
    this.listeners.get(method).push(listener);
  }

  once(method) {
    return new Promise((resolve) => {
      const listener = (params) => {
        const listeners = this.listeners.get(method) || [];
        this.listeners.set(
          method,
          listeners.filter((candidate) => candidate !== listener),
        );
        resolve(params);
      };
      this.on(method, listener);
    });
  }

  send(method, params = {}) {
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    this.ws.send(payload);
    return promise;
  }

  close() {
    this.ws.close();
  }
}

function launchChromium() {
  const userDataDir = mkdtempSync(join(tmpdir(), "fishystuff-calculator-e2e-"));
  const args = [
    "--headless=new",
    "--disable-gpu",
    "--disable-dev-shm-usage",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-debugging-port=0",
    `--user-data-dir=${userDataDir}`,
    "about:blank",
  ];
  if (typeof process.getuid === "function" && process.getuid() === 0) {
    args.unshift("--no-sandbox");
  }

  const child = spawn("chromium", args, {
    stdio: ["ignore", "ignore", "pipe"],
  });

  const wsUrlPromise = new Promise((resolve, reject) => {
    let stderr = "";
    const timeout = setTimeout(() => {
      reject(new Error(`Chromium did not expose DevTools in time.\n${stderr}`));
    }, 10_000);

    child.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
  });

  return {
    child,
    userDataDir,
    wsUrlPromise,
    async close() {
      child.kill("SIGTERM");
      await new Promise((resolve) => child.once("exit", resolve));
      rmSync(userDataDir, { recursive: true, force: true });
    },
  };
}

async function openPage(browserWsUrl) {
  const browser = await CdpClient.connect(browserWsUrl);
  const { targetId } = await browser.send("Target.createTarget", { url: "about:blank" });
  const { host } = new URL(browserWsUrl);
  const targets = await fetch(`http://${host}/json/list`).then((response) => response.json());
  const target = targets.find((entry) => entry.id === targetId);
  assert.ok(target?.webSocketDebuggerUrl, `Could not resolve page target ${targetId}`);
  const page = await CdpClient.connect(target.webSocketDebuggerUrl);
  return { browser, page };
}

async function navigate(page, url) {
  const loaded = page.once("Page.loadEventFired");
  await page.send("Page.navigate", { url });
  await loaded;
}

async function evaluate(page, expression) {
  const result = await page.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.exception?.description || result.exceptionDetails.text);
  }
  return result.result?.value;
}

async function waitForCalculator(page) {
  return evaluate(
    page,
    `(() => new Promise(async (resolve, reject) => {
      const startedAt = performance.now();
      while (!window.__fishystuffCalculator && performance.now() - startedAt < 10000) {
        await new Promise((step) => setTimeout(step, 50));
      }
      if (!window.__fishystuffCalculator) {
        reject(new Error("calculator helper did not load"));
        return;
      }
      await window.__fishystuffCalculator.ready;
      while (performance.now() - startedAt < 10000) {
        const signals = window.__fishystuffCalculator.signalObject();
        if (signals && signals._calc && signals._live) {
          resolve({
            zone: signals.zone,
            calcZone: signals._calc.zone_name,
            liveAvg: signals._live.zone_bite_avg,
          });
          return;
        }
        await new Promise((step) => setTimeout(step, 50));
      }
      reject(new Error("calculator signals did not initialize"));
    }))()`,
  );
}

async function selectAhrmoZone(page) {
  return evaluate(
    page,
    `(() => new Promise(async (resolve, reject) => {
      const waitFor = async (predicate, label) => {
        const startedAt = performance.now();
        while (performance.now() - startedAt < 10000) {
          const value = predicate();
          if (value) return value;
          await new Promise((step) => setTimeout(step, 50));
        }
        throw new Error("timed out waiting for " + label);
      };
      try {
        const dropdown = document.querySelector("#calculator-zone-picker");
        if (!dropdown || typeof dropdown.open !== "function") {
          throw new Error("zone picker is unavailable");
        }
        dropdown.open();
        const option = await waitFor(
          () => Array.from(document.querySelectorAll("[data-searchable-dropdown-option]"))
            .find((element) => element.textContent.includes("Ahrmo Sea - Depth 3")),
          "Ahrmo option",
        );
        option.click();
        const signals = await waitFor(() => {
          const current = window.__fishystuffCalculator.signalObject();
          return current?._calc?.zone_name?.includes("Ahrmo Sea - Depth 3") ? current : null;
        }, "Ahrmo calc signals");
        resolve({
          zone: signals.zone,
          calcZone: signals._calc.zone_name,
          liveAvg: signals._live.zone_bite_avg,
        });
      } catch (error) {
        const current = window.__fishystuffCalculator?.signalObject?.();
        reject(new Error((error && error.message ? error.message : String(error)) + " " + JSON.stringify({
          zone: current?.zone,
          calcZone: current?._calc?.zone_name,
          liveAvg: current?._live?.zone_bite_avg,
        })));
      }
    }))()`,
  );
}

async function discardCalculatorChanges(page) {
  return evaluate(
    page,
    `(() => new Promise(async (resolve, reject) => {
      const waitFor = async (predicate, label) => {
        const startedAt = performance.now();
        while (performance.now() - startedAt < 10000) {
          const value = predicate();
          if (value) return value;
          await new Promise((step) => setTimeout(step, 50));
        }
        throw new Error("timed out waiting for " + label);
      };
      try {
        await waitFor(() => {
          const signals = window.__fishystuffCalculator.signalObject();
          return window.__fishystuffCalculator.presetCollectionCanDiscard(
            signals?._user_presets,
            "calculator-presets",
          );
        }, "dirty calculator preset state");
        const button = Array.from(document.querySelectorAll("button"))
          .find((element) => String(element.getAttribute("data-on:click") || "")
            .includes("discardCalculatorToken"));
        if (!button) {
          throw new Error("discard button is unavailable");
        }
        button.click();
        const signals = await waitFor(() => {
          const current = window.__fishystuffCalculator.signalObject();
          return current?.zone === "${DEFAULT_ZONE}"
            && current?._calc?.zone_name?.includes("Velia Beach")
            ? current
            : null;
        }, "discarded calculator signals");
        resolve({
          zone: signals.zone,
          calcZone: signals._calc.zone_name,
          liveAvg: signals._live.zone_bite_avg,
          overviewBite: Array.from(document.querySelectorAll('[data-text="$_live.bite_time"]'))
            .map((element) => element.textContent.trim())
            .find(Boolean) || "",
        });
      } catch (error) {
        reject(error);
      }
    }))()`,
  );
}

e2eTest("calculator preset discard refreshes derived overview signals", async () => {
  await fetch(CALCULATOR_URL, { method: "GET" }).then((response) => {
    assert.ok(response.ok, `Local calculator page is not reachable at ${CALCULATOR_URL}`);
  });

  const chrome = launchChromium();
  let browser = null;
  let page = null;
  const evalRequests = [];
  try {
    const browserWsUrl = await chrome.wsUrlPromise;
    ({ browser, page } = await openPage(browserWsUrl));
    await page.send("Page.enable");
    await page.send("Runtime.enable");
    await page.send("Network.enable");
    await page.send("Storage.clearDataForOrigin", {
      origin: BASE_URL,
      storageTypes: "local_storage,session_storage",
    });
    page.on("Network.requestWillBeSent", ({ request }) => {
      if (request?.method === "POST" && request?.url?.includes("/api/v1/calculator/datastar/eval")) {
        evalRequests.push({
          method: request.method,
          url: request.url,
        });
      }
    });

    await navigate(page, CALCULATOR_URL);

    const initial = await waitForCalculator(page);
    assert.equal(initial.zone, DEFAULT_ZONE);
    assert.match(initial.calcZone, /Velia Beach/);

    const hook = await evaluate(
      page,
      `(() => ({
        signalPatch: String(document.querySelector('[data-on-signal-patch]')?.getAttribute('data-on-signal-patch') || '').includes('shouldEvalPatch(patch)'),
        delayedSignalPatch: document.querySelector('[data-on-signal-patch__debounce\\\\.150ms]') !== null,
        customEvalRequest: document.querySelector('[data-on\\\\:fishystuff-calculator-eval-request__window]') !== null,
      }))()`,
    );
    assert.deepEqual(hook, {
      signalPatch: true,
      delayedSignalPatch: false,
      customEvalRequest: false,
    });

    const beforeSelectEvalCount = evalRequests.length;
    const selected = await selectAhrmoZone(page);
    await waitFor(
      () => evalRequests.length > beforeSelectEvalCount,
      { label: "zone select eval request" },
    );
    const afterSelectEvalCount = evalRequests.length;
    await wait(750);
    assert.equal(evalRequests.length, afterSelectEvalCount, "zone selection must not keep posting eval requests");
    assert.equal(
      afterSelectEvalCount - beforeSelectEvalCount <= 2,
      true,
      `zone selection kept posting eval requests: ${JSON.stringify(evalRequests.slice(beforeSelectEvalCount, afterSelectEvalCount))}`,
    );
    assert.equal(selected.zone, AHRMO_ZONE);
    assert.match(selected.calcZone, /Ahrmo Sea - Depth 3/);
    assert.notEqual(selected.liveAvg, initial.liveAvg);

    const beforeDiscardEvalCount = evalRequests.length;
    const discarded = await discardCalculatorChanges(page);
    await waitFor(
      () => evalRequests.length > beforeDiscardEvalCount,
      { label: "discard eval request" },
    );
    const afterDiscardEvalCount = evalRequests.length;
    await wait(750);
    assert.equal(evalRequests.length, afterDiscardEvalCount, "discard must not keep posting eval requests");
    assert.equal(
      afterDiscardEvalCount - beforeDiscardEvalCount <= 2,
      true,
      `discard kept posting eval requests: ${JSON.stringify(evalRequests.slice(beforeDiscardEvalCount, afterDiscardEvalCount))}`,
    );
    assert.equal(discarded.zone, DEFAULT_ZONE);
    assert.match(discarded.calcZone, /Velia Beach/);
    assert.equal(discarded.liveAvg, initial.liveAvg);
    assert.equal(discarded.overviewBite, initial.liveAvg);
  } finally {
    page?.close();
    browser?.close();
    await chrome.close();
  }
}, 60_000);
