// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.


export const n = (x: number) => {
  const s = (Math.round(x * 1e6) / 1e6).toString();
  return s.includes("e") ? x.toFixed(9) : s;
};
