import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The drag preview of a library tile (X2, D132): instead of the browser's screenshot of the whole
// tile, a small chip with the tile's own rendering (its thumbnail: the real text look or component)
// and its name. Built off screen for the dragstart's snapshot, then removed.
export function setDragChip(e: React.DragEvent<HTMLElement>, label: string) {
  const thumb = e.currentTarget.querySelector<HTMLElement>(".thumb");
  const chip = document.createElement("div");
  chip.className = "drag-chip";
  chip.dataset.testid = "drag-chip";
  if (thumb) {
    const copy = thumb.cloneNode(true) as HTMLElement;
    copy.classList.add("drag-chip-thumb");
    copy.style.width = `${thumb.offsetWidth}px`;
    copy.style.height = `${thumb.offsetHeight}px`;
    const box = document.createElement("div");
    box.className = "drag-chip-box";
    const k = Math.min(1, 112 / Math.max(1, thumb.offsetWidth));
    box.style.width = `${Math.round(thumb.offsetWidth * k)}px`;
    box.style.height = `${Math.round(thumb.offsetHeight * k)}px`;
    copy.style.transform = `scale(${k})`;
    box.append(copy);
    chip.append(box);
  }
  const name = document.createElement("span");
  name.textContent = label;
  chip.append(name);
  editorRoot().append(chip);
  e.dataTransfer.setDragImage(chip, 16, 16);
  // Chromium snapshots it at the end of dragstart.
  setTimeout(() => chip.remove(), 0);
}
