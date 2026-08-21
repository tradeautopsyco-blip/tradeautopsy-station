/**
 * TradeAutopsy chart design system — Apple HIG Charts + WWDC22 size/form rules.
 * Line = change over time. Horizontal bars = category compare (not a time series).
 * Interaction is optional: static platters have no grid; interactive plots scrub the whole area.
 * Color is never the only encoding (zero line, area sign, point shape, bar length + label).
 */
(function (global) {
  var NS = "http://www.w3.org/2000/svg";
  var reduceMotion = global.matchMedia
    ? global.matchMedia("(prefers-reduced-motion: reduce)").matches
    : false;

  function svgEl(name, attrs) {
    var el = document.createElementNS(NS, name);
    if (attrs) {
      Object.keys(attrs).forEach(function (k) {
        if (attrs[k] != null) el.setAttribute(k, attrs[k]);
      });
    }
    return el;
  }

  function niceTicks(min, max, count) {
    count = count || 3;
    if (min === max) {
      min -= 1;
      max += 1;
    }
    var span = max - min;
    var raw = span / (count - 1);
    var mag = Math.pow(10, Math.floor(Math.log(raw) / Math.LN10));
    var nice = [1, 2, 2.5, 5, 10];
    var step = mag;
    for (var i = 0; i < nice.length; i++) {
      if (raw <= mag * nice[i] * 1.15) {
        step = mag * nice[i];
        break;
      }
      step = mag * 10;
    }
    var tMin = Math.floor(min / step) * step;
    var tMax = Math.ceil(max / step) * step;
    var ticks = [];
    for (var v = tMin; v <= tMax + step * 0.01; v += step) {
      ticks.push(Math.round(v * 1000) / 1000);
    }
    return { min: tMin, max: tMax, ticks: ticks };
  }

  function pad(spec, host) {
    var interactive = !!spec.interactive;
    var staticH = spec.height || (interactive ? 148 : 48);
    return {
      l: interactive ? 8 : 2,
      r: interactive ? 40 : 2,
      t: interactive ? 10 : 4,
      b: interactive ? 22 : 4,
      w: Math.max(120, host.clientWidth || 280),
      h: staticH
    };
  }

  function xAt(i, n, p) {
    if (n <= 1) return p.l + (p.w - p.l - p.r) / 2;
    return p.l + (i / (n - 1)) * (p.w - p.l - p.r);
  }

  function yAt(v, yMin, yMax, p) {
    var plotH = p.h - p.t - p.b;
    return p.t + (1 - (v - yMin) / (yMax - yMin)) * plotH;
  }

  function clear(host) {
    while (host.firstChild) host.removeChild(host.firstChild);
  }

  function a11yList(data, format) {
    var ol = document.createElement("ol");
    ol.className = "ta-a11y";
    data.forEach(function (d) {
      var li = document.createElement("li");
      li.textContent = d.a11y || (d.x + ", " + format(d.y));
      ol.appendChild(li);
    });
    return ol;
  }

  function lineChart(host, spec) {
    if (!host) return;
    clear(host);
    var data = spec.data || [];
    var n = data.length;
    if (!n) return;

    var ys = data.map(function (d) { return d.y; });
    var yMin = spec.yMin;
    var yMax = spec.yMax;
    if (yMin == null || yMax == null) {
      var lo = Math.min.apply(null, ys);
      var hi = Math.max.apply(null, ys);
      if (spec.includeZero) {
        lo = Math.min(0, lo);
        hi = Math.max(0, hi);
      }
      var padY = (hi - lo) * 0.12 || 1;
      yMin = yMin == null ? lo - padY : yMin;
      yMax = yMax == null ? hi + padY : yMax;
    }
    var ticks = niceTicks(yMin, yMax, spec.tickCount || 3);
    if (spec.yMin == null) yMin = ticks.min;
    if (spec.yMax == null) yMax = ticks.max;
    if (spec.fixedRange) {
      yMin = spec.yMin;
      yMax = spec.yMax;
      ticks = niceTicks(yMin, yMax, spec.tickCount || 3);
    }

    var format = spec.yFormat || function (v) { return String(v); };
    var p = pad(spec, host);
    var svg = svgEl("svg", {
      class: "ta-svg" + (spec.interactive ? " is-interactive" : " is-static"),
      viewBox: "0 0 " + p.w + " " + p.h,
      preserveAspectRatio: "none",
      role: "group",
      "aria-labelledby": spec.labelledBy || null
    });
    svg.style.width = "100%";
    svg.style.height = p.h + "px";

    var plot = svgEl("g", { "aria-hidden": "true" });

    if (spec.interactive) {
      ticks.ticks.forEach(function (tv) {
        if (tv < yMin - 0.001 || tv > yMax + 0.001) return;
        var y = yAt(tv, yMin, yMax, p);
        plot.appendChild(svgEl("line", {
          class: "ta-grid",
          x1: p.l, x2: p.w - p.r, y1: y, y2: y
        }));
        var lbl = svgEl("text", {
          class: "ta-tick",
          x: p.w - 4,
          y: y + 3,
          "text-anchor": "end"
        });
        lbl.textContent = format(tv);
        plot.appendChild(lbl);
      });
    }

    if (spec.zeroLine && yMin < 0 && yMax > 0) {
      var z = yAt(0, yMin, yMax, p);
      plot.appendChild(svgEl("line", {
        class: "ta-zero",
        x1: p.l, x2: p.w - p.r, y1: z, y2: z
      }));
    }

    var coords = data.map(function (d, i) {
      return { x: xAt(i, n, p), y: yAt(d.y, yMin, yMax, p), d: d, i: i };
    });

    var lineD = coords.map(function (c, i) {
      return (i ? "L" : "M") + c.x.toFixed(1) + " " + c.y.toFixed(1);
    }).join(" ");

    if (spec.fill) {
      var zeroY = yAt(Math.max(yMin, Math.min(yMax, 0)), yMin, yMax, p);
      var areaD = lineD +
        " L" + coords[n - 1].x.toFixed(1) + " " + zeroY.toFixed(1) +
        " L" + coords[0].x.toFixed(1) + " " + zeroY.toFixed(1) + " Z";
      plot.appendChild(svgEl("path", { class: "ta-area " + (spec.areaClass || ""), d: areaD }));
    }

    plot.appendChild(svgEl("path", {
      class: "ta-line " + (spec.lineClass || ""),
      d: lineD,
      fill: "none"
    }));

    if (spec.points || spec.interactive) {
      coords.forEach(function (c) {
        var shape = spec.pointShape || "circle";
        var mark;
        if (shape === "diamond") {
          var s = spec.interactive ? 4.5 : 3;
          mark = svgEl("polygon", {
            class: "ta-point diamond",
            points: [
              c.x + "," + (c.y - s),
              (c.x + s) + "," + c.y,
              c.x + "," + (c.y + s),
              (c.x - s) + "," + c.y
            ].join(" ")
          });
        } else {
          mark = svgEl("circle", {
            class: "ta-point circle",
            cx: c.x, cy: c.y,
            r: spec.interactive ? 3.2 : 2
          });
        }
        mark.setAttribute("data-i", String(c.i));
        plot.appendChild(mark);
      });
    }

    var scrub = svgEl("line", {
      class: "ta-scrub",
      y1: p.t, y2: p.h - p.b,
      hidden: "true"
    });
    plot.appendChild(scrub);
    var focus = svgEl("circle", { class: "ta-focus", r: 5, hidden: "true" });
    plot.appendChild(focus);
    svg.appendChild(plot);

    var hit = svgEl("rect", {
      class: "ta-hit",
      x: p.l, y: p.t,
      width: p.w - p.l - p.r,
      height: p.h - p.t - p.b,
      fill: "transparent"
    });
    svg.appendChild(hit);

    host.appendChild(svg);
    host.appendChild(a11yList(data, format));

    var state = { index: n - 1, coords: coords, data: data };

    function show(i, announce) {
      if (i < 0 || i >= n) return;
      state.index = i;
      var c = coords[i];
      scrub.removeAttribute("hidden");
      focus.removeAttribute("hidden");
      scrub.setAttribute("x1", c.x);
      scrub.setAttribute("x2", c.x);
      focus.setAttribute("cx", c.x);
      focus.setAttribute("cy", c.y);
      host.querySelectorAll(".ta-point").forEach(function (pt) {
        pt.classList.toggle("is-on", pt.getAttribute("data-i") === String(i));
      });
      if (spec.onScrub) spec.onScrub(i, data[i], announce !== false);
    }

    function nearest(clientX) {
      var rect = svg.getBoundingClientRect();
      var x = ((clientX - rect.left) / rect.width) * p.w;
      var best = 0, dist = Infinity;
      coords.forEach(function (c, i) {
        var d = Math.abs(c.x - x);
        if (d < dist) { dist = d; best = i; }
      });
      return best;
    }

    if (spec.interactive) {
      svg.setAttribute("tabindex", "0");
      hit.style.cursor = "crosshair";
      function pointer(ev) {
        show(nearest(ev.clientX));
      }
      svg.addEventListener("pointerdown", function (ev) {
        svg.setPointerCapture(ev.pointerId);
        pointer(ev);
      });
      svg.addEventListener("pointermove", function (ev) {
        if (ev.buttons || ev.pointerType === "mouse") pointer(ev);
      });
      svg.addEventListener("keydown", function (ev) {
        if (ev.key === "ArrowLeft") {
          ev.preventDefault();
          show(Math.max(0, state.index - 1));
        } else if (ev.key === "ArrowRight") {
          ev.preventDefault();
          show(Math.min(n - 1, state.index + 1));
        }
      });
      show(n - 1, false);
    } else {
      scrub.setAttribute("hidden", "true");
      focus.setAttribute("hidden", "true");
    }

    return {
      highlight: function (i) { show(i, false); },
      last: function () { return state.index; }
    };
  }

  function barChart(host, spec) {
    if (!host) return;
    clear(host);
    var data = spec.data || [];
    var n = data.length;
    if (!n) return;
    var format = spec.yFormat || function (v) { return String(v); };
    var max = spec.yMax != null ? spec.yMax : Math.max.apply(null, data.map(function (d) { return d.y; }));
    if (!max) max = 1;

    var list = document.createElement("ul");
    list.className = "ta-bars" + (spec.interactive ? " is-interactive" : " is-static");
    list.setAttribute("role", spec.interactive ? "listbox" : "list");
    if (spec.labelledBy) list.setAttribute("aria-labelledby", spec.labelledBy);

    data.forEach(function (d, i) {
      var li = document.createElement("li");
      li.className = "ta-bar" + (i === 0 ? " is-on" : "");
      li.setAttribute("role", spec.interactive ? "option" : "listitem");
      if (spec.interactive) {
        li.tabIndex = i === 0 ? 0 : -1;
        li.setAttribute("aria-selected", i === 0 ? "true" : "false");
      }
      var lab = document.createElement("span");
      lab.className = "ta-bar-lab";
      lab.textContent = d.x;
      var track = document.createElement("span");
      track.className = "ta-bar-track";
      var fill = document.createElement("span");
      fill.className = "ta-bar-fill";
      var pct = Math.max(2, (d.y / max) * 100);
      fill.style.width = pct + "%";
      if (!reduceMotion) fill.style.transition = "width 280ms cubic-bezier(0.22, 1, 0.36, 1)";
      track.appendChild(fill);
      var val = document.createElement("span");
      val.className = "ta-bar-val";
      val.textContent = format(d.y);
      li.appendChild(lab);
      li.appendChild(track);
      li.appendChild(val);
      li.setAttribute("aria-label", d.a11y || (d.x + ", " + format(d.y)));
      if (spec.interactive) {
        li.addEventListener("click", function () {
          list.querySelectorAll(".ta-bar").forEach(function (row) {
            row.classList.remove("is-on");
            row.setAttribute("aria-selected", "false");
            row.tabIndex = -1;
          });
          li.classList.add("is-on");
          li.setAttribute("aria-selected", "true");
          li.tabIndex = 0;
          if (spec.onScrub) spec.onScrub(i, d, true);
        });
      }
      list.appendChild(li);
    });

    host.appendChild(list);
    return {
      highlight: function (i) {
        var row = list.children[i];
        if (row) row.click();
      }
    };
  }

  global.TACharts = { line: lineChart, bars: barChart };
})(typeof window !== "undefined" ? window : this);
