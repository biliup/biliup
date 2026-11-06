/* biliupMotion (Uncut, MIT): one-take motion film helpers for graphics (D292). Load it with the
   marker biliup:lib/motion.js (a script tag with that src). Every function is a pure function of its
   arguments (time in seconds): no state, no timers, so render(t) can be called in any order.
     biliupMotion.spring(t, "spring.snap")                  // closed-form step response, 0 → 1 (same as biliupSpring)
     biliupMotion.springs(t, [{ at: 0, to: 0 }, { at: 1, to: 200 }, { at: 1.4, to: 80 }], "spring.snap")
                                                         // sum of one spring per target change: retargets stay continuous
     biliupMotion.edges(t, [{ at: 0, left: 0, right: 100 }, { at: 1, left: 200, right: 300 }], { lead: "spring.snap", trail: "spring.settle" })
                                                         // { left, right }: the leading edge rides the faster spring (liquid tab indicator, toggle knob)
     biliupMotion.rise(t, { at: 0.5, out: 2, height: 80 }) // { y, visible }: text rising out of a mask line (wrap it in overflow: hidden), and leaving upward
     biliupMotion.flood(t, { at, x, y, r, width, height, dur: 0.4, until, to: { x, y, r } })
                                                         // { x, y, r, full }: a circle that overscales past the frame's corners, then contracts into the next shape
     biliupMotion.cover(x, y, width, height)              // the radius that covers the frame from (x, y), 6 % past the farthest corner
     biliupMotion.shape(t, [{ at, x, y, w, h, r }, { at, bleed: true }, ...], { width, height, spring: "spring.calm" })
                                                         // { x, y, w, h, r, full }: ONE rounded rect (x, y its centre) morphing key to key;
                                                         // bleed: true is the whole frame 12 % past every edge, square corners (a flood is a morph)
     biliupMotion.shapePath(ctx, s)                       // that shape as the canvas path (clip to it, then draw the footage)
     biliupMotion.footageRect(srcW, srcH, width, height, s) // { x, y, w, h }: draw the clip there: covers the frame and the shape,
                                                         // the same scale while the shape fits the frame (no zoom-in that goes soft, no flat edges).
                                                         // It is the whole frame's crop, not the shape's: a shot already on screen at another
                                                         // size (a preview fitted to a card) never switches to it on one frame (a zoom snap);
                                                         // keep drawing it at its old rect and ease to this one, or use this rect from the start
     biliupMotion.riseText(ctx, text, x, baseline, t, { at, out, size })
                                                         // draws a line rising out of its mask line and leaving upward, whole:
                                                         // the mask has room for ascenders and descenders, the exit is quick (spring.snap)
     biliupMotion.iris(open, { x, y, r, blades: 6, turn }) // blade polygons [[x, y], ...]: open 1 = no blades in the circle, 0 = closed
     biliupMotion.squeeze(xs, widths, anchor, f)           // [{ x, scale }]: letters pulled toward anchor by f (1 = as set, 0 = in the anchor), widths follow
     biliupMotion.cursor(t, [{ at, x, y, click }], { spring })  // { x, y, scale, down }: a drawn cursor's path (put clicks on click)
     biliupMotion.click(t, { from: [x, y], x, y, fire, travel: 0.35 }) // { x, y, scale, press, button, fire, ... }: one click as
                                                         // measured: glide in, rest 0.1 s, press (cursor scale 0.85, button
                                                         // scale 0.95) and release; key the change at fire (0.03 s into the
                                                         // press), never on the arrival; a list chains clicks
     biliupMotion.camera(t, [{ at, x, y, zoom, dur }], ease, o) // { x, y, zoom }: the world point at the frame's centre and its zoom;
                                                         // each move a glide (ease.glide) in log space for zoom: 0.75 s, or
                                                         // longer for a big push (o: { height } also slows a long pan), its
                                                         // key's dur wins; a spring name as ease keeps a spring; a key
                                                         // missing x, y or zoom keeps the previous key's; the next key or
                                                         // a flight may start as it settles (one main move at a time);
                                                         // { drift: 0.015 } as a 4th argument: no hold is still, zoom
                                                         // creeps 1.5 % from each key to the next (a key's own drift wins)
     biliupMotion.frameRect(left, top, w, h, { width, height, fit, pad, align, safe, keep })
                                                         // a camera key { x, y, zoom } that frames a UI rect (world px, top-left):
                                                         // by default it COVERS the frame (the UI surface is the frame, the overflow
                                                         // cropped at align { x, y }, 0.5 = centred); fit: true shows all of it, pad px around;
                                                         // safe: screen px kept clear at every edge (the safe margin, canvas min side / 15,
                                                         // 72 at 1080 x 1350): with fit the rect lands inside it (cover: pass keep); keep: [left, top, w, h], a
                                                         // box (a dialog's text, a button) that must stay whole inside that inset, zooming
                                                         // out and sliding as needed: "push in on this dialog, its words biliup" is one call
     biliupMotion.apply(ctx, cam, width, height)          // ctx.transform for that camera (save first): draw the UI in world px after it
     biliupMotion.toScreen(cam, x, y, width, height)      // [sx, sy]: a world point on screen (the cursor, a headline pinned to the UI)
     biliupMotion.morph(t, from, to, { at, dur: 0.4 })     // { x, y, w, h, r, p, full }: rect to rect (centres, corner radius) in dur s,
                                                         // eased; to: "bleed" is the whole frame past every edge. The button grows into
                                                         // the next scene's full shape; never shrink a scene to a dot
     biliupMotion.coverage(s, width, height)              // 0..1: the share of the frame inside shape s (keep it at 0.7 or more on every frame)
     biliupMotion.grow(t, { at, full, kind, origin, width, height, dur: 0.4 })
                                                         // { rects, x, y, w, h, r, scale, full, visible }: the next shot's entry in a one-take,
                                                         // no cut: on the kick at it grows out of origin (a rect in the previous shot, else
                                                         // its own centre) into a seed shape, kind "inset" | "pill" | "circle" | "strip" | "split",
                                                         // and on the kick full (default at + 0.6) bleeds past every edge; draw the old shot
                                                         // under it until full is true. visible is true from at on (the seed may still be a
                                                         // point: its clip shows only the old shot), so "full: new alone, visible: old + new
                                                         // clipped, else old" never paints the new shot unclipped before its grow (A1r8 12.98 s).
                                                         // scale (1.12 to 1) eases the new picture in: scale it about the frame's centre
     biliupMotion.growPath(ctx, g), biliupMotion.growCSS(g) // that entry as a canvas clip path, or a CSS clip-path for a frame-sized div
     biliupMotion.frameWidth(drawnWidth, zoom)            // the width to ask biliup.frame for: drawn width x camera zoom (x the content
                                                         // scale), rounded UP to a fixed step so the cache holds (never upscale footage)
   DOM, CSS and SVG are first class (backdrop-filter, SVG filters, masks, variable-font axes all export):
     biliupMotion.cameraCSS(cam, width, height)           // "matrix(...)" for a world div (transform-origin: 0 0)
     biliupMotion.shapeCSS(s)                             // { left, top, width, height, borderRadius } in px for a div
     biliupMotion.riseCSS(t, { at, out, size })           // { height, transform, visible }: a mask div of that height
                                                         // (overflow: hidden) around a span with that transform
     biliupMotion.swap(t, at, { out: 0.16, in: 0.26, gap: 0.06, blur: 5, lift: 10 })
                                                         // { out: { alpha, blur, dy, visible }, in: {...} }: the content inside a morphing
                                                         // shape swaps on its own timing: the old content leaves (up, a few px of blur),
                                                         // then the new arrives; draw each with ctx.globalAlpha, ctx.filter = blur(px), dy
     biliupMotion.beat(n, { bpm: 120, offset: 0 })         // seconds of beat n
     biliupMotion.loop(t, duration)                        // t wrapped into [0, duration): the last frame meets the first
     biliupMotion.ease.inOut(p), .out(p), .in(p), .glide(p) // cubic eases for opacity or colour; glide: the calm move (eases off,
                                                         // slows down 1.7x as long, no overshoot): a move of about 0.8 s
     biliupMotion.mix(a, b, p), biliupMotion.clamp(v, lo, hi), biliupMotion.mixColor("#000000", "#ffffff", p)
*/
(function () {
  "use strict";
  var PRESETS = {
    "spring.snap": { stiffness: 400, damping: 36 },
    "spring.settle": { stiffness: 170, damping: 20 },
    "spring.land": { stiffness: 220, damping: 17 },
    "spring.bounce": { stiffness: 300, damping: 9 },
    "spring.calm": { stiffness: 90, damping: 18 },
    spring: { stiffness: 170, damping: 20 },
  };
  function params(p) {
    var s = typeof p === "string" ? PRESETS[p] || PRESETS.spring : p || PRESETS["spring.snap"];
    var k = typeof s.stiffness === "number" && s.stiffness > 0 ? s.stiffness : 170;
    var c = typeof s.damping === "number" && s.damping >= 0 ? s.damping : 20;
    var m = typeof s.mass === "number" && s.mass > 0 ? s.mass : 1;
    var v = typeof s.velocity === "number" && Number.isFinite(s.velocity) ? s.velocity : 0;
    return { k: k, c: c, m: m, v: v };
  }
  // The damped spring's step response (0 at t <= 0, settling at 1), as in packages/core/src/visual/spring.ts.
  function spring(t, preset) {
    var p = params(preset);
    if (!(t > 0)) return 0;
    var w0 = Math.sqrt(p.k / p.m);
    var zeta = p.c / (2 * Math.sqrt(p.k * p.m));
    if (Math.abs(zeta - 1) < 1e-6) return 1 + (-1 + (p.v - w0) * t) * Math.exp(-w0 * t);
    if (zeta < 1) {
      var wd = w0 * Math.sqrt(1 - zeta * zeta);
      var b = (p.v - zeta * w0) / wd;
      return 1 + Math.exp(-zeta * w0 * t) * (-Math.cos(wd * t) + b * Math.sin(wd * t));
    }
    var root = w0 * Math.sqrt(zeta * zeta - 1);
    var r1 = -zeta * w0 + root;
    var r2 = -zeta * w0 - root;
    var c2 = (p.v + r1) / (r2 - r1);
    var c1 = -1 - c2;
    return 1 + c1 * Math.exp(r1 * t) + c2 * Math.exp(r2 * t);
  }
  function sorted(keys) {
    return (keys || []).slice().sort(function (a, b) {
      return a.at - b.at;
    });
  }
  // A value with many targets: the first key's value, plus one spring per change, each from its own key's time.
  function springs(t, keys, preset, field) {
    var ks = sorted(keys);
    if (!ks.length) return 0;
    var f = field || "to";
    var v = ks[0][f];
    for (var i = 1; i < ks.length; i++) {
      var d = ks[i][f] - ks[i - 1][f];
      if (d) v += d * spring(t - ks[i].at, ks[i].spring || preset);
    }
    return v;
  }
  // Two edges of one shape: on each change, the edge moving in the direction of travel leads (faster spring).
  function edges(t, keys, o) {
    var ks = sorted(keys);
    var lead = (o && o.lead) || "spring.snap";
    var trail = (o && o.trail) || { stiffness: 140, damping: 22 };
    if (!ks.length) return { left: 0, right: 0 };
    var left = ks[0].left;
    var right = ks[0].right;
    for (var i = 1; i < ks.length; i++) {
      var dl = ks[i].left - ks[i - 1].left;
      var dr = ks[i].right - ks[i - 1].right;
      var forward = dl + dr >= 0;
      left += dl * spring(t - ks[i].at, forward ? trail : lead);
      right += dr * spring(t - ks[i].at, forward ? lead : trail);
    }
    return { left: left, right: right };
  }
  // Text rising out of a mask line: y is the offset inside an overflow: hidden box of `height`.
  function rise(t, o) {
    var h = o.height || 100;
    var s = o.spring || "spring.snap";
    var y = h * (1 - spring(t - o.at, s));
    if (o.out !== undefined) y -= h * spring(t - o.out, o.outSpring || s);
    return { y: y, visible: t >= o.at && (o.out === undefined || y > -h + 0.5) };
  }
  function cover(x, y, w, h) {
    var dx = Math.max(x, w - x);
    var dy = Math.max(y, h - y);
    return Math.sqrt(dx * dx + dy * dy) * 1.06;
  }
  function easeInOut(p) {
    p = p <= 0 ? 0 : p >= 1 ? 1 : p;
    return p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2;
  }
  function easeOut(p) {
    p = p <= 0 ? 0 : p >= 1 ? 1 : p;
    return 1 - Math.pow(1 - p, 3);
  }
  function easeIn(p) {
    p = p <= 0 ? 0 : p >= 1 ? 1 : p;
    return p * p * p;
  }
  // The calm glide measured on the reference films: it eases off rest (top speed a third of the
  // way in), slows down about 1.7x as long as it sped up and lands with no overshoot (velocity
  // 12p(1 - p)^2: 10 to 90 % of the travel in 54 % of the time, the last 10 % in the final 32 %).
  function glide(p) {
    p = p <= 0 ? 0 : p >= 1 ? 1 : p;
    return p * p * (6 - 8 * p + 3 * p * p);
  }
  // A flood: the shape grows past the corners in `dur` s (it never covers half the frame in one
  // frame), stays full until `until`, then contracts into `to` over `dur`.
  function flood(t, o) {
    var dur = o.dur || 0.4;
    var R = cover(o.x, o.y, o.width, o.height);
    var p = easeInOut((t - o.at) / dur);
    var x = o.x, y = o.y, r = o.r + (R - o.r) * p;
    if (o.until !== undefined && o.to && t > o.until) {
      var q = easeInOut((t - o.until) / dur);
      var R2 = cover(o.to.x, o.to.y, o.width, o.height);
      x = o.to.x;
      y = o.to.y;
      r = R2 + (o.to.r - R2) * q;
    }
    return { x: x, y: y, r: r, full: r >= cover(x, y, o.width, o.height) / 1.06 };
  }
  // D296: one rounded rect morphing between keys (centre x, y, size w, h, corner r); bleed keys
  // fill the frame past its edges. Each field is a sum of springs (springs), so it's continuous.
  function shape(t, keys, o) {
    o = o || {};
    var W = o.width || 1080;
    var H = o.height || 1350;
    var pre = o.spring || "spring.calm";
    var ks = sorted(keys).map(function (k) {
      if (!k.bleed) return k;
      return { at: k.at, x: W / 2, y: H / 2, w: W * 1.24, h: H * 1.24, r: 0, spring: k.spring };
    });
    if (!ks.length) return { x: W / 2, y: H / 2, w: W, h: H, r: 0, full: true };
    var out = {};
    ["x", "y", "w", "h", "r"].forEach(function (f) {
      out[f] = springs(t, ks, pre, f);
    });
    out.w = Math.max(0, out.w);
    out.h = Math.max(0, out.h);
    out.r = clamp(out.r, 0, Math.min(out.w, out.h) / 2);
    out.full = out.x - out.w / 2 <= 0 && out.y - out.h / 2 <= 0 && out.x + out.w / 2 >= W && out.y + out.h / 2 >= H;
    return out;
  }
  function shapePath(ctx, s) {
    ctx.beginPath();
    roundRect(ctx, s);
  }
  function roundRect(ctx, s) {
    var x = s.x - s.w / 2, y = s.y - s.h / 2, r = clamp(s.r || 0, 0, Math.min(s.w, s.h) / 2);
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + s.w, y, x + s.w, y + s.h, r);
    ctx.arcTo(x + s.w, y + s.h, x, y + s.h, r);
    ctx.arcTo(x, y + s.h, x, y, r);
    ctx.arcTo(x, y, x + s.w, y, r);
    ctx.closePath();
  }
  // Where to draw a srcW × srcH clip so it covers the frame and the shape's whole extent, centred on
  // the frame: while the shape stays inside the frame the scale never changes (no zoom on resize).
  function footageRect(srcW, srcH, W, H, s) {
    var x0 = 0, y0 = 0, x1 = W, y1 = H;
    if (s) {
      x0 = Math.min(0, s.x - s.w / 2);
      y0 = Math.min(0, s.y - s.h / 2);
      x1 = Math.max(W, s.x + s.w / 2);
      y1 = Math.max(H, s.y + s.h / 2);
    }
    var bw = 2 * Math.max(W / 2 - x0, x1 - W / 2);
    var bh = 2 * Math.max(H / 2 - y0, y1 - H / 2);
    var k = Math.max(bw / srcW, bh / srcH);
    return { x: W / 2 - (srcW * k) / 2, y: H / 2 - (srcH * k) / 2, w: srcW * k, h: srcH * k };
  }
  // A line rising out of its mask line and leaving upward, never cut while it rests: the mask runs
  // from 1.05 × size above the baseline to 0.35 × size below it, the line travels that whole box.
  function riseText(ctx, text, x, baseline, t, o) {
    var size = o.size || 96;
    var top = baseline - size * 1.05;
    var h = size * 1.4;
    var r = rise(t, { at: o.at, out: o.out, height: h, spring: o.spring || "spring.snap", outSpring: o.outSpring || "spring.snap" });
    if (!r.visible) return false;
    ctx.save();
    ctx.beginPath();
    ctx.rect(-1e5, top, 2e5, h);
    ctx.clip();
    ctx.fillText(text, x, baseline + r.y);
    ctx.restore();
    return true;
  }
  // Iris: `blades` blades around a regular polygon aperture. Each blade is its two aperture vertices,
  // both edge extensions out to the circle, and the short arc between them.
  function iris(open, o) {
    var n = Math.max(3, Math.round(o.blades || 6));
    var R = o.r;
    var a = Math.max(0, Math.min(1, open)) * R * 1.0001;
    var turn = (o.turn === undefined ? Math.PI / 3 : o.turn) * (1 - Math.max(0, Math.min(1, open)));
    var V = [];
    for (var i = 0; i < n; i++) {
      var ang = (i / n) * Math.PI * 2 + turn;
      V.push([o.x + Math.cos(ang) * a, o.y + Math.sin(ang) * a]);
    }
    // Where the ray from p along d leaves the circle.
    function hit(p, d) {
      var px = p[0] - o.x, py = p[1] - o.y;
      var len = Math.hypot(d[0], d[1]) || 1;
      var ux = d[0] / len, uy = d[1] / len;
      var b = px * ux + py * uy;
      var c = px * px + py * py - R * R;
      var s = -b + Math.sqrt(Math.max(0, b * b - c));
      return [p[0] + ux * s, p[1] + uy * s];
    }
    var blades = [];
    for (var j = 0; j < n; j++) {
      var A = V[j], B = V[(j + 1) % n], P = V[(j - 1 + n) % n];
      // At open 0 every vertex is the centre: the edge direction comes from the angles.
      var angA = (j / n) * Math.PI * 2 + turn, angB = ((j + 1) / n) * Math.PI * 2 + turn, angP = ((j - 1) / n) * Math.PI * 2 + turn;
      var dAB = a > 1e-6 ? [B[0] - A[0], B[1] - A[1]] : [Math.cos(angB) - Math.cos(angA), Math.sin(angB) - Math.sin(angA)];
      var dPA = a > 1e-6 ? [A[0] - P[0], A[1] - P[1]] : [Math.cos(angA) - Math.cos(angP), Math.sin(angA) - Math.sin(angP)];
      var Q1 = hit(B, dAB);
      var Q0 = hit(A, dPA);
      var poly = [A, B, Q1];
      // The short arc from Q1 back to Q0.
      var a1 = Math.atan2(Q1[1] - o.y, Q1[0] - o.x);
      var a0 = Math.atan2(Q0[1] - o.y, Q0[0] - o.x);
      var da = a0 - a1;
      while (da > Math.PI) da -= Math.PI * 2;
      while (da < -Math.PI) da += Math.PI * 2;
      for (var k = 1; k < 8; k++) poly.push([o.x + Math.cos(a1 + (da * k) / 8) * R, o.y + Math.sin(a1 + (da * k) / 8) * R]);
      poly.push(Q0);
      blades.push(poly);
    }
    return blades;
  }
  // Wordmark squeeze into its own period: every letter moves toward the anchor by the same factor
  // and its drawn width follows, so neighbours stay touching.
  function squeeze(xs, widths, anchor, f) {
    var out = [];
    for (var i = 0; i < xs.length; i++) out.push({ x: anchor + (xs[i] - anchor) * f, scale: f, width: (widths[i] || 0) * f });
    return out;
  }
  // A drawn cursor: x and y ride springs from waypoint to waypoint; a click dips the scale and comes back.
  function cursor(t, path, o) {
    var s = (o && o.spring) || { stiffness: 120, damping: 22 };
    var ks = sorted(path);
    var x = springs(t, ks, s, "x");
    var y = springs(t, ks, s, "y");
    var scale = 1;
    var down = false;
    for (var i = 0; i < ks.length; i++) {
      if (!ks[i].click) continue;
      var c = ks[i].click === true ? ks[i].at : ks[i].click;
      var hold = ks[i].hold || 0.12;
      if (t >= c && t < c + hold) down = true;
      scale -= 0.15 * (spring(t - c, "spring.snap") - spring(t - c - hold, "spring.snap"));
    }
    return { x: x, y: y, scale: scale, down: down };
  }
  // A believable click, timed on reference films at 60 fps: the cursor glides in and decelerates
  // (travel), lies still (rest 0.1 s), presses (cursor to 0.85 in 0.09 s, the button to 0.95), holds
  // 0.02 s and releases in 0.13 s. What the click causes starts at fire, 0.03 s into the press, never
  // on the arrival. Give fire (the moment the change starts) and the glide is scheduled back from it,
  // or at (the moment the glide starts); a fire under 0.48 s starts the glide before t = 0. A list of
  // clicks (in time order) chains them, each from the last one's target; one keyed too soon for a
  // full glide leaves after the last press (a faster glide, 0.12 s at least) as that press eases out.
  function click(t, o) {
    if (Array.isArray(o)) {
      var prev = null;
      var cur = null;
      var held = 0;
      var last = null;
      for (var i = 0; i < o.length; i++) {
        var spec = {};
        for (var k in o[i]) spec[k] = o[i][k];
        if (!spec.from && prev) spec.from = [prev.x, prev.y];
        // Too close to the last click for a full glide: it leaves only after that press, a faster glide.
        if (last && spec.fire != null && spec.travel == null) {
          var room = spec.fire - 0.03 - (spec.rest == null ? 0.1 : spec.rest) - last.release;
          if (room < 0.35) spec.travel = Math.max(0.12, room);
        }
        var c = click(t, spec);
        if (!cur || t >= c.start) cur = c;
        // A click that glides off before the last one's release ends keeps that press easing out (no one-frame pop).
        held = Math.max(held, c.press);
        prev = spec;
        last = c;
      }
      if (held > cur.press) {
        var out = {};
        for (var q in cur) out[q] = cur[q];
        out.press = held;
        out.scale = 1 - 0.15 * held;
        out.button = 1 - 0.05 * held;
        return out;
      }
      return cur;
    }
    var travel = o.travel == null ? 0.35 : o.travel;
    var rest = o.rest == null ? 0.1 : o.rest;
    var lead = 0.03, down = 0.09, hold = 0.02, up = 0.13;
    var start = o.fire != null ? o.fire - lead - rest - travel : o.at || 0;
    var arrive = start + travel;
    var press = arrive + rest;
    var fire = press + lead;
    var release = press + down + hold;
    var end = release + up;
    var from = o.from || [o.x, o.y];
    // Smooth start, long ease-out landing: most of the distance early, the last pixels slow.
    var p = clamp((t - start) / Math.max(travel, 1e-6), 0, 1);
    var e = easeOut(p * p * (3 - 2 * p) * 0.5 + p * 0.5);
    // A slight arc, as a hand moves: off the straight line by bend x the distance at mid-travel.
    var dx = o.x - from[0];
    var dy = o.y - from[1];
    var bow = (o.bend == null ? 0.08 : o.bend) * Math.sin(Math.PI * e);
    var x = from[0] + dx * e - dy * bow;
    var y = from[1] + dy * e + dx * bow;
    var pr = 0;
    if (t >= press && t < press + down) pr = easeInOut((t - press) / down);
    else if (t >= press + down && t < release) pr = 1;
    else if (t >= release && t < end) pr = 1 - easeInOut((t - release) / up);
    return {
      x: x,
      y: y,
      scale: 1 - 0.15 * pr,
      press: pr,
      button: 1 - 0.05 * pr,
      down: t >= press && t < release,
      fired: t >= fire,
      start: start,
      arrive: arrive,
      pressAt: press,
      fire: fire,
      release: release,
      end: end,
    };
  }
  // Screen-studio camera: (x, y) is the world point at the frame's centre. Missing fields carry
  // over from the previous key; zoom springs in log space, so a 1x to 3x push moves evenly.
  //
  // Default (no preset, or "glide"): each key's move is a calm glide of its own dur (key.dur, else
  // o.dur, else measured from the move): at least 0.75 s, longer for a big push (zoom at most about
  // 1.2 ln/s at its peak) or, with o.height, a long pan (about 1.3 frame heights/s at its peak), up
  // to 2 s. A spring preset ("spring.settle", ...) keeps the old spring camera.
  function camera(t, keys, preset, o) {
    var ks = sorted(keys);
    if (!ks.length) return { x: 0, y: 0, zoom: 1 };
    var full = [];
    for (var i = 0; i < ks.length; i++) {
      var p = full[i - 1] || { x: 0, y: 0, zoom: 1 };
      var k = ks[i];
      full.push({ at: k.at, spring: k.spring, drift: k.drift, dur: k.dur, x: k.x === undefined ? p.x : k.x, y: k.y === undefined ? p.y : k.y, lz: Math.log(Math.max(1e-3, k.zoom === undefined ? p.zoom : k.zoom)), zoom: k.zoom === undefined ? p.zoom : k.zoom });
    }
    if (preset && preset !== "glide") return { x: springs(t, full, preset, "x"), y: springs(t, full, preset, "y"), zoom: Math.exp(springs(t, full, preset, "lz") + drift(t, full, preset, o)) };
    var out = { x: full[0].x, y: full[0].y, lz: full[0].lz };
    for (var j = 1; j < full.length; j++) {
      var a = full[j - 1], b = full[j];
      b.dur = Math.max(1e-3, b.dur === undefined ? glideDur(a, b, o) : Number(b.dur) || 0);
      var g = glide((t - b.at) / b.dur);
      if (!g) continue;
      out.x += (b.x - a.x) * g;
      out.y += (b.y - a.y) * g;
      out.lz += (b.lz - a.lz) * g;
    }
    return { x: out.x, y: out.y, zoom: Math.exp(out.lz + drift(t, full, "glide", o)) };
  }
  function glideDur(a, b, o) {
    if (o && o.dur > 0) return o.dur;
    var d = Math.max(0.75, (1.78 * Math.abs(b.lz - a.lz)) / 1.2);
    if (o && o.height > 0) d = Math.max(d, (1.78 * Math.hypot(b.x - a.x, b.y - a.y) * Math.max(a.zoom, b.zoom)) / (1.3 * o.height));
    return Math.min(2, d);
  }
  // A hold that never freezes: from each key to the next, zoom creeps up by d (linear, in log
  // space); from the next key on, that creep springs away under the key's own move, so it stays
  // continuous. Nothing after the last key (a key at the loop's end keeps the seam).
  function drift(t, ks, preset, o) {
    var base = (o && o.drift) || 0;
    var sum = 0;
    for (var i = 0; i + 1 < ks.length; i++) {
      var d = ks[i].drift === undefined ? base : ks[i].drift;
      var a = ks[i].at;
      var b = ks[i + 1].at;
      if (!d || !(b > a) || t <= a) continue;
      var lz = Math.log(1 + d);
      var away = preset === "glide" ? glide((t - b) / ks[i + 1].dur) : spring(t - b, ks[i + 1].spring || preset);
      sum += t < b ? (lz * (t - a)) / (b - a) : lz * (1 - away);
    }
    return sum;
  }
  // A camera key framing a UI rect (left, top, w, h in world px): cover (default) fills the frame
  // with it, cropping the overflow at align; fit shows all of it with pad px around. Round 4's A2:
  // pushes into a dialog cut its button and card text at the frame edge three rounds running, so
  // safe (screen px at each edge) and keep (a world box that stays whole inside that inset).
  function frameRect(left, top, w, h, o) {
    o = o || {};
    var W = o.width || 1080;
    var H = o.height || 1350;
    var pad = o.pad || 0;
    var s = Math.max(0, Math.min(o.safe || 0, W / 2 - 1, H / 2 - 1));
    var al = o.align || {};
    var ax = al.x === undefined ? 0.5 : al.x;
    var ay = al.y === undefined ? 0.5 : al.y;
    var zw = (o.fit ? W - 2 * s : W) / Math.max(1, w + 2 * pad);
    var zh = (o.fit ? H - 2 * s : H) / Math.max(1, h + 2 * pad);
    var zoom = o.fit ? Math.min(zw, zh) : Math.max(zw, zh);
    var k = o.keep;
    if (k) zoom = Math.min(zoom, (W - 2 * s) / Math.max(1, k[2]), (H - 2 * s) / Math.max(1, k[3]));
    var vw = W / zoom;
    var vh = H / zoom;
    // The visible window slides inside the rect (cover) or centres on it (fit) along align.
    var x = left + (o.fit ? w / 2 : vw / 2 + (w - vw) * ax);
    var y = top + (o.fit ? h / 2 : vh / 2 + (h - vh) * ay);
    if (k) {
      // Then slides just enough that the kept box sits inside the inset.
      var hx = (W / 2 - s) / zoom;
      var hy = (H / 2 - s) / zoom;
      x = Math.min(Math.max(x, k[0] + k[2] - hx), k[0] + hx);
      y = Math.min(Math.max(y, k[1] + k[3] - hy), k[1] + hy);
    }
    return { x: x, y: y, zoom: zoom };
  }
  function apply(ctx, cam, W, H) {
    var z = cam.zoom;
    ctx.transform(z, 0, 0, z, W / 2 - cam.x * z, H / 2 - cam.y * z);
  }
  function toScreen(cam, x, y, W, H) {
    return [(x - cam.x) * cam.zoom + W / 2, (y - cam.y) * cam.zoom + H / 2];
  }
  // Rect to rect (centre x, y, size w, h, corner r) over dur s from at, eased in and out.
  function morph(t, from, to, o) {
    o = o || {};
    var W = o.width || 1080;
    var H = o.height || 1350;
    var bleed = function (k) {
      return k === "bleed" || (k && k.bleed) ? { x: W / 2, y: H / 2, w: W * 1.24, h: H * 1.24, r: 0 } : k;
    };
    var a = bleed(from);
    var b = bleed(to);
    var p = easeInOut((t - (o.at || 0)) / (o.dur || 0.4));
    var out = { p: p };
    ["x", "y", "w", "h", "r"].forEach(function (f) {
      out[f] = mix(a[f] || 0, b[f] || 0, p);
    });
    out.r = clamp(out.r, 0, Math.min(out.w, out.h) / 2);
    out.full = out.x - out.w / 2 <= 0 && out.y - out.h / 2 <= 0 && out.x + out.w / 2 >= W && out.y + out.h / 2 >= H;
    return out;
  }
  // DOM twins of apply, shapePath and riseText.
  function cameraCSS(cam, W, H) {
    var z = cam.zoom;
    return "matrix(" + z + ", 0, 0, " + z + ", " + (W / 2 - cam.x * z) + ", " + (H / 2 - cam.y * z) + ")";
  }
  function shapeCSS(s) {
    var r = clamp(s.r || 0, 0, Math.min(s.w, s.h) / 2);
    return { left: s.x - s.w / 2, top: s.y - s.h / 2, width: s.w, height: s.h, borderRadius: r };
  }
  function riseCSS(t, o) {
    var size = o.size || 96;
    var h = size * 1.4;
    var r = rise(t, { at: o.at, out: o.out, height: h, spring: o.spring || "spring.snap", outSpring: o.outSpring || "spring.snap" });
    return { height: h, transform: "translateY(" + Math.round(r.y * 100) / 100 + "px)", visible: r.visible };
  }
  // Content swapping inside a morphing shape: the old content leaves first (alpha down, lifting,
  // a short blur), the new one arrives after a gap; both quick, so the swap reads as one change.
  function swap(t, at, o) {
    o = o || {};
    var dOut = o.out || 0.16;
    var dIn = o.in || 0.26;
    var gap = o.gap === undefined ? 0.06 : o.gap;
    var blur = o.blur === undefined ? 5 : o.blur;
    var lift = o.lift === undefined ? 10 : o.lift;
    var po = easeIn((t - at) / dOut);
    var pi = easeOut((t - at - dOut - gap) / dIn);
    return {
      out: { alpha: 1 - po, blur: blur * po, dy: -lift * po, visible: po < 1 },
      in: { alpha: pi, blur: blur * (1 - pi), dy: lift * (1 - pi), visible: pi > 0 },
    };
  }
  // The share of the W x H frame inside the rounded rect s (its corners' cut-outs subtracted where
  // they fall inside the frame).
  function coverage(s, W, H) {
    var x0 = s.x - s.w / 2, y0 = s.y - s.h / 2, x1 = s.x + s.w / 2, y1 = s.y + s.h / 2;
    var iw = Math.max(0, Math.min(W, x1) - Math.max(0, x0));
    var ih = Math.max(0, Math.min(H, y1) - Math.max(0, y0));
    var area = iw * ih;
    var r = clamp(s.r || 0, 0, Math.min(s.w, s.h) / 2);
    var cut = r * r * (1 - Math.PI / 4);
    [[x0, y0], [x1 - r, y0], [x0, y1 - r], [x1 - r, y1 - r]].forEach(function (c) {
      if (c[0] >= 0 && c[1] >= 0 && c[0] + r <= W && c[1] + r <= H) area -= cut;
    });
    return clamp(area / (W * H), 0, 1);
  }
  // A shape-grow entry: the next shot in a one-take never cuts in, it grows out of a shape. Phase 1
  // (at, dur s): origin to the seed shape; phase 2 (full, dur s): the seed to past every edge.
  function inRect(px, py, s) {
    var x0 = s.x - s.w / 2, y0 = s.y - s.h / 2, x1 = s.x + s.w / 2, y1 = s.y + s.h / 2;
    if (px < x0 || px > x1 || py < y0 || py > y1) return false;
    var r = clamp(s.r || 0, 0, Math.min(s.w, s.h) / 2);
    var cx = clamp(px, x0 + r, x1 - r), cy = clamp(py, y0 + r, y1 - r);
    return (px - cx) * (px - cx) + (py - cy) * (py - cy) <= r * r + 1e-6;
  }
  function growSeeds(kind, W, H, o) {
    var B = { x: W / 2, y: H / 2, w: W * 1.24, h: H * 1.24, r: 0 };
    if (kind === "circle") {
      var d = Math.min(W, H) * 0.6, D = 2 * cover(W / 2, H / 2, W, H);
      return { from: [{ x: W / 2, y: H / 2, w: 0, h: 0, r: 0 }], seed: [{ x: W / 2, y: H / 2, w: d, h: d, r: d / 2 }], to: [{ x: W / 2, y: H / 2, w: D, h: D, r: D / 2 }] };
    }
    if (kind === "pill") {
      var ph = H * 0.25;
      return { from: [{ x: W / 2, y: H / 2, w: 0, h: 0, r: 0 }], seed: [{ x: W / 2, y: H / 2, w: W * 0.86, h: ph, r: ph / 2 }], to: [B] };
    }
    if (kind === "strip") {
      var left = o.side === "left", sw = W * 0.65;
      var edge = left ? -W * 0.12 : W * 1.12;
      return { from: [{ x: edge, y: H / 2, w: 0, h: H * 1.24, r: 0 }], seed: [{ x: left ? edge + sw / 2 : edge - sw / 2, y: H / 2, w: sw, h: H * 1.24, r: 0 }], to: [B] };
    }
    if (kind === "split") {
      // The seed leaves a clear window of the old shot (the middle 40 %), not a sliver: round 2's A1
      // held an 8 % strip of the old shot across the frame for 0.2 s, which read as a torn block wipe.
      var bw = W * 1.24, top = -H * 0.12, bot = H * 1.12, sh = H * 0.12 + H * 0.3, fh = H * 0.12 + H * 0.52;
      return {
        from: [{ x: W / 2, y: top, w: bw, h: 0, r: 0 }, { x: W / 2, y: bot, w: bw, h: 0, r: 0 }],
        seed: [{ x: W / 2, y: top + sh / 2, w: bw, h: sh, r: 0 }, { x: W / 2, y: bot - sh / 2, w: bw, h: sh, r: 0 }],
        to: [{ x: W / 2, y: top + fh / 2, w: bw, h: fh, r: 0 }, { x: W / 2, y: bot - fh / 2, w: bw, h: fh, r: 0 }],
      };
    }
    var iw = W * 0.7, ih = H * 0.58;
    return { from: [{ x: W / 2, y: H / 2, w: 0, h: 0, r: 0 }], seed: [{ x: W / 2, y: H / 2, w: iw, h: ih, r: Math.min(W, H) * 0.024 }], to: [B] };
  }
  function grow(t, o) {
    o = o || {};
    var W = o.width || 1080;
    var H = o.height || 1350;
    // dur 0 is an instant cut-in; a negative one counts as 0; none is 0.4 s.
    var dur = o.dur === undefined || o.dur === null ? 0.4 : Math.max(0, Number(o.dur) || 0);
    var at = o.at || 0;
    // The bleed never comes before the seed (a full before at would leave the scale stuck high).
    var fullAt = o.full === undefined ? at + 0.6 : Math.max(Number(o.full) || 0, at);
    var g = growSeeds(o.kind || "inset", W, H, o);
    var from = o.origin ? [o.origin] : g.from;
    var ramp = function (start, length, ease) {
      return length > 0 ? ease((t - start) / length) : t >= start ? 1 : 0;
    };
    var p1 = ramp(at, dur, easeInOut);
    var p2 = ramp(fullAt, dur, easeInOut);
    var rects = g.seed.map(function (sd, i) {
      var a = from[Math.min(i, from.length - 1)], b = g.to[i], out = {};
      ["x", "y", "w", "h", "r"].forEach(function (f) {
        out[f] = mix(mix(a[f] || 0, sd[f] || 0, p1), b[f] || 0, p2);
      });
      out.r = clamp(out.r, 0, Math.min(out.w, out.h) / 2);
      return out;
    });
    var full = t >= at;
    for (var gx = 0; gx <= 8 && full; gx++)
      for (var gy = 0; gy <= 10 && full; gy++) {
        var px = (gx / 8) * W, py = (gy / 10) * H;
        full = rects.some(function (s) {
          return inRect(px, py, s);
        });
      }
    var k = o.scale === undefined ? 1.12 : o.scale;
    var first = rects[0];
    return {
      rects: rects,
      x: first.x,
      y: first.y,
      w: first.w,
      h: first.h,
      r: first.r,
      scale: 1 + (k - 1) * (1 - ramp(at, fullAt + dur - at, easeOut)),
      full: full,
      // From the kick on, even while the seed is still a point: a graphic that draws the old shot
      // under the clip only while visible otherwise falls through to the new shot unclipped for the
      // grow's first frames (A1r8 12.98 and 15.48 s: two frames of the next shot before its grow).
      visible: t >= at,
    };
  }
  function growPath(ctx, g) {
    ctx.beginPath();
    g.rects.forEach(function (s) {
      roundRect(ctx, s);
    });
  }
  function growCSS(g) {
    var n = function (v) {
      return Math.round(v * 100) / 100;
    };
    var d = g.rects
      .map(function (s) {
        var x = s.x - s.w / 2, y = s.y - s.h / 2, r = clamp(s.r || 0, 0, Math.min(s.w, s.h) / 2), a = " A " + n(r) + " " + n(r) + " 0 0 1 ";
        return "M " + n(x + r) + " " + n(y) + " H " + n(x + s.w - r) + a + n(x + s.w) + " " + n(y + r) + " V " + n(y + s.h - r) + a + n(x + s.w - r) + " " + n(y + s.h) + " H " + n(x + r) + a + n(x) + " " + n(y + s.h - r) + " V " + n(y + r) + a + n(x + r) + " " + n(y) + " Z";
      })
      .join(" ");
    return 'path("' + d + '")';
  }
  var FRAME_STEPS = [360, 540, 720, 1080, 1440, 1920];
  function frameWidth(drawn, zoom) {
    var need = (drawn || 0) * (zoom || 1);
    for (var i = 0; i < FRAME_STEPS.length; i++) if (FRAME_STEPS[i] >= need) return FRAME_STEPS[i];
    return FRAME_STEPS[FRAME_STEPS.length - 1];
  }
  function beat(n, o) {
    var bpm = (o && o.bpm) || 120;
    return ((o && o.offset) || 0) + (n * 60) / bpm;
  }
  function loop(t, d) {
    return d > 0 ? ((t % d) + d) % d : t;
  }
  function mix(a, b, p) {
    return a + (b - a) * p;
  }
  function clamp(v, lo, hi) {
    return v < lo ? lo : v > hi ? hi : v;
  }
  function hex(c) {
    var h = String(c).replace("#", "");
    if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
    var n = parseInt(h, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  function mixColor(a, b, p) {
    var A = hex(a), B = hex(b), q = clamp(p, 0, 1);
    return "rgb(" + Math.round(mix(A[0], B[0], q)) + ", " + Math.round(mix(A[1], B[1], q)) + ", " + Math.round(mix(A[2], B[2], q)) + ")";
  }
  window.biliupMotion = {
    spring: spring,
    springs: springs,
    edges: edges,
    rise: rise,
    flood: flood,
    cover: cover,
    shape: shape,
    shapePath: shapePath,
    footageRect: footageRect,
    riseText: riseText,
    iris: iris,
    squeeze: squeeze,
    cursor: cursor,
    click: click,
    camera: camera,
    frameRect: frameRect,
    apply: apply,
    toScreen: toScreen,
    morph: morph,
    coverage: coverage,
    swap: swap,
    grow: grow,
    growPath: growPath,
    growCSS: growCSS,
    frameWidth: frameWidth,
    cameraCSS: cameraCSS,
    shapeCSS: shapeCSS,
    riseCSS: riseCSS,
    beat: beat,
    loop: loop,
    ease: { inOut: easeInOut, out: easeOut, in: easeIn, glide: glide },
    mix: mix,
    clamp: clamp,
    mixColor: mixColor,
    presets: PRESETS,
  };
})();
