/* biliupGlobe (Uncut, MIT): a dotted, spinning globe drawn on a 2D canvas as a pure function of
   time (D193), for graphics. Load it with the marker biliup:lib/globe.js (a script tag with that
   src). Unlike cobe it has no WebGL loop and no state: the rotation at t is rotation + speed × t, so any frame can be
   drawn in any order. The land is a coarse outline of the continents (a look, not a map).
     biliupGlobe.draw(ctx, t, {
       x, y, radius,                    // centre and radius in canvas px (default: centred, 36 % of the shorter side)
       dots: 4000,                      // dots over the whole sphere (land dots are drawn from these)
       rotation: 0, speed: 20,          // longitude facing the viewer at t = 0 (degrees), degrees per second
       tilt: 18,                        // tilt toward the viewer (degrees)
       dotSize: 1,                      // dot size (1 = about 0.8 % of the radius)
       ocean: 0,                        // opacity of dots over the ocean (0 = land only)
       markers: [{ lat: 52.2, lon: 21, color: "#ff5e7e", size: 1 }],
       colors: { dot: "#ffffff", marker: "#ff5e7e", sphere: "rgba(255,255,255,0.04)", rim: "rgba(255,255,255,0.25)" },
     })
   biliupGlobe.project(ctx, t, lat, lon, options) gives where a place is drawn: { x, y, visible }.
*/
(function () {
  "use strict";
  /* Continents as rough [lon, lat] outlines. */
  var LAND = [
    [[-168, 66], [-140, 70], [-110, 73], [-85, 70], [-75, 62], [-60, 55], [-55, 48], [-66, 44], [-75, 38], [-81, 31], [-80, 25], [-90, 29], [-97, 26], [-97, 20], [-87, 21], [-83, 10], [-78, 8], [-90, 14], [-105, 20], [-112, 30], [-118, 34], [-124, 40], [-124, 48], [-135, 58], [-150, 60], [-165, 60]],
    [[-55, 60], [-45, 60], [-20, 70], [-20, 80], [-40, 83], [-65, 80], [-70, 76], [-55, 68]],
    [[-80, 9], [-72, 12], [-60, 10], [-50, 0], [-35, -6], [-39, -15], [-48, -26], [-58, -38], [-65, -42], [-68, -55], [-74, -52], [-72, -35], [-71, -18], [-81, -5]],
    [[-10, 36], [-9, 43], [-2, 44], [-5, 48], [3, 51], [8, 54], [10, 58], [5, 62], [15, 69], [28, 71], [40, 67], [45, 55], [40, 45], [30, 45], [26, 41], [22, 37], [15, 38], [12, 44], [3, 42]],
    [[-6, 50], [2, 51], [0, 54], [-3, 58], [-6, 57]],
    [[-17, 21], [-16, 12], [-8, 4], [6, 4], [9, -1], [13, -12], [12, -18], [18, -34], [26, -34], [33, -26], [40, -15], [40, -3], [51, 12], [43, 12], [32, 31], [20, 32], [10, 37], [-6, 36]],
    [[44, -25], [50, -15], [49, -12], [43, -17]],
    [[26, 41], [36, 36], [35, 30], [43, 12], [52, 16], [57, 25], [67, 25], [73, 20], [77, 8], [80, 15], [88, 22], [93, 17], [98, 8], [104, 1], [106, 10], [109, 20], [118, 24], [122, 31], [118, 39], [127, 38], [130, 43], [141, 46], [142, 53], [156, 58], [163, 62], [178, 65], [180, 69], [140, 73], [110, 77], [70, 73], [60, 69], [45, 68], [40, 67], [45, 55], [40, 45]],
    [[130, 31], [136, 34], [141, 37], [142, 43], [140, 41], [135, 35]],
    [[95, 5], [106, -6], [115, -8], [120, -9], [125, -8], [118, 0], [117, 7], [110, 2], [104, -2]],
    [[114, -22], [122, -18], [130, -12], [137, -12], [142, -11], [146, -19], [153, -26], [150, -37], [141, -38], [135, -34], [116, -35]],
    [[166, -46], [174, -41], [178, -38], [174, -36], [172, -41], [168, -44]],
  ];
  function inside(poly, lon, lat) {
    var hit = false;
    for (var i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      var xi = poly[i][0], yi = poly[i][1], xj = poly[j][0], yj = poly[j][1];
      if (yi > lat !== yj > lat && lon < ((xj - xi) * (lat - yi)) / (yj - yi) + xi) hit = !hit;
    }
    return hit;
  }
  function isLand(lat, lon) {
    if (lat < -70) return true;
    for (var i = 0; i < LAND.length; i++) if (inside(LAND[i], lon, lat)) return true;
    return false;
  }
  var cache = {};
  /* A Fibonacci sphere: n evenly spread points, each [lat, lon, land]. */
  function points(n) {
    if (cache[n]) return cache[n];
    var out = [];
    var golden = Math.PI * (3 - Math.sqrt(5));
    for (var i = 0; i < n; i++) {
      var y = 1 - (2 * (i + 0.5)) / n;
      var lat = (Math.asin(y) * 180) / Math.PI;
      var lon = ((((i * golden * 180) / Math.PI) % 360) + 540) % 360 - 180;
      out.push([lat, lon, isLand(lat, lon)]);
    }
    cache[n] = out;
    return out;
  }
  function num(v, d) {
    return typeof v === "number" && Number.isFinite(v) ? v : d;
  }
  function frame(ctx, t, o) {
    var W = ctx && ctx.canvas ? ctx.canvas.width : 1920;
    var H = ctx && ctx.canvas ? ctx.canvas.height : 1080;
    return {
      cx: num(o.x, W / 2),
      cy: num(o.y, H / 2),
      r: num(o.radius, Math.min(W, H) * 0.36),
      spin: ((num(o.rotation, 0) + num(o.speed, 20) * t) * Math.PI) / 180,
      tilt: (num(o.tilt, 18) * Math.PI) / 180,
    };
  }
  function place(f, lat, lon) {
    var phi = (lat * Math.PI) / 180;
    var lam = (lon * Math.PI) / 180 - f.spin;
    var x = Math.cos(phi) * Math.sin(lam);
    var y = Math.sin(phi);
    var z = Math.cos(phi) * Math.cos(lam);
    var y2 = y * Math.cos(f.tilt) - z * Math.sin(f.tilt);
    var z2 = y * Math.sin(f.tilt) + z * Math.cos(f.tilt);
    return { x: f.cx + f.r * x, y: f.cy - f.r * y2, z: z2 };
  }
  function draw(ctx, t, options) {
    var o = options || {};
    var c = o.colors || {};
    var f = frame(ctx, t, o);
    var n = Math.max(100, Math.min(20000, Math.floor(num(o.dots, 4000))));
    var dot = num(o.dotSize, 1) * f.r * 0.008;
    var ocean = Math.max(0, Math.min(1, num(o.ocean, 0)));
    ctx.save();
    if (c.sphere !== null) {
      ctx.fillStyle = c.sphere || "rgba(255,255,255,0.04)";
      ctx.beginPath();
      ctx.arc(f.cx, f.cy, f.r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = c.dot || "#ffffff";
    var list = points(n);
    for (var i = 0; i < list.length; i++) {
      var p = list[i];
      var weight = p[2] ? 1 : ocean;
      if (weight <= 0) continue;
      var s = place(f, p[0], p[1]);
      if (s.z <= 0) continue;
      ctx.globalAlpha = weight * (0.25 + 0.75 * s.z);
      ctx.beginPath();
      ctx.arc(s.x, s.y, dot * (0.6 + 0.4 * s.z), 0, Math.PI * 2);
      ctx.fill();
    }
    if (c.rim !== null) {
      ctx.globalAlpha = 1;
      ctx.strokeStyle = c.rim || "rgba(255,255,255,0.25)";
      ctx.lineWidth = Math.max(1, f.r * 0.006);
      ctx.beginPath();
      ctx.arc(f.cx, f.cy, f.r, 0, Math.PI * 2);
      ctx.stroke();
    }
    var markers = Array.isArray(o.markers) ? o.markers : [];
    for (var m = 0; m < markers.length; m++) {
      var mk = markers[m] || {};
      var at = place(f, num(mk.lat, 0), num(mk.lon, 0));
      if (at.z <= 0) continue;
      var size = num(mk.size, 1) * f.r * 0.03;
      var pulse = (t * 0.8 + m * 0.37) % 1;
      ctx.fillStyle = mk.color || c.marker || "#ff5e7e";
      ctx.strokeStyle = ctx.fillStyle;
      ctx.globalAlpha = (1 - pulse) * at.z;
      ctx.lineWidth = Math.max(1, size * 0.2);
      ctx.beginPath();
      ctx.arc(at.x, at.y, size * (1 + 2 * pulse), 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = 0.5 + 0.5 * at.z;
      ctx.beginPath();
      ctx.arc(at.x, at.y, size, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }
  window.biliupGlobe = {
    draw: draw,
    isLand: isLand,
    project: function (ctx, t, lat, lon, options) {
      var s = place(frame(ctx, t, options || {}), lat, lon);
      return { x: s.x, y: s.y, visible: s.z > 0 };
    },
  };
})();
