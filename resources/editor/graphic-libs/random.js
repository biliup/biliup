/* biliupRandom (Uncut, MIT): seeded randomness for graphics (D193). Load it with the marker
   biliup:lib/random.js (a script tag with that src). Never use Math.random in a graphic: the
   export draws frames out of order and in another window, so only values from a seed repeat.
     const rnd = biliupRandom(42);     // or a string seed; the same seed gives the same sequence
     rnd()                            // a float in [0, 1)
     rnd.range(a, b), rnd.int(a, b)   // a float in [a, b), an integer in [a, b]
     rnd.pick(list), rnd.normal()     // an element; a standard normal value
     biliupRandom.at(seed, i)          // the i-th value of seed's sequence without a generator (pure)
     biliupRandom.noise(seed, x)       // smooth value noise in [0, 1] along x (pure)
*/
(function () {
  "use strict";
  function hash(seed) {
    if (typeof seed === "number" && Number.isFinite(seed)) return (Math.floor(seed) ^ 0x9e3779b9) >>> 0;
    var s = String(seed === undefined ? "" : seed);
    var h = 0x811c9dc5;
    for (var i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 0x01000193) >>> 0;
    }
    return h >>> 0;
  }
  /* mulberry32: one step from a 32-bit state. */
  function step(a) {
    var t = (a + 0x6d2b79f5) >>> 0;
    var r = Math.imul(t ^ (t >>> 15), t | 1);
    r ^= r + Math.imul(r ^ (r >>> 7), r | 61);
    return { state: t, value: ((r ^ (r >>> 14)) >>> 0) / 4294967296 };
  }
  function biliupRandom(seed) {
    var state = hash(seed);
    var next = function () {
      var s = step(state);
      state = s.state;
      return s.value;
    };
    next.range = function (a, b) {
      return a + (b - a) * next();
    };
    next.int = function (a, b) {
      return Math.floor(a + (b - a + 1) * next());
    };
    next.pick = function (list) {
      return list[Math.floor(next() * list.length)];
    };
    next.normal = function () {
      var u = 1 - next();
      var v = next();
      return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
    };
    return next;
  }
  biliupRandom.hash = hash;
  biliupRandom.at = function (seed, i) {
    return step((hash(seed) + Math.imul(Math.floor(i) >>> 0, 0x9e3779b9)) >>> 0).value;
  };
  biliupRandom.noise = function (seed, x) {
    var i = Math.floor(x);
    var f = x - i;
    var u = f * f * (3 - 2 * f);
    return biliupRandom.at(seed, i) * (1 - u) + biliupRandom.at(seed, i + 1) * u;
  };
  window.biliupRandom = biliupRandom;
})();
