/* biliupSpring (Uncut, MIT): the same closed-form damped spring as Biliup's keys and moves (D192,
   packages/core/src/visual/spring.ts), for graphics (D193). Load it with the marker
   biliup:lib/spring.js (a script tag with that src). A pure function of time: no state, no timers.
     biliupSpring(t, "spring.land")          // value at t seconds, 0 at the start, settling at 1
     biliupSpring(t, { stiffness: 170, damping: 20, mass: 1, velocity: 0 })
     biliupSpring.state(t, spring)           // { value, velocity }
     biliupSpring.settle(spring)             // seconds until it stays within 0.001 of 1
     biliupSpring.ease(spring)(p)            // as an ease over p in [0, 1], fitted to its settle time
     biliupSpring.presets                    // spring.snap, spring.settle (= spring), spring.land, spring.bounce
*/
(function () {
  "use strict";
  var PRESETS = {
    "spring.snap": { stiffness: 400, damping: 36 },
    "spring.settle": { stiffness: 170, damping: 20 },
    "spring.land": { stiffness: 220, damping: 17 },
    "spring.bounce": { stiffness: 300, damping: 9 },
    spring: { stiffness: 170, damping: 20 },
  };
  function params(p) {
    var s = typeof p === "string" ? PRESETS[p] || PRESETS.spring : p || PRESETS.spring;
    var pos = function (n, d) {
      return typeof n === "number" && Number.isFinite(n) && n > 0 ? n : d;
    };
    return {
      stiffness: pos(s.stiffness, 170),
      damping: typeof s.damping === "number" && Number.isFinite(s.damping) && s.damping >= 0 ? s.damping : 20,
      mass: pos(s.mass, 1),
      velocity: typeof s.velocity === "number" && Number.isFinite(s.velocity) ? s.velocity : 0,
    };
  }
  function state(t, spring) {
    var p = params(spring);
    var k = p.stiffness, c = p.damping, m = p.mass, v0 = p.velocity;
    if (!(t > 0)) return { value: 0, velocity: v0 };
    var w0 = Math.sqrt(k / m);
    var zeta = c / (2 * Math.sqrt(k * m));
    if (Math.abs(zeta - 1) < 1e-6) {
      var b0 = v0 - w0;
      var e0 = Math.exp(-w0 * t);
      var y0 = (-1 + b0 * t) * e0;
      return { value: 1 + y0, velocity: (b0 - w0 * (-1 + b0 * t)) * e0 };
    }
    if (zeta < 1) {
      var wd = w0 * Math.sqrt(1 - zeta * zeta);
      var a = -1;
      var b = (v0 + zeta * w0 * a) / wd;
      var e = Math.exp(-zeta * w0 * t);
      var cos = Math.cos(wd * t);
      var sin = Math.sin(wd * t);
      var y = e * (a * cos + b * sin);
      return { value: 1 + y, velocity: e * ((b * wd - zeta * w0 * a) * cos - (a * wd + zeta * w0 * b) * sin) };
    }
    var root = w0 * Math.sqrt(zeta * zeta - 1);
    var r1 = -zeta * w0 + root;
    var r2 = -zeta * w0 - root;
    var c2 = (v0 + r1) / (r2 - r1);
    var c1 = -1 - c2;
    var e1 = Math.exp(r1 * t);
    var e2 = Math.exp(r2 * t);
    return { value: 1 + c1 * e1 + c2 * e2, velocity: c1 * r1 * e1 + c2 * r2 * e2 };
  }
  var cache = {};
  function settle(spring, tolerance) {
    var tol = tolerance === undefined ? 1e-3 : tolerance;
    var p = params(spring);
    var key = p.stiffness + "/" + p.damping + "/" + p.mass + "/" + p.velocity + "/" + tol;
    if (cache[key] !== undefined) return cache[key];
    var w0 = Math.sqrt(p.stiffness / p.mass);
    var zeta = p.damping / (2 * Math.sqrt(p.stiffness * p.mass));
    var rate = zeta < 1 ? zeta * w0 : zeta === 1 ? w0 : w0 * (zeta - Math.sqrt(zeta * zeta - 1));
    var horizon = Math.min(120, rate > 0 ? (Math.log(1 / tol) + Math.log(1 + Math.abs(p.velocity) / w0 + 1) + 8) / rate : 120);
    var dt = Math.min(1 / 1000, horizon / 20000);
    var last = 0;
    for (var t = 0; t <= horizon; t += dt) {
      var s = state(t, p);
      if (Math.abs(s.value - 1) > tol || Math.abs(s.velocity) > tol * w0) last = t;
    }
    var out = Math.min(horizon, last + dt);
    cache[key] = out;
    return out;
  }
  function biliupSpring(t, spring) {
    return state(t, spring).value;
  }
  biliupSpring.state = state;
  biliupSpring.settle = settle;
  biliupSpring.ease = function (spring) {
    var T = settle(spring);
    var end = state(T, spring).value;
    return function (p) {
      return p <= 0 ? 0 : p >= 1 ? 1 : state(p * T, spring).value + (1 - end) * p;
    };
  };
  biliupSpring.presets = PRESETS;
  window.biliupSpring = biliupSpring;
})();
