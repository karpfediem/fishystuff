var initialited = !1;
!function () {
    const e = {0: "#ffffff", 1: "#5ff369", 2: "#0391c4", 3: "#f6c232", 4: "#d36200", 5: "#ff8b37"};
    let t = 0, i = 0, n = 0;
    const o = document.createElement("iframe");
    o.style.position = "fixed", o.style.top = "0", o.style.left = "0", o.style.border = "none", o.style.zIndex = 2e5, o.scrolling = "no", o.sandbox = "allow-scripts allow-same-origin allow-popups", o.style.visibility = "hidden", o.style.pointerEvents = "none";
    let l = "";

    function d() {
        o.parentNode && o.parentNode.removeChild(o)
    }

    function s() {
        const e = t, l = i, {width: d, height: s} = o.getBoundingClientRect();
        let r = l - n, a = e;
        a += 10, r += 10, window.innerWidth - 10 < a + d && (a = a - d - 15), window.innerHeight - 10 < r + s && (r = r - s - 15, r < 0 && (r = 10)), o.style.top = r + "px", o.style.left = a + "px"
    }

    let r = null;

    function a() {
        initialited || (initialited = !0, window.addEventListener("message", (t => {
            if (t && t.data && t.data.type && "bdolytics:iframe-mount" === t.data.type) {
                o.style.height = `${t.data.height}px`, o.style.width = `${t.data.width}px`, o.style.visibility = "visible";
                const i = t.data.grade ?? 0;
                o.style.borderRadius = "5px", o.style.border = `1px solid ${e[i]}`, s()
            }
        })), document.addEventListener("mousemove", (e => {
            if ("ontouchstart" in document.documentElement && navigator.userAgent.toLocaleLowerCase().includes("mobi")) return;
            let a = e.target.closest("a");
            if (!(a && a.href && a.href.includes("/db/") && a.href.includes("bdolytics.com") && a.href.match(/\/\d+/))) return;
            const c = new URL(a.href);
            let u = c.href.match(/(?<=\.com\/)(.*?)(?=\/db\/)/), f = "en", m = "EU";
            if (u && 2 !== u[0].split("/").length) return void console.error("bdolytics: error parsing language and region from url");
            u && ([f, m] = u[0].split("/"));
            let h = c.pathname.replace(/.*\/db\//, "");
            h.split("/")[0].endsWith("s") || (h = c.pathname.replace(/.*\/db\//, "tooltip/"), h = `${f}/${m}/${h}`, h.endsWith("/") && (h = h.slice(0, -1)), t = e.clientX, i = e.clientY, n = e.offsetY, o && s(), l !== h && (l = h, r && clearTimeout(r), r = setTimeout((() => {
                d(), o.src = `https://bdolytics.com/${h}`, document.body.appendChild(o), s()
            }), 150)))
        })), document.addEventListener("mouseout", (e => {
            d();
            let t = e.target.closest("a");
            t && t.href && t.href.includes("/db/") && t.href.includes("bdolytics.com") && (o.src = "", o.style.visibility = "hidden", l = "", r && clearTimeout(r))
        })), window.addEventListener("scroll", (() => {
            d(), o.style.visibility = "hidden", o.src = "", l = "", r && clearTimeout(r)
        })))
    }

    "interactive" === document.readyState || "complete" === document.readyState ? a() : document.addEventListener("DOMContentLoaded", a), console.log("bdolytics: tooltips loaded")
}();