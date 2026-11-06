// Adapted from Uncut agentFootage.ts, MIT; see licenses/uncut/LICENSE.
export const SHEET_MAX_EDGE=1568,MIN_CELL_WIDTH=80,CELL_WIDTH=320,CAPTION_HEIGHT=22,GAP=6
export function sheetFrames(start: number, end: number, every: number | undefined, max: number): ReadonlyArray<number> | { readonly refused: string } {
  const length = end - start;
  if (length <= 0) return { refused: `The range is empty (${start} to ${end}).` };
  if (every !== undefined) {
    const count = Math.ceil(length / every);
    if (count > max) {
      const wider = Math.ceil(length / max);
      return { refused: `Every ${every} frames over ${length} frames is ${count} cells; a sheet holds at most ${max}. Use every ≥ ${wider}, or a shorter range.` };
    }
    return Array.from({ length: count }, (_, i) => start + i * every);
  }
  const count = Math.min(max, length);
  return Array.from({ length: count }, (_, i) => start + Math.floor((i * length) / count));
}

export interface SheetLayout {
  readonly columns: number;
  readonly rows: number;
  readonly cellWidth: number;
  readonly cellHeight: number;
  readonly width: number;
  readonly height: number;
}

/** The widest cell `columns` columns of `count` cells allow inside SHEET_MAX_EDGE both ways. */
function fit(count: number, aspect: number, columns: number): SheetLayout {
  const rows = Math.ceil(count / columns);
  const byWidth = Math.floor((SHEET_MAX_EDGE - GAP * (columns + 1)) / columns);
  const cellRoom = Math.floor((SHEET_MAX_EDGE - GAP * (rows + 1)) / rows) - CAPTION_HEIGHT;
  const byHeight = Math.floor(cellRoom * aspect);
  const cellWidth = Math.max(0, Math.min(CELL_WIDTH, byWidth, byHeight));
  const cellHeight = Math.max(1, Math.round(cellWidth / aspect));
  return {
    columns,
    rows,
    cellWidth,
    cellHeight,
    width: columns * cellWidth + GAP * (columns + 1),
    height: rows * (cellHeight + CAPTION_HEIGHT) + GAP * (rows + 1),
  };
}

/**
 * The grid for `count` cells of a picture `aspect` (width / height) wide, at most SHEET_MAX_EDGE
 * on its longest side: the column count that gives the biggest cells (ties: the one nearest a
 * landscape-friendly default), or `columns` when asked and its cells stay at least MIN_CELL_WIDTH
 * wide (else the best fit, which the caller reports).
 */
export function sheetLayout(count: number, aspect: number, columns?: number): SheetLayout {
  const a = aspect > 0 && Number.isFinite(aspect) ? aspect : 16 / 9;
  const n = Math.max(1, count);
  const most = Math.min(n, 10);
  if (columns !== undefined) {
    const asked = fit(n, a, Math.max(1, Math.min(most, Math.round(columns))));
    if (asked.cellWidth >= MIN_CELL_WIDTH) return asked;
  }
  const preferred = n <= 4 ? n : n <= 9 ? 3 : n <= 16 ? 4 : n <= 30 ? 5 : 6;
  let best: SheetLayout | undefined;
  for (let c = 1; c <= most; c++) {
    const l = fit(n, a, c);
    if (!best || l.cellWidth > best.cellWidth || (l.cellWidth === best.cellWidth && Math.abs(c - preferred) < Math.abs(best.columns - preferred))) best = l;
  }
  return best!;
}

/** 83.5 s → "1:23.50" (minutes, seconds and hundredths), "1:02:03.00" past an hour. */
export function clock(seconds: number): string {
  const cs = Math.max(0, Math.round(seconds * 100));
  const h = Math.floor(cs / 360000);
  const m = Math.floor((cs % 360000) / 6000);
  const s = ((cs % 6000) / 100).toFixed(2).padStart(5, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

const escapeHtml = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

/** The sheet as HTML for the rasteriser: the frames (JPEG data URLs) in a grid, a caption under each. */
export function sheetHtml(cells: ReadonlyArray<{ readonly jpeg: string; readonly caption: string }>, layout: SheetLayout): string {
  const { columns, cellWidth: w, cellHeight: h } = layout;
  const size = w < 150 ? 11 : 13;
  const body = cells
    .map((cell, i) => {
      const x = GAP + (i % columns) * (w + GAP);
      const y = GAP + Math.floor(i / columns) * (h + CAPTION_HEIGHT + GAP);
      return (
        `<div style="position:absolute;left:${x}px;top:${y}px;width:${w}px">` +
        `<img src="data:image/jpeg;base64,${cell.jpeg}" style="display:block;width:${w}px;height:${h}px;object-fit:contain;background:#000">` +
        `<div style="height:${CAPTION_HEIGHT}px;line-height:${CAPTION_HEIGHT}px;white-space:pre;overflow:hidden;text-overflow:ellipsis">${escapeHtml(cell.caption)}</div></div>`
      );
    })
    .join("");
  return `<div style="position:absolute;inset:0;background:#16161a;color:#e8e8ec;font:600 ${size}px 'DM Sans',Arial,sans-serif;font-variant-numeric:tabular-nums">${body}</div>`;
}

