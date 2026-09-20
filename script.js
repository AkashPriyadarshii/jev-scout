/* jev-scout landing — one authored motion: soundings press down onto the chart.
   Verified candidates land teal with a contour ring; filtered stay ghosted.
   Reduced motion: instant flush. Everything else is static print. */

(function () {
  "use strict";
  var REDUCED = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  var data = [
    { name: "sabiql", x: 250, y: 150, kind: "verified", label: "308★ · fit 2.8 · conf 0.76", dock: "sleeping sickness UI" },
    { name: "tuitab", x: 430, y: 215, kind: "ghost", label: "fit 1.6 — filtered" },
    { name: "WayGet", x: 560, y: 255, kind: "ghost", label: "fit 0.4 — filtered" }
  ];

  var plotEl = document.querySelector(".plot");
  if (!plotEl) return;
  var NS = "http://www.w3.org/2000/svg";

  function el(name, attrs) {
    var n = document.createElementNS(NS, name);
    for (var k in attrs) n.setAttribute(k, attrs[k]);
    return n;
  }

  // graticule grid lines
  for (var gx = 1; gx < 7; gx++) plotEl.appendChild(el("line", { x1: gx * 120, y1: 0, x2: gx * 120, y2: 300, "class": "grid-line" }));
  for (var gy = 1; gy < 3; gy++) plotEl.appendChild(el("line", { x1: 0, y1: gy * 120, x2: 760, y2: gy * 120, "class": "grid-line" }));

  // survey route to the best sounding
  plotEl.appendChild(el("path", { d: "M30 270 C 120 260, 170 170, 250 150", "class": "route" }));

  var group = el("g");
  plotEl.appendChild(group);

  var delay = 400, step = 700;
  data.forEach(function (d, i) {
    var t = REDUCED ? 0 : delay + i * step;
    var isGhost = d.kind === "ghost";

    var g = el("g", { transform: "translate(" + d.x + "," + d.y + ")", opacity: isGhost ? 0.75 : 0 });
    g.setAttribute("data-x", d.x);
    g.setAttribute("data-y", d.y);
    if (!isGhost) {
      var ring = el("circle", { r: 8, "class": "ring", "stroke-width": 1.5 });
      g.appendChild(ring);
    }
    if (isGhost) {
      g.appendChild(el("circle", { r: 9, "class": "point-ghost" }));
    } else {
      g.appendChild(el("circle", { r: 5, "class": "point-verified" }));
    }
    g.appendChild(renderLabel(d));
    group.appendChild(g);

    if (REDUCED) {
      g.setAttribute("opacity", 1);
      return;
    }
    animateLand(g, ring, t);
  });

  function renderLabel(d) {
    var wrap = el("g");
    var lbl = el("text", { "class": d.kind === "ghost" ? "lbl-dim" : "lbl", x: 14, y: d.kind === "ghost" ? 26 : -8, "font-family": "IBM Plex Mono" });
    lbl.textContent = d.name;
    var val = el("text", { "class": d.kind === "ghost" ? "lbl-dim" : "val", x: 14, y: d.kind === "ghost" ? 40 : 6, "font-family": "IBM Plex Mono" });
    val.textContent = d.label;
    wrap.appendChild(lbl);
    wrap.appendChild(val);
    return wrap;
  }

  function animateLand(g, ring, t) {
    setTimeout(function () {
      g.setAttribute("opacity", 1);
      var x = g.getAttribute("data-x"), y = g.getAttribute("data-y");
      var start = null;
      function frame(ts) {
        if (!start) start = ts;
        var p = Math.min((ts - start) / 420, 1);
        var dy = (1 - (1 + 2.2 * Math.pow(p - 1, 3) + 1.6 * Math.pow(p - 1, 2))) * 14; // overshoot settle
        g.setAttribute("transform", "translate(" + x + "," + y + ") translate(0," + dy.toFixed(1) + ")");
        if (p < 1) requestAnimationFrame(frame);
      }
      requestAnimationFrame(frame);
      if (ring) {
        ring.setAttribute("r", 8);
        ring.setAttribute("opacity", 1);
        var rs = null;
        function rframe(ts) {
          if (!rs) rs = ts;
          var p = Math.min((ts - rs) / 700, 1);
          ring.setAttribute("r", 8 + p * 34);
          ring.setAttribute("opacity", String(1 - p));
          if (p < 1) requestAnimationFrame(rframe);
        }
        requestAnimationFrame(rframe);
      }
    }, t);
  }

  // evallog: verification lines type in amber, settle to final ink
  var log = document.getElementById("evallog");
  if (log) {
    var lines = [
      { txt: "poll github + crates.io …", cls: "dim", t: 200 },
      { txt: "5 candidates fetched — 308★ sabiql, 2 noise", cls: "amber", t: 1100 },
      { txt: "fan-out 1 call → Score×5 + Noul×5 + Choice", cls: "amber", t: 2100 },
      { txt: "sabiql · fit 2.8 · conf 0.76 · verified ✔", cls: "ok", t: 3200 },
      { txt: "tuitab · fit 1.6 · filtered (weak) — ghosted", cls: "dim", t: 4000 },
      { txt: "WayGet · fit 0.4 · filtered (weak) — ghosted", cls: "dim", t: 4500 },
      { txt: "search 860ms + eval 940ms · settled", cls: "ok", t: 5200 }
    ];
    var li = 0;
    function typeLine() {
      if (li >= lines.length) return;
      var L = lines[li];
      var p = document.createElement("p");
      p.className = L.cls + " amber";
      log.appendChild(p);
      var i = 0;
      function chars() {
        p.textContent = L.txt.slice(0, ++i);
        if (i < L.txt.length && !REDUCED) setTimeout(chars, 14);
        else { p.className = L.cls; li++; setTimeout(typeLine, 300); }
      }
      if (REDUCED) { p.textContent = L.txt; p.className = L.cls; li++; setTimeout(typeLine, 120); }
      else chars();
    }
    setTimeout(typeLine, REDUCED ? 0 : 200);
  }
})();