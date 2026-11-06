/* biliupConfetti (Uncut, MIT): a confetti burst drawn as a pure function of time (D193), for
   graphics. Load it with the marker biliup:lib/confetti.js (a script tag with that src). Unlike
   canvas-confetti it has no timers and no state: every particle's place at t comes from a seeded
   random draw and a closed form (launch, air drag, gravity, flutter), so any frame can be drawn
   in any order.
     biliupConfetti.draw(ctx, t, {
       seed: 1, count: 160,             // the same seed draws the same burst
       at: 0,                           // when the burst starts (seconds; nothing before)
       origin: { x: 0.5, y: 0.7 },      // where it starts, as fractions of the canvas
       angle: 90, spread: 70,           // launch direction (degrees, 90 = up) and its spread
       velocity: 1.8,                   // launch speed in canvas heights per second
       gravity: 0.9, drag: 1.2,         // canvas heights per second squared; air drag per second
       size: 1,                         // particle size (1 = about 1 % of the canvas height)
       life: 3.5,                       // seconds until a particle has faded out
       colors: ["#ff5e7e", ...],
       shapes: ["rect", "circle"],
     })
   It draws on ctx as it is (no clear), and returns how many particles it drew.
*/
(function () {
  "use strict";
  var COLORS = ["#ff5e7e", "#ffd25e", "#5ecbff", "#7cff8a", "#b77cff", "#ff9f43"];
  function rng(seed) {
    var h = 0x811c9dc5;
    var s = String(seed);
    for (var i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193) >>> 0;
    var a = h >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) >>> 0;
      var r = Math.imul(a ^ (a >>> 15), a | 1);
      r ^= r + Math.imul(r ^ (r >>> 7), r | 61);
      return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
    };
  }
  function num(v, d) {
    return typeof v === "number" && Number.isFinite(v) ? v : d;
  }
  /* Position along one axis under linear drag k and constant acceleration g, from rest at 0 with speed v0. */
  function travel(v0, g, k, t) {
    if (k <= 1e-9) return v0 * t + 0.5 * g * t * t;
    var e = 1 - Math.exp(-k * t);
    return (g * t) / k + ((v0 - g / k) * e) / k;
  }
  function draw(ctx, t, options) {
    var o = options || {};
    var W = ctx.canvas ? ctx.canvas.width : 1920;
    var H = ctx.canvas ? ctx.canvas.height : 1080;
    var local = t - num(o.at, 0);
    if (!(local >= 0)) return 0;
    var count = Math.max(0, Math.min(2000, Math.floor(num(o.count, 160))));
    var origin = o.origin || {};
    var ox = num(origin.x, 0.5) * W;
    var oy = num(origin.y, 0.7) * H;
    var angle = (num(o.angle, 90) * Math.PI) / 180;
    var spread = (num(o.spread, 70) * Math.PI) / 180;
    var speed = num(o.velocity, 1.8) * H;
    var gravity = num(o.gravity, 0.9) * H;
    var drag = Math.max(0, num(o.drag, 1.2));
    var size = num(o.size, 1) * H * 0.01;
    var life = Math.max(0.01, num(o.life, 3.5));
    var colors = Array.isArray(o.colors) && o.colors.length ? o.colors : COLORS;
    var shapes = Array.isArray(o.shapes) && o.shapes.length ? o.shapes : ["rect", "rect", "circle"];
    var rnd = rng(o.seed === undefined ? 1 : o.seed);
    var drawn = 0;
    for (var i = 0; i < count; i++) {
      /* Every particle takes the same number of draws, so particle i is the same whatever count is. */
      var a = angle + (rnd() - 0.5) * spread;
      var v = speed * (0.45 + 0.55 * rnd());
      var color = colors[Math.floor(rnd() * colors.length)];
      var shape = shapes[Math.floor(rnd() * shapes.length)];
      var s = size * (0.6 + 0.8 * rnd());
      var spin = (rnd() - 0.5) * 12;
      var flipRate = 4 + rnd() * 8;
      var phase = rnd() * Math.PI * 2;
      var sway = (0.004 + rnd() * 0.012) * H;
      var swayRate = 2 + rnd() * 3;
      var lifeK = life * (0.75 + 0.25 * rnd());
      if (local >= lifeK) continue;
      var x = ox + travel(Math.cos(a) * v, 0, drag, local) + sway * Math.sin(swayRate * local + phase) * (1 - Math.exp(-2 * local));
      var y = oy + travel(-Math.sin(a) * v, gravity, drag, local);
      var fadeFrom = lifeK * 0.7;
      var alpha = local <= fadeFrom ? 1 : 1 - (local - fadeFrom) / (lifeK - fadeFrom);
      if (x < -s * 2 || x > W + s * 2 || y > H + s * 2 || alpha <= 0) continue;
      ctx.save();
      ctx.globalAlpha = alpha;
      ctx.fillStyle = color;
      ctx.translate(x, y);
      ctx.rotate(phase + spin * local);
      ctx.scale(1, Math.cos(flipRate * local + phase));
      if (shape === "circle") {
        ctx.beginPath();
        ctx.arc(0, 0, s * 0.5, 0, Math.PI * 2);
        ctx.fill();
      } else ctx.fillRect(-s * 0.5, -s * 0.3, s, s * 0.6);
      ctx.restore();
      drawn++;
    }
    return drawn;
  }
  window.biliupConfetti = { draw: draw, colors: COLORS.slice() };
})();
