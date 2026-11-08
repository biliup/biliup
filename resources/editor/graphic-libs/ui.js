/* biliupUI (Uncut, MIT): Biliup's own editor UI for a film about Biliup itself: the real app's markup
   and CSS, generated from its React components (scripts/ui-parts.ts), as live DOM (crisp at any
   size). Load it with the marker biliup:lib/ui.js (a script tag with that src). Dark, as the app.
   READABLE: every word the viewer must read stands at least 3 % of the frame's height (its letters; a font
   of 4.2 %). Two calls get there: a piece at its default type (4.8 % of the height; a smaller type is
   smaller words, and type sizes the piece's FIRST line: a thread's 12 px step rows come out 12/13 of its
   message, so give steps their own "turn.steps" key), or biliupUI.frameText(el, world, { width, height })
   as the camera key on the stage. render_frames lists every word under 3 % in each frame it draws.
   Build the film from PIECES: one element of the UI on its own (the composer, a step row, a lane of
   clips, the filter slider, the preview, Export, Saved), set LARGE on a calm canvas (60 to 90 % of
   the width, its text 4.5 % of the frame's height or more), each one morphing into the next. A whole
   panel only briefly, and only when its words read on a phone; never a camera panning over a
   desktop-sized window: that reads as a screen recording. Or the whole program as the STAGE: the
   editor mounted live (its panels follow props every frame), blurred behind a big hero piece, then
   in focus while a camera (biliupMotion.camera) pushes into the region where the work happens.
     biliupUI.sequence(host, t, keys, { dur: 0.9 })    // ONE shell carried through keys (call it every frame):
       keys: [{ at, piece: "composer", props }, { at: 2, piece: "message", props, carry: [["input", "text"]] }, ...]
       piece: "part.slot" (or "part"); props: the part's props below; index: the nth match. Big by default:
       centred in the host, 86 % of its width, its words type px tall (default 4.8 % of the host's height:
       the layout narrows so they wrap and set large). x, y: its centre; width: host px; type: words' height
       (0: the app's); size: its own layout size in app px instead; height: a taller or wider shell; pad,
       radius, background, color (restyles); theme: "light" (white surfaces, dark words) or the app's dark;
       press: the shell's scale (biliupMotion.click's .button); fit: "contain" | "cover" (default cover for
       preview.frame); shadow: false; dur: its morph; from: "down" (default), "up", "left", "right", "none".
       From each key's at the shell's centre, size, radius and colour travel to it in dur s (a calm glide:
       eases off, slows down longer than it sped up, no overshoot) while the new content pushes in from its
       side out of a blur and the old leaves by the other, both at one uniform scale (never two legible
       pictures at once); carry: selectors (or [old's, new's]) that travel instead, the prompt shrinking
       into its message. A leaving preview frame stays put, cropped by the shell. The same piece twice (one more step) grows without a swap. Pure in t.
       A swapped-in piece reads from about at + 0.6 dur and is at rest at at + dur: start the key at least its dur before
       the moment it shows (a frame mid-swap is blurred and dim by design; render_frames names a word softened by a swap).
       typing: { at, dur, from }: props.text typed in (from characters at at, all of it by at + dur), the shell
       growing a line at a time over 0.22 s at each wrap, its toolbar riding the bottom edge: pass the WHOLE text
       (also in a fly's from key), never a slice you cut per frame, which makes the card jump a line per wrap.
       Returns { x, y, w, h, r, p, key, el, type } (el: the shown piece's element, for find and rectIn).
     biliupUI.piece(host, "export.high", { props, width, x, y, ... })  // one piece alone (sequence's key at rest)
     biliupUI.mount(host, "editor", { width: 1440, height: 1000, blur, dim, ...props })  // the program as a live stage:
       call it every frame; each panel is rebuilt only when its props change (clips moving, steps growing,
       the playhead, an export's progress). blur (px on screen at its scale) and dim (0..1): the program
       behind a hero. width, height: the window it lays out in (the real responsive layout). time: t, so a
       running step's spinner turns (pass it every frame).
     biliupUI.fly(fx, t, { at, dur: 0.9, from, to, arc, leave, fit, id })  // one element travelling into its place,
       call it every frame for every flight: from a piece key (the big hero composer, shown at rest before at,
       its Send pressed), an element (a media tile, the prompt's text) or a rect, to an element in the stage
       (the editor's composer, a timeline clip, the preview's frame); rect, radius and colour travel on an arc
       (host px, negative: up) while the start's look hands over to the landing element's own; the landing
       element is hidden until it lands, then the real one shows. leave: the from element goes at at (the
       prompt leaving the composer); fit: "cover" (default) | "contain" (text into a bubble); id: one per
       flight in the air. fx: a layer above the stage's camera (rects are measured on screen, so a moving
       camera is fine), best once the camera has settled; several flights go one after another, each
       leaving as the last lands. Returns { x, y, w, h, p, el } (el: the hero piece's element while it is up).
     biliupUI.rectIn(el, host)                         // { x, y, w, h }: an element's rect in host px (aim a click at Send)
     biliupUI.frameText(el, host, { width, height, text: 0.048, safe: 72 })  // a camera key { x, y, zoom } on el's
       words (host: the world div the camera moves) zoomed until their font is text of the frame's height (a piece's
       4.8 %), kept whole inside the safe inset; returns text (the share they get) and fits (false: too wide to set
       that large: framing the agent panel's whole width leaves its 12 px steps about 2 % tall; set them as a piece)
     biliupUI.mount(host, name, props)                 // a whole part (returns its root): rarely, briefly
     biliupUI.parts()                                  // editor, topbar, agent, message, turn, review, timeline, preview,
                                                      // composer, storyboard, export, exportProgress, inspector, effects, menu
     biliupUI.slots(name), biliupUI.size(name)          // its slots (what a piece can be); its box in the app, CSS px
     biliupUI.find(root, slot), biliupUI.findAll(root, slot)  // a slot's element (or any CSS selector's) in a part or piece
   Pieces (part.slot): composer, composer.box, composer.send; message; turn.steps, turn.step, turn.edit,
     turn.review; storyboard.beat, storyboard.approve; review.score; timeline.main, timeline.lane, timeline.clip,
     timeline.playhead; preview.frame, preview.play; inspector.amount (a filter's slider), inspector.effect,
     inspector.filter; effects.warm (any look: natural, cool, vintage, film, bw, vivid), effects.tile, effects.filters;
     export.draft, export.standard, export.high, export.export; exportProgress.job (status and bar), exportProgress.progress, exportProgress.bar;
     topbar.export, topbar.saved, topbar.title; menu (the whole menu), menu.addToChat, menu.item.
   Props (every part): text { slot: "..." }; press { slot: scale }; highlight, hide (keeps its room), remove (slot
   or [slots]); background (mount: "none" or a colour behind a part that isn't opaque). Each part's own:
       composer: text (typed so far; empty disables Send), caret, placeholder, context ("Playhead 00:00:12:00"),
         chips: ["DJI 0063"] (each "@DJI 0063" in text drawn as the app's chip: right-click a clip, Add to chat, and
         the composer holds "@<its name> " with the caret after it; a typing key's from past the chip types after it)
       message: text (the user's prompt bubble), chips (as the composer's: the sent prompt keeps its chip)
       turn: steps: ["Read the brand kit", ...], done (rows done, the rest running), edit (the edit row's label),
         reply (the agent's answer), review (true, or the review's props: its card in the turn), status
       storyboard: summary, beats: [{ start, end, title, lines: ["..."] }], count (the first n)
       review: scores { hook, readability, motion, variety, sound, brand } (0 to 10), progress 0..1, status, title
       timeline: clips: [{ track: "Main" | "Text" | "Graphics" | "Audio" | "Sound effects" | index, start, duration
         (s), name, image, media (its media id), in (s into the media), selected, color }]: the whole timeline
         (it starts empty); playhead (s)
       preview: frame (your canvas or img: it fills the frame; false: empty), time (s: the timecode), duration
         (s: the length after it; an editor's is its clips' end), canvas ("1080×1350, 60 fps"). Without a frame,
         the editor's preview shows the clip under the playhead (timeline.playhead, else preview.time): its
         media's frame at in + (playhead - start) through biliup.frame, else its image; a gap is black
       inspector: title (the clip's name), filter (the look's name on its slider), amount (0 to 100),
         filter: false, effects: false (those sections out)
       effects: active (the look picked: "warm"; none: ""), target ("On DJI 0063")
       export: quality ("draft" | "standard" | "high"), file ("Launch film.mp4"), motionBlur, duration (s: the range)
       exportProgress: progress 0..1 ("Rendering... 42%"), eta (s left)
       topbar: exportLabel ("Exporting 42%"), title (the project's name)
       menu: a timeline clip's right-click menu as the app builds it for a video clip on Main (Add to chat ⌘L,
         Split S, Copy, Paste, Duplicate, Delete, Ripple delete, Speed, Rename, Replace media, Show in Media,
         Reveal in Finder, Lock), "Add to chat" highlighted under the pointer; active: the row under the pointer
         (an entry id: "add-to-chat", "split", "copy", ...; "": none). Its top-left corner goes at the pointer.
       agent (the side panel), editor (the window, 1920 x 900): the parts inside by name ({ composer: { text } });
         editor: media: [{ name, meta, image, duration }] (the library's tiles), side: "inspector" (that tab, its
         props: inspector), library: "effects" (that panel, props: effects), export or exportProgress (that dialog
         over the window); a running step (turn: done) shows the app's spinner
   An image is a data: or blob: URL: await biliup.url(id, seconds) once before the first frame (an biliup:media/
   marker only resolves in the graphic's own HTML). No run fact of the fixture shows unless you pass it: no prompt
   in a message, no clips, media tiles, steps, reply, edit count or review card in a turn (review: true
   is a blank card), no export progress; the review says "Reviewing" with empty bars, no round, model,
   time or time left; the project's title, length and canvas, a tile's length badge, the clip named in the
   Inspector and Effects, its timings and the export's file name start blank too (topbar title, preview
   duration and canvas, media duration, inspector title, effects target, export duration and file). Run facts are real or absent: pass a result,
   round, score or timing only from this run's real results (review_submit, beat_grid), never invented.
     biliupUI.cursor(host, { x, y, scale, size: 28 })  // the macOS pointer, its tip at (x, y) in host px
     biliupUI.timecode(seconds, fps), biliupUI.css()    // "00:00:12:30"; the parts' CSS (added once by mount)
   The fonts are the Mac's system UI fonts, as in the app; nothing loads from the network. */
/* biliup-ui-inputs: 2bce3dcbe3b495fd916f5c23334dfaae9f91500616c1c63e9a97fac880691e0c */
(function () {
var DATA = {"css":".biliup-ui .composer-ctx .ellipsis{min-width: 0px}\n.biliup-ui .tile-chat{width: 22px; height: 22px; color: var(--fg); opacity: 0; background: rgba(0, 0, 0, 0.6); border-radius: 50%; place-items: center; padding: 0px; display: grid; position: absolute; top: 4px; right: 30px}\n.biliup-ui .tile-chat.second{right: 56px}\n.biliup-ui .tile-chat svg.lucide{width: 13px; height: 13px}\n.biliup-ui .app > .banners{z-index: 2147483500}\n.biliup-ui .app{--focus-col: clamp(360px, 34vw, 560px)}\n.biliup-ui .layout-toggles{border-radius: var(--radius-sm); align-items: center; gap: 1px; margin-right: 4px; padding: 1px; display: inline-flex}\n.biliup-ui .layout-toggles .icon-btn[aria-pressed=\"false\"]{color: var(--muted); opacity: 0.6}\n.biliup-ui .wordmark{align-items: center; gap: 8px; min-width: 0px; display: flex}\n.biliup-ui .wordmark-name{flex: 0 0 auto; align-items: center; gap: 7px; display: inline-flex}\n.biliup-ui .wordmark-mark{width: 11px; height: 13px; color: var(--fg); flex: 0 0 auto}\n.biliup-ui .wordmark-text{letter-spacing: -0.02em; background-image: linear-gradient(118deg, var(--fg) 0 40%, #fff 45%, #a9bcff 50%, #fff 55%, var(--fg) 60% 100%); color: rgba(0, 0, 0, 0); background-position: 100% 0px; background-size: 300% 100%; background-clip: text; font-weight: 650}\n.biliup-ui .wordmark-slash{background: var(--border-strong); flex: 0 0 auto; width: 1px; height: 14px; transform: rotate(28deg)}\n.biliup-ui .wordmark .project-title{padding: 4px 2px}\n.biliup-ui{--bg: #0a0a0a; --panel: #111; --panel-2: #161616; --raised: #1c1c1c; --border: #ffffff12; --border-strong: #ffffff1f; --fg: #ededed; --muted: #8f8f8f; --dim: #5f5f5f; --select: #6d8cff; --select-soft: #6d8cff29; --ok: #4ade80; --warn: #fbbf24; --error: #f87171; --radius: 10px; --radius-sm: 6px; --font: -apple-system, BlinkMacSystemFont, \"SF Pro Text\", \"Inter\", system-ui, sans-serif; --mono: \"SF Mono\", ui-monospace, Menlo, monospace; --lightningcss-light:; --lightningcss-dark: initial; color-scheme: dark}\n.biliup-ui, .biliup-ui *{box-sizing: border-box}\n.biliup-ui{color: var(--fg); font: 13px/1.4 var(--font)}\n.biliup-ui button{font: inherit; color: inherit; cursor: pointer; background: 0px 0px; border: 0px}\n.biliup-ui button:disabled{cursor: default; opacity: 0.45}\n.biliup-ui input{font: inherit; color: inherit}\n.biliup-ui svg.lucide{stroke-width: 1.75px; flex: 0 0 auto; width: 16px; height: 16px}\n.biliup-ui .muted{color: var(--muted)}\n.biliup-ui .small{font-size: 11px}\n.biliup-ui .mono{font-family: var(--mono)}\n.biliup-ui .ellipsis{white-space: nowrap; text-overflow: ellipsis; min-width: 0px; overflow: hidden}\n.biliup-ui .icon-btn{border-radius: var(--radius-sm); width: 28px; height: 28px; color: var(--muted); justify-content: center; align-items: center; display: inline-flex}\n.biliup-ui .icon-btn.active{color: var(--fg); background: var(--raised)}\n.biliup-ui .btn{border-radius: var(--radius-sm); border: 1px solid var(--border-strong); white-space: nowrap; flex: 0 0 auto; align-items: center; gap: 6px; height: 28px; padding: 0px 12px; display: inline-flex}\n.biliup-ui .btn svg.lucide{width: 14px; height: 14px}\n.biliup-ui .btn.primary{background: var(--fg); color: rgb(0, 0, 0); border-color: rgba(0, 0, 0, 0); font-weight: 500}\n.biliup-ui .btn.ghost{color: var(--muted); border-color: rgba(0, 0, 0, 0)}\n.biliup-ui .modal{background: var(--panel); border: 1px solid var(--border-strong); border-radius: 14px}\n.biliup-ui .modal.wiz{border-radius: 16px; width: 640px; max-width: 100%; overflow: hidden}\n.biliup-ui .wiz-panel{background: rgba(255, 255, 255, 0.02); border-top: 1px solid rgba(255, 255, 255, 0.05); padding: 22px 24px 20px}\n.biliup-ui .wiz-panel h1{letter-spacing: -0.02em; margin: 0px; font-size: 24px; font-weight: 600}\n.biliup-ui .wiz-lead{color: var(--muted); margin: 10px 0px 18px; font-size: 14px; line-height: 1.6}\n.biliup-ui .wiz-list{flex-direction: column; gap: 8px; display: flex}\n.biliup-ui .wiz-cards{grid-template-columns: repeat(3, 1fr); gap: 8px; display: grid}\n.biliup-ui .wiz-card{border: 1px solid var(--border); background: var(--bg); text-align: left; border-radius: 10px; flex-direction: column; align-items: flex-start; gap: 2px; padding: 12px; font-weight: 500; display: flex}\n.biliup-ui .wiz-card span{color: var(--muted); font-size: 12px; font-weight: 400}\n.biliup-ui .wiz-card.active{border-color: var(--fg)}\n.biliup-ui .wiz-field{border: 1px solid var(--border); background: var(--bg); border-radius: 10px; align-items: center; gap: 12px; min-height: 46px; padding: 6px 8px 6px 14px; display: flex}\n.biliup-ui .wiz-field > span:first-child{width: 48px; color: var(--muted); flex: 0 0 auto; font-size: 12px}\n.biliup-ui .wiz-field b{flex: 1 1 0%; font-weight: 500}\n.biliup-ui .wiz-field input{background: 0px 0px; border: 0px; outline: 0px; flex: 1 1 0%; min-width: 0px; height: 32px; font-size: 14px}\n.biliup-ui .wiz-meta{color: var(--dim); margin: 12px 0px 0px; font-size: 12px}\n.biliup-ui .wiz-foot{justify-content: flex-end; align-items: center; gap: 8px; margin-top: 22px; display: flex}\n.biliup-ui .form-fields{margin-top: 12px}\n.biliup-ui .app{--lib-col: clamp(200px, 20vw, 272px); --side-col: clamp(260px, 24vw, 320px); --bottom-row: clamp(170px, 30vh, 280px); grid-template-columns: 56px var(--lib-col) minmax(0, 1fr) var(--side-col); grid-template-rows: 40px minmax(0, 1fr) minmax(0, var(--bottom-row)); grid-template-areas: \"top top top top\" \"rail lib stage side\" \"bottom bottom bottom bottom\"; height: 100vh; display: grid; position: relative; overflow: clip}\n.biliup-ui .resizer{z-index: 20; position: relative}\n.biliup-ui .resizer.v{cursor: col-resize; grid-row: 2; width: 7px}\n.biliup-ui .resizer.h{cursor: row-resize; grid-area: bottom; align-self: start; height: 7px; margin-top: -3px}\n.biliup-ui .resizer-lib{grid-column: 2; justify-self: end; margin-right: -4px}\n.biliup-ui .resizer-side{grid-column: 4; justify-self: start; margin-left: -4px}\n.biliup-ui .resizer::after{content: \"\"; background: var(--select); opacity: 0; pointer-events: none; border-radius: 2px; position: absolute}\n.biliup-ui .resizer.v::after{width: 2px; margin-left: -1px; top: 0px; bottom: 0px; left: 50%}\n.biliup-ui .resizer.h::after{height: 2px; margin-top: -1px; top: 50%; left: 0px; right: 0px}\n.biliup-ui input,.biliup-ui textarea,.biliup-ui .msg-user,.biliup-ui .turn .reply{user-select: text}\n.biliup-ui input[type=\"checkbox\"],.biliup-ui input[type=\"range\"]{user-select: none}\n.biliup-ui .topbar{border-bottom: 1px solid var(--border); app-region: drag; user-select: none; grid-area: top; align-items: center; gap: 6px; padding: 0px 10px 0px 80px; display: flex}\n.biliup-ui .topbar button{app-region: no-drag}\n.biliup-ui .brand{letter-spacing: -0.01em; font-weight: 600}\n.biliup-ui .project-title{color: var(--muted); white-space: nowrap; text-overflow: ellipsis; min-width: 0px; padding: 4px 8px; overflow: hidden}\n.biliup-ui .topbar-spacer{flex: 1 1 0%}\n.biliup-ui .saved{color: var(--dim); align-items: center; gap: 5px; margin-right: 6px; font-size: 12px; display: flex}\n.biliup-ui .saved svg.lucide{width: 14px; height: 14px}\n.biliup-ui .banners{z-index: 30; pointer-events: none; flex-direction: column; gap: 6px; width: min(680px, 100% - 32px); display: flex; position: absolute; top: 48px; left: 50%; transform: translate(-50%)}\n.biliup-ui .rail{border-right: 1px solid var(--border); scrollbar-width: none; flex-direction: column; grid-area: rail; align-items: center; gap: 2px; min-height: 0px; padding: 8px 0px; display: flex; overflow: hidden auto}\n.biliup-ui .rail button{border-radius: var(--radius-sm); width: 48px; color: var(--muted); flex-direction: column; flex: 0 0 auto; align-items: center; gap: 3px; padding: 7px 0px 5px; display: flex}\n.biliup-ui .rail button span{font-size: 9.5px}\n.biliup-ui .rail button.active{color: var(--fg); background: var(--raised)}\n.biliup-ui .library{border-right: 1px solid var(--border); background: var(--panel); flex-direction: column; grid-area: lib; display: flex; overflow: auto}\n.biliup-ui .lib-head{justify-content: space-between; align-items: center; padding: 12px 14px 8px; font-weight: 600; display: flex}\n.biliup-ui .stage{flex-direction: column; grid-area: stage; min-width: 0px; min-height: 0px; display: flex; position: relative; container: stage / size}\n.biliup-ui .stage-canvas{background: rgb(8, 8, 8); flex: 1 1 0%; justify-content: center; align-items: center; min-height: 0px; padding: 18px; display: flex}\n.biliup-ui .frame{isolation: isolate; outline: 1px solid var(--border); background: rgb(0, 0, 0); position: relative; overflow: hidden; container-type: size}\n.biliup-ui .pv-canvas{transform-origin: 0px 0px; position: absolute; top: 0px; left: 0px; container-type: size}\n.biliup-ui .pv-layer{position: absolute; inset: 0px}\n.biliup-ui .pv-box{width: 100%; height: 100%; position: absolute; top: 0px; left: 0px; overflow: hidden}\n.biliup-ui .pv-box > video{object-fit: contain; width: 100%; max-width: none; height: 100%; position: absolute; top: 0px; left: 0px}\n.biliup-ui .pv-layer > canvas.pv-gpu{pointer-events: none; width: 100%; height: 100%; display: block; position: absolute; top: 0px; left: 0px}\n.biliup-ui .pv-missing{padding: calc(16px / var(--pv-scale)); text-align: center; color: var(--muted); font-size: calc(12px / var(--pv-scale)); background: repeating-linear-gradient(135deg, #111 0 calc(10px / var(--pv-scale)), #0d0d0d calc(10px / var(--pv-scale)) calc(20px / var(--pv-scale))); place-items: center; display: grid; position: absolute; inset: 0px}\n.biliup-ui .pv-overlay{z-index: 100000; pointer-events: none}\n.biliup-ui .transport{border-top: 1px solid var(--border); flex: 0 0 auto; align-items: center; gap: 10px; height: 40px; min-height: 40px; padding: 0px 12px; display: flex; overflow: hidden}\n.biliup-ui .transport > *,.biliup-ui .transport-mid > *{flex: 0 0 auto}\n.biliup-ui .timecode{font: 12px var(--mono); white-space: nowrap}\n.biliup-ui .transport > [data-testid=\"canvas-info\"]{white-space: nowrap; text-overflow: ellipsis; flex: 0 1 auto; min-width: 0px; overflow: hidden}\n.biliup-ui .transport > .transport-mid{flex: 1 0 auto; justify-content: center; gap: 4px; display: flex}\n.biliup-ui .icon-btn.play{color: var(--fg)}\n.biliup-ui .side{border-left: 1px solid var(--border); background: var(--panel); flex-direction: column; grid-area: side; min-height: 0px; display: flex}\n.biliup-ui .tabs{border-bottom: 1px solid var(--border); gap: 2px; padding: 6px; display: flex}\n.biliup-ui .tabs button{border-radius: var(--radius-sm); height: 28px; color: var(--muted); flex: 1 1 0%}\n.biliup-ui .tabs button.active{background: var(--raised); color: var(--fg)}\n.biliup-ui .side-body{flex-direction: column; flex: 1 1 0%; min-height: 0px; display: flex; overflow: auto}\n.biliup-ui .insp-title{align-items: center; gap: 8px; padding: 12px 14px 4px; font-weight: 600; display: flex}\n.biliup-ui .insp-sub{color: var(--muted); padding: 0px 14px 8px; font-size: 11px}\n.biliup-ui .group{border-bottom: 1px solid var(--border); padding: 12px 14px}\n.biliup-ui .group-label{color: var(--muted); margin-bottom: 8px; font-size: 11px}\n.biliup-ui .field-row{grid-template-columns: 1fr 1fr; gap: 8px; display: grid}\n.biliup-ui .field{border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--panel-2); align-items: center; gap: 6px; min-width: 0px; height: 28px; padding: 0px 8px; display: flex}\n.biliup-ui .field .k{color: var(--dim); font-size: 11px}\n.biliup-ui .field .v{font-family: var(--mono); margin-left: auto; font-size: 12px}\n.biliup-ui .field.input{outline: 0px; width: 100%}\n.biliup-ui .bottom{border-top: 1px solid var(--border); background: var(--panel); flex-direction: column; grid-area: bottom; min-width: 0px; display: flex}\n.biliup-ui .bottom-toolbar{border-bottom: 1px solid var(--border); flex: 0 0 auto; align-items: center; gap: 12px; height: 38px; padding: 0px 8px; display: flex}\n.biliup-ui .seg{border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 2px; display: flex}\n.biliup-ui .seg button{color: var(--muted); border-radius: 4px; padding: 3px 10px}\n.biliup-ui .seg button.active{background: var(--raised); color: var(--fg)}\n.biliup-ui .bottom-body{flex: 1 1 0%; min-height: 0px; overflow: auto}\n.biliup-ui .tl{grid-template-rows: 1fr; grid-template-columns: 184px 1fr; width: max-content; min-width: 100%; min-height: 100%; display: grid; position: relative}\n.biliup-ui .tl-heads{z-index: 3; background: var(--panel); position: sticky; left: 0px}\n.biliup-ui .tl-corner{border-bottom: 1px solid var(--border); border-right: 1px solid var(--border); z-index: 2; background: var(--panel); height: 44px; position: sticky; top: 0px}\n.biliup-ui .tl-lanes{z-index: 0; position: relative}\n.biliup-ui .tl-ruler{z-index: 8; border-bottom: 1px solid var(--border); background: var(--panel); height: 44px; position: sticky; top: 0px}\n.biliup-ui .tl-ruler .tick{border-left: 1px solid var(--border-strong); position: absolute; top: 24px; bottom: 0px}\n.biliup-ui .tl-ruler .tick span{font: 10px var(--mono); color: var(--dim); position: absolute; top: -2px; left: 4px}\n.biliup-ui .tl-head{border-bottom: 1px solid var(--border); border-right: 1px solid var(--border); align-items: center; gap: 4px; height: 40px; padding: 0px 6px 0px 10px; display: flex}\n.biliup-ui .tl-head .name{white-space: nowrap; flex: 1 1 0%; align-items: center; gap: 6px; min-width: 0px; font-size: 12px; display: flex}\n.biliup-ui .tl-head .name .badge{color: var(--muted); display: inline-flex}\n.biliup-ui .tl-head .name .badge svg.lucide{width: 13px; height: 13px}\n.biliup-ui .tl-head .icon-btn{width: 22px; height: 22px}\n.biliup-ui .tl-head .icon-btn svg.lucide{width: 13px; height: 13px}\n.biliup-ui .tl-head.main{background: var(--panel-2); height: 52px}\n.biliup-ui .tl-lane{border-bottom: 1px solid var(--border); height: 40px; position: relative}\n.biliup-ui .tl-lane.main{background: var(--panel-2); height: 52px}\n.biliup-ui .playhead{height: calc(var(--tl-view-h,200px) - 1px); background: var(--fg); z-index: 4; pointer-events: none; width: 1px; position: absolute; top: 0px}\n.biliup-ui .playhead-handle{pointer-events: auto; cursor: ew-resize; width: 15px; height: 44px; position: absolute; top: 0px; left: -7px}\n.biliup-ui .playhead::before{content: \"\"; border-right: 5px solid rgba(0, 0, 0, 0); border-bottom: 5px solid rgba(0, 0, 0, 0); border-left: 5px solid rgba(0, 0, 0, 0); border-image: none; border-top: 7px solid var(--fg); position: absolute; top: 0px; left: -5px}\n.biliup-ui .bottom{position: relative}\n.biliup-ui .modal-layer{z-index: 50; background: rgba(0, 0, 0, 0.6); flex-direction: column; padding: 24px; display: flex; position: fixed; inset: 0px; overflow: auto}\n.biliup-ui .modal-layer > .modal{flex: 0 0 auto; margin: auto}\n.biliup-ui .link-btn{color: var(--muted); border-radius: var(--radius-sm); align-items: center; gap: 5px; padding: 3px 7px; font-size: 12px; display: inline-flex}\n.biliup-ui .link-btn svg.lucide{width: 14px; height: 14px}\n.biliup-ui .wiz-foot p{color: var(--muted); flex: 1 1 0%; margin: 0px; font-size: 12px; line-height: 1.5}\n.biliup-ui .wiz-foot{justify-content: space-between; gap: 20px}\n.biliup-ui .library{position: relative}\n.biliup-ui .lib-section{padding: 6px 14px 14px}\n.biliup-ui .lib-label{color: var(--muted); margin: 4px 0px 8px; font-size: 11px}\n.biliup-ui .grid2{grid-template-columns: repeat(auto-fill, minmax(96px, 1fr)); gap: 8px; display: grid}\n.biliup-ui .tile{border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--panel-2); text-align: left; min-width: 0px; overflow: hidden}\n.biliup-ui .thumb{aspect-ratio: 16 / 10; color: var(--dim); background: repeating-linear-gradient(135deg, rgb(26, 26, 26) 0px, rgb(26, 26, 26) 6px, rgb(22, 22, 22) 6px, rgb(22, 22, 22) 12px); justify-content: center; align-items: center; display: flex; position: relative; overflow: hidden}\n.biliup-ui .thumb .dur{font: 10px var(--mono); color: var(--fg); background: rgba(0, 0, 0, 0.6); border-radius: 3px; padding: 0px 4px; position: absolute; bottom: 4px; right: 4px}\n.biliup-ui .tile-name{white-space: nowrap; text-overflow: ellipsis; padding: 5px 7px 0px; font-size: 11px; overflow: hidden}\n.biliup-ui .tile-sub{color: var(--muted); padding: 1px 7px 6px; font-size: 10px; line-height: 1.35}\n.biliup-ui [data-testid=\"media-tile\"] .tile-name{white-space: normal; overflow-wrap: anywhere; -webkit-line-clamp: 2; -webkit-box-orient: vertical; line-height: 1.3; display: -webkit-box}\n.biliup-ui .media-bar{justify-content: space-between; align-items: center; gap: 6px; display: flex}\n.biliup-ui .media-bar-tools{align-items: center; gap: 2px; display: inline-flex}\n.biliup-ui .media-bar-tools .icon-btn{width: 22px; height: 22px}\n.biliup-ui .media-bar-tools .icon-btn svg.lucide{width: 13px; height: 13px}\n.biliup-ui .media-bar-tools .icon-btn.active{color: var(--fg); background: var(--raised)}\n.biliup-ui .media-sort{background: var(--panel-2); height: 22px; color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius-sm); margin-right: 4px; padding: 0px 4px; font-size: 11px}\n.biliup-ui .tile-used{background: var(--select); color: rgb(0, 0, 0); pointer-events: none; border-radius: 3px; height: 15px; padding: 0px 5px; font-size: 9.5px; font-weight: 600; line-height: 15px; position: absolute; top: 4px; left: 4px}\n.biliup-ui .toolbar-spacer{flex: 1 1 0%}\n.biliup-ui .bottom-body{overscroll-behavior: contain}\n.biliup-ui .tl-ruler{cursor: ew-resize; user-select: none}\n.biliup-ui .tl-ruler .tick span{white-space: nowrap}\n.biliup-ui .clip{cursor: pointer; user-select: none; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 5px; flex-direction: column; justify-content: space-between; padding: 3px 6px; font-size: 11px; display: flex; position: absolute; top: 4px; bottom: 4px; overflow: hidden}\n.biliup-ui .clip .nm,.biliup-ui .clip .meta{z-index: 1; white-space: nowrap; text-overflow: ellipsis; position: relative; overflow: hidden}\n.biliup-ui .clip .meta{font: 10px var(--mono); color: rgba(255, 255, 255, 0.6)}\n.biliup-ui .clip.video{background: rgb(29, 29, 29)}\n.biliup-ui .clip.video .nm,.biliup-ui .clip.video .meta{text-shadow: rgb(0, 0, 0) 0px 1px 3px}\n.biliup-ui .clip .strip{opacity: 0.85; background-repeat: repeat-x; background-size: auto 100%; position: absolute; inset: 0px}\n.biliup-ui .clip .filmstrip{pointer-events: none; overflow: hidden}\n.biliup-ui .clip.text{background: rgb(59, 47, 92); border-color: rgb(90, 74, 138)}\n.biliup-ui .clip.audio{background: rgb(23, 53, 42); border-color: rgb(36, 80, 58)}\n.biliup-ui .clip.graphic{background: rgb(70, 48, 28); border-color: rgb(106, 74, 44)}\n.biliup-ui .tile{position: relative}\n.biliup-ui .tile[draggable=\"true\"]{cursor: grab}\n.biliup-ui .tile-add{background: var(--fg); color: rgb(0, 0, 0); opacity: 0; border-radius: 50%; place-items: center; width: 22px; height: 22px; padding: 0px; display: grid; position: absolute; top: 4px; right: 4px}\n.biliup-ui .tile-add svg.lucide{stroke-width: 2.25px; width: 14px; height: 14px}\n.biliup-ui .tile-analyze{width: 22px; height: 22px; color: var(--fg); opacity: 0; background: rgba(0, 0, 0, 0.6); border-radius: 50%; place-items: center; padding: 0px; display: grid; position: absolute; top: 4px; right: 30px}\n.biliup-ui .tile-analyze svg.lucide{width: 13px; height: 13px}\n.biliup-ui .agent{flex-direction: column; flex: 1 1 0%; min-height: 0px; display: flex}\n.biliup-ui .agent-thread{overscroll-behavior: contain; flex-direction: column; flex: 1 1 0%; gap: 10px; min-height: 0px; padding: 12px; display: flex; overflow: auto}\n.biliup-ui .turn-wrap{flex-direction: column; gap: 6px; display: flex}\n.biliup-ui .msg-user{background: var(--raised); border-radius: var(--radius); white-space: pre-wrap; overflow-wrap: anywhere; align-self: flex-end; max-width: 88%; padding: 7px 10px}\n.biliup-ui .turn{flex-direction: column; gap: 5px; padding: 2px 0px 0px; display: flex}\n.biliup-ui .turn > .reply{border: 1px solid var(--border); border-radius: var(--radius); background: var(--panel-2); align-self: flex-start; max-width: 100%; padding: 2px 10px 4px}\n.biliup-ui .turn > .worklog,.biliup-ui .turn > .edit-row{padding: 0px 4px}\n.biliup-ui .step{color: var(--muted); align-items: flex-start; gap: 6px; font-size: 12px; line-height: 1.4; display: flex}\n.biliup-ui .step svg.lucide{width: 13px; height: 13px; margin-top: 2px}\n.biliup-ui .reply p{overflow-wrap: anywhere; margin: 6px 0px 2px; line-height: 1.5}\n.biliup-ui .turn-edits{display: inline-flex; position: relative}\n.biliup-ui .turn-edits-list{flex-direction: column; gap: 6px; display: flex}\n.biliup-ui .turn-edits-label{border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--muted); font-style: inherit; font-variant: inherit; font-weight: inherit; font-stretch: inherit; line-height: inherit; font-family: inherit; font-optical-sizing: inherit; font-size-adjust: inherit; font-kerning: inherit; font-feature-settings: inherit; font-variation-settings: inherit; font-language-override: inherit; cursor: pointer; background: 0px 0px; align-items: center; gap: 4px; padding: 1px 6px; font-size: 11px; display: inline-flex}\n.biliup-ui .turn-edits-label svg.lucide{width: 11px; height: 11px}\n.biliup-ui .turn-edit-summary{font-size: 12px; font-weight: 500}\n.biliup-ui .turn-edit-line{color: var(--muted); overflow-wrap: anywhere; padding-left: 10px; font-size: 11px; line-height: 1.4}\n.biliup-ui .turn-foot{color: var(--dim); white-space: nowrap; flex-wrap: nowrap; align-items: center; gap: 6px; min-width: 0px; min-height: 24px; padding: 0px 2px 0px 4px; font-size: 11px; display: flex}\n.biliup-ui .turn-status{text-overflow: ellipsis; flex: 0 1 auto; min-width: 0px; display: block; overflow: hidden}\n.biliup-ui .turn-foot > .turn-edits,.biliup-ui .turn-foot > .link-btn{flex: 0 0 auto}\n.biliup-ui .turn-wrap{container: turn / inline-size}\n.biliup-ui .turn-foot .spacer{flex: 1 1 0%}\n.biliup-ui .turn-foot .link-btn svg.lucide{width: 13px; height: 13px}\n.biliup-ui .composer{border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--panel-2); flex-direction: column; flex: 0 0 auto; margin: 0px 12px 12px; display: flex}\n.biliup-ui .composer-ctx{color: var(--dim); align-items: center; gap: 6px; min-width: 0px; padding: 7px 10px 0px; font-size: 11px; display: flex}\n.biliup-ui .composer-ctx svg.lucide{width: 12px; height: 12px}\n.biliup-ui .composer{position: relative}\n.biliup-ui .composer-box{display: grid; position: relative}\n.biliup-ui .composer-box > textarea,.biliup-ui .composer-hl{font: 13px/1.45 var(--font); white-space: pre-wrap; overflow-wrap: anywhere; letter-spacing: normal; grid-area: 1 / 1; min-height: 58px; max-height: 180px; padding: 6px 10px}\n.biliup-ui .composer textarea{resize: none; color: var(--fg); background: 0px 0px; border: 0px; outline: 0px; position: relative; overflow-y: auto}\n.biliup-ui .composer-hl{color: rgba(0, 0, 0, 0); pointer-events: none; overflow: hidden}\n.biliup-ui .composer-hl mark.chip,.biliup-ui .msg-user mark.chip{background: var(--select-soft); color: inherit; border-radius: 4px; box-shadow: rgba(109, 140, 255, 0.3) 0px 0px 0px 1px}\n.biliup-ui .msg-user mark.chip{color: rgb(201, 212, 255); padding: 0px 2px}\n.biliup-ui .composer-at{color: var(--dim)}\n.biliup-ui .composer-at svg.lucide{width: 14px; height: 14px}\n.biliup-ui .sr-only{clip: rect(0px, 0px, 0px, 0px); white-space: nowrap; width: 1px; height: 1px; position: absolute; overflow: hidden}\n.biliup-ui .turn-wrap{user-select: text; cursor: auto}\n.biliup-ui .turn-wrap button,.biliup-ui .turn-wrap svg,.biliup-ui .turn-wrap .sr-only,.biliup-ui .copy-btn{user-select: none}\n.biliup-ui .turn-wrap .edit-step{cursor: pointer}\n.biliup-ui .copy-btn{flex: 0 0 auto; width: 22px; height: 22px}\n.biliup-ui .copy-btn svg.lucide{width: 12px; height: 12px}\n.biliup-ui .reply{position: relative}\n.biliup-ui .reply .reply-copy{opacity: 0; background: var(--panel-2); position: absolute; top: 2px; right: 2px}\n.biliup-ui .insp-chat{flex: 0 0 auto; align-items: center; gap: 4px; font-weight: 500; display: inline-flex}\n.biliup-ui .insp-chat svg.lucide{width: 12px; height: 12px}\n.biliup-ui .composer textarea::placeholder{color: var(--dim)}\n.biliup-ui .composer-foot{align-items: center; gap: 8px; padding: 0px 6px 6px 10px; display: flex}\n.biliup-ui .composer-spacer{flex: 1 1 0px; min-width: 0px}\n.biliup-ui .composer{container-type: inline-size}\n.biliup-ui .composer-foot{flex-wrap: nowrap; gap: 2px; min-width: 0px; padding-left: 4px}\n.biliup-ui .composer-foot > .icon-btn,.biliup-ui .composer-foot > .send{flex: 0 0 auto}\n.biliup-ui .composer-choices{flex-wrap: nowrap; flex: 0 1 auto; align-items: center; gap: 1px; min-width: 0px; display: flex}\n.biliup-ui .composer-choices .choice{border-radius: var(--radius-sm); height: 24px; color: var(--muted); white-space: nowrap; flex: 0 0 auto; align-items: center; gap: 4px; padding: 0px 6px; font-size: 11.5px; display: inline-flex}\n.biliup-ui .composer-choices .choice svg.lucide{flex: 0 0 auto; width: 12px; height: 12px}\n.biliup-ui .composer-choices .choice.model{flex: 0 1 auto; min-width: 0px; overflow: hidden}\n.biliup-ui .composer-choices .choice.model .choice-label{text-overflow: ellipsis; flex: 0 100 auto; min-width: 0px; overflow: hidden}\n.biliup-ui .composer-choices .choice.model .provider-wordmark{text-overflow: ellipsis; flex: 0 1 auto; min-width: 0px; overflow: hidden}\n.biliup-ui .composer-choices .choice svg.choice-caret{width: 11px; height: 11px}\n.biliup-ui .send{background: var(--fg); color: rgb(0, 0, 0); border-radius: 50%; place-items: center; width: 26px; height: 26px; display: grid}\n.biliup-ui .send svg.lucide{width: 15px; height: 15px}\n.biliup-ui .send:disabled{background: var(--raised); color: var(--dim); opacity: 1}\n.biliup-ui .toolbar-sep{background: var(--border-strong); width: 1px; height: 16px; margin: 0px 4px}\n.biliup-ui .icon-btn.on{color: var(--fg)}\n.biliup-ui .clip .edge{z-index: 2; cursor: ew-resize; width: 6px; position: absolute; top: 0px; bottom: 0px}\n.biliup-ui .clip .edge.start{border-radius: 5px 0px 0px 5px; left: 0px}\n.biliup-ui .clip .edge.end{border-radius: 0px 5px 5px 0px; right: 0px}\n.biliup-ui .tool-picker{align-items: center; gap: 2px; display: flex}\n.biliup-ui .kv-row{border-bottom: 1px solid var(--border); justify-content: space-between; gap: 12px; padding: 4px 0px; font-size: 12px; display: flex}\n.biliup-ui .kv-row:last-child{border-bottom: 0px}\n.biliup-ui .kv-row .mono{text-align: right}\n.biliup-ui .small-btn{height: 22px; padding: 0px 6px; font-size: 11px}\n.biliup-ui .slider-row{grid-template-columns: 1fr 92px; align-items: center; gap: 8px; margin-bottom: 8px; display: grid}\n.biliup-ui .slider{width: 100%; accent-color: var(--select)}\n.biliup-ui .num-field input{text-align: right; width: 100%; min-width: 0px; color: inherit; background: 0px 0px; border: 0px; outline: 0px}\n.biliup-ui .num-field input::-webkit-inner-spin-button{display: none}\n.biliup-ui .check-row{align-items: center; gap: 8px; margin-bottom: 8px; font-size: 12px; display: flex}\n.biliup-ui .clip .wave{pointer-events: none; width: 100%; height: 100%; position: absolute; bottom: 0px; left: 0px; right: 0px}\n.biliup-ui .clip.audio .wave{top: 0px}\n.biliup-ui .clip .fade-handle{z-index: 3; cursor: ew-resize; opacity: 0; background: rgb(255, 255, 255); border: 1px solid rgba(0, 0, 0, 0.55); border-radius: 3px; width: 12px; height: 12px; margin-left: -6px; margin-right: -6px; position: absolute; top: 1px; box-shadow: rgba(0, 0, 0, 0.45) 0px 1px 3px}\n.biliup-ui .clip .fade-handle::before{content: \"\"; position: absolute; inset: -4px -5px -6px}\n.biliup-ui .clip.audio .fade-handle{opacity: 0.55}\n.biliup-ui .track-volume{display: inline-flex; position: relative}\n.biliup-ui .export-head{justify-content: space-between; align-items: center; margin-bottom: 16px; display: flex}\n.biliup-ui .export-fields{margin-top: 12px}\n.biliup-ui .export-label{margin: 16px 0px 8px}\n.biliup-ui .export-note{color: var(--muted); margin: 0px auto 0px 0px; font-size: 12px}\n.biliup-ui .export-status{justify-content: space-between; gap: 12px; margin-bottom: 8px; font-size: 13px; display: flex}\n.biliup-ui .export-bar{background: var(--raised); border-radius: 3px; height: 6px; overflow: hidden}\n.biliup-ui .export-bar > div{background: var(--fg); border-radius: 3px; height: 100%}\n.biliup-ui .export-btn{margin-right: 4px}\n.biliup-ui .export-advanced-toggle{color: var(--muted); cursor: pointer; background: 0px 0px; border: 0px; align-items: center; gap: 4px; margin-top: 10px; padding: 2px 0px; font-size: 12px; display: inline-flex}\n.biliup-ui .export-advanced-toggle svg.lucide{width: 14px; height: 14px}\n.biliup-ui .fx-tile{cursor: pointer}\n.biliup-ui .fx-tile.active{border-color: var(--select); box-shadow: inset 0 0 0 1px var(--select)}\n.biliup-ui .fx-thumb{aspect-ratio: 16 / 9; background: rgb(13, 13, 13)}\n.biliup-ui .fx-thumb canvas{width: 100%; height: 100%; display: block; position: absolute; inset: 0px}\n.biliup-ui .fx-thumb-overlay{transform-origin: 0px 0px; pointer-events: none; width: 1920px; height: 1080px; position: absolute; top: 0px; left: 0px}\n.biliup-ui .fx-none svg.lucide{width: 22px; height: 22px; color: var(--dim)}\n.biliup-ui .fx-entry{border-top: 1px solid var(--border); flex-direction: column; gap: 4px; padding: 4px 0px 6px; display: flex}\n.biliup-ui .fx-entry:first-child{border-top: 0px}\n.biliup-ui .fx-entry-head{color: var(--fg); justify-content: space-between; align-items: center; font-size: 11px; display: flex}\n.biliup-ui .fx-entry-head .icon-btn{width: 22px; height: 22px}\n.biliup-ui .fx-entry-head .icon-btn svg.lucide{width: 14px; height: 14px}\n.biliup-ui .tr-search{align-items: center; display: flex; position: relative}\n.biliup-ui .tr-search .input{flex: 1 1 0%; padding-right: 26px}\n.biliup-ui .cut-point{z-index: 3; cursor: pointer; border-radius: 3px; width: 12px; height: 18px; margin-left: -6px; position: absolute; top: 2px}\n.biliup-ui .cut-point::after{content: \"\"; background: 0px 0px; border-radius: 1px; width: 2px; position: absolute; top: 3px; bottom: 3px; left: 5px}\n.biliup-ui .field-row.key-row{grid-template-columns: 1fr 24px 1fr 24px; align-items: center}\n.biliup-ui .slider-row.key-row{grid-template-columns: 1fr 120px 24px}\n.biliup-ui .key-btn{width: 24px; height: 24px}\n.biliup-ui .key-btn svg{width: 12px; height: 12px}\n.biliup-ui .seg.fit{margin-top: 8px}\n.biliup-ui .seg.fit button{flex: 1 1 0%}\n.biliup-ui .clip .envelope{z-index: 2; pointer-events: none; width: 100%; height: 100%; position: absolute; inset: 0px; overflow: visible}\n.biliup-ui .clip .envelope .env-line{fill: none; stroke: rgb(243, 201, 107); stroke-width: 1.5px; vector-effect: non-scaling-stroke}\n.biliup-ui .check-row{text-align: left; border-radius: var(--radius-sm); align-items: flex-start; gap: 8px; width: 100%; padding: 6px 6px 2px; display: flex}\n.biliup-ui .music-filters{flex-direction: column; gap: 6px; margin-bottom: 6px; display: flex}\n.biliup-ui .music-filters .input{height: 26px}\n.biliup-ui .approval{border-radius: var(--radius-sm); background: rgba(251, 191, 36, 0.08); border: 1px solid rgba(251, 191, 36, 0.4); flex-direction: column; gap: 6px; margin-top: 8px; padding: 10px 12px; display: flex}\n.biliup-ui .approval-text{font-size: 13px; font-weight: 600}\n.biliup-ui .approval-sub{color: var(--muted); font-size: 11px}\n.biliup-ui .approval-btns{justify-content: flex-end; gap: 8px; display: flex}\n.biliup-ui .export-subs{border: 1px solid var(--border); background: var(--bg); border-radius: 10px; gap: 2px; padding: 6px 12px; display: grid}\n.biliup-ui .export-sub-row{align-items: center; gap: 10px; min-height: 30px; font-size: 13px; display: flex}\n.biliup-ui .export-sub-row .muted{font-size: 12px}\n.biliup-ui .grow{flex: 1 1 0%}\n.biliup-ui .check-row{align-items: center; gap: 8px; font-size: 12px; display: flex}\n.biliup-ui .stage-canvas{position: relative; overflow: hidden}\n.biliup-ui .zoom-select{background: var(--panel-2); height: 24px; color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0px 4px; font-size: 11px}\n.biliup-ui .stage-canvas > .frame{flex: 0 0 auto}\n.biliup-ui .ctx-menu{z-index: 80; backdrop-filter: blur(18px) saturate(1.4); user-select: none; background: rgba(30, 30, 30, 0.92); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 9px; outline: none; min-width: 220px; padding: 5px; font-size: 12.5px; position: fixed; box-shadow: rgba(0, 0, 0, 0.55) 0px 12px 32px, rgba(0, 0, 0, 0.6) 0px 0px 0px 0.5px}\n.biliup-ui .ctx-item{height: 24px; color: var(--fg); cursor: default; white-space: nowrap; border-radius: 5px; align-items: center; gap: 6px; padding: 0px 10px 0px 4px; display: flex}\n.biliup-ui .ctx-item.active{background: var(--select); color: rgb(255, 255, 255)}\n.biliup-ui .ctx-item.disabled{color: var(--dim)}\n.biliup-ui .ctx-item.danger:not(.disabled):not(.active){color: rgb(255, 138, 138)}\n.biliup-ui .ctx-check{justify-content: center; width: 14px; display: inline-flex}\n.biliup-ui .ctx-label{flex: 1 1 0%}\n.biliup-ui .ctx-key{color: var(--muted); letter-spacing: 0.02em; padding-left: 18px; font-size: 11.5px}\n.biliup-ui .ctx-item.active .ctx-key{color: rgba(255, 255, 255, 0.8)}\n.biliup-ui .ctx-more{width: 12px; height: 12px; color: var(--muted); margin-left: 8px}\n.biliup-ui .ctx-sep{background: rgba(255, 255, 255, 0.09); height: 1px; margin: 4px 8px}\n.biliup-ui .speed-row{flex-wrap: wrap; gap: 4px; display: flex}\n.biliup-ui .speed-row .small-btn.active{background: var(--select); color: rgb(255, 255, 255); border-color: rgba(0, 0, 0, 0)}\n.biliup-ui .psec{border-bottom: 1px solid var(--border); padding: 4px 14px 10px}\n.biliup-ui .psec-head{justify-content: space-between; align-items: center; min-height: 30px; display: flex}\n.biliup-ui .psec-title{color: var(--fg); letter-spacing: 0.01em; border-radius: 4px; align-items: center; gap: 4px; margin-left: -4px; padding: 2px 4px; font-size: 11px; font-weight: 600; display: inline-flex}\n.biliup-ui .psec-title svg.lucide{width: 12px; height: 12px; color: var(--dim)}\n.biliup-ui .psec-head .icon-btn{width: 22px; height: 22px}\n.biliup-ui .psec-head .icon-btn svg.lucide{width: 14px; height: 14px}\n.biliup-ui .psec-body{flex-direction: column; gap: 6px; display: flex}\n.biliup-ui .psec-body[hidden]{display: none}\n.biliup-ui .psec-actions{align-items: center; gap: 6px; display: flex}\n.biliup-ui .psec-body .group{border-bottom: 0px; padding: 2px 0px 6px}\n.biliup-ui .psec-body .group + .group{border-top: 1px solid var(--border); padding-top: 8px}\n.biliup-ui .fx-entry-head{gap: 6px}\n.biliup-ui .fx-entry-head .grow{flex: 1 1 0%; min-width: 0px}\n.biliup-ui .fx-power{display: inline-flex}\n.biliup-ui .prop{grid-template-columns: 70px minmax(0px, 1fr) 18px; align-items: center; gap: 6px; min-height: 28px; display: grid}\n.biliup-ui .prop-k{color: var(--muted); white-space: nowrap; text-overflow: ellipsis; font-size: 11px; overflow: hidden}\n.biliup-ui .prop-v{align-items: center; min-width: 0px; display: flex}\n.biliup-ui .prop-v > *{flex: 1 1 0%; min-width: 0px}\n.biliup-ui .prop-reset-gap{width: 18px; height: 18px}\n.biliup-ui .icon-btn svg.lucide{width: 15px; height: 15px}\n.biliup-ui .nfield{border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--panel-2); align-items: center; min-width: 0px; height: 28px; display: flex; overflow: hidden}\n.biliup-ui .nfield input{min-width: 0px; height: 100%; color: var(--fg); font: 12px var(--mono); text-align: left; background: 0px 0px; border: 0px; outline: 0px; flex: 1 1 0%; padding: 0px 6px}\n.biliup-ui .nfield-u{color: var(--dim); flex: 0 0 auto; padding-right: 8px; font-size: 11px}\n.biliup-ui .slider-num{grid-template-columns: minmax(0px, 1fr) 76px; align-items: center; gap: 8px; width: 100%; display: grid}\n.biliup-ui .ptoggle{background: var(--raised); border: 1px solid var(--border-strong); border-radius: 9px; flex: 0 0 auto; width: 30px; height: 18px; position: relative}\n.biliup-ui .ptoggle span{background: var(--muted); border-radius: 50%; width: 12px; height: 12px; position: absolute; top: 2px; left: 2px}\n.biliup-ui .ptoggle.on{background: var(--select); border-color: rgba(0, 0, 0, 0)}\n.biliup-ui .ptoggle.on span{background: rgb(255, 255, 255); transform: translate(12px)}\n.biliup-ui .psec-head .icon-btn,.biliup-ui .ptoggle,.biliup-ui .psec-title{padding: 0px}\n.biliup-ui .psec-title{padding: 2px 4px}\n.biliup-ui .insp-title .grow{flex: 1 1 0%; min-width: 0px}\n.biliup-ui .approval-text svg{vertical-align: -2px; width: 14px; height: 14px; color: var(--muted); margin-right: 6px}\n.biliup-ui .storyboard-card{border-color: var(--select); background: var(--select-soft)}\n.biliup-ui .beats{flex-direction: column; gap: 6px; display: flex}\n.biliup-ui .beat{border-left: 2px solid var(--select); background: var(--panel); border-radius: 0 var(--radius-sm) var(--radius-sm) 0; flex-direction: column; gap: 2px; padding: 6px 8px; font-size: 12px; line-height: 1.35; display: flex}\n.biliup-ui .beat-head{align-items: baseline; gap: 8px; display: flex}\n.biliup-ui .beat-time{font-family: var(--mono); color: var(--muted); flex: 0 0 auto; font-size: 11px}\n.biliup-ui .card-item{border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--panel-2); flex-direction: column; font-size: 12px; display: flex}\n.biliup-ui .card-head{color: var(--fg); font-style: inherit; font-variant: inherit; font-weight: inherit; font-stretch: inherit; line-height: inherit; font-family: inherit; font-optical-sizing: inherit; font-size-adjust: inherit; font-kerning: inherit; font-feature-settings: inherit; font-variation-settings: inherit; font-language-override: inherit; text-align: left; cursor: pointer; background: 0px 0px; border: 0px; align-items: center; gap: 6px; width: 100%; padding: 6px 8px; font-size: 12px; display: flex}\n.biliup-ui .card-head svg{width: 13px; height: 13px; color: var(--muted); flex: 0 0 auto}\n.biliup-ui .card-item .scores{padding: 0px 8px 8px}\n.biliup-ui .card-outcome{background: var(--raised); color: var(--muted); border-radius: 8px; flex: 0 0 auto; padding: 1px 6px; font-size: 10.5px}\n.biliup-ui .card-outcome.approved{color: var(--ok)}\n.biliup-ui .scores{flex-direction: column; gap: 3px; display: flex}\n.biliup-ui .score{grid-template-columns: 112px 1fr 26px; align-items: center; gap: 8px; font-size: 11.5px; display: grid}\n.biliup-ui .score-bar{background: var(--raised); border-radius: 3px; height: 6px; overflow: hidden}\n.biliup-ui .score-bar > span{background: var(--ok); height: 100%; display: block}\n.biliup-ui .score-value{font-family: var(--mono); text-align: right}\n.biliup-ui .plan-strip{z-index: 3; border-bottom: 1px solid var(--border); flex: 0 0 auto; justify-content: flex-end; padding: 6px 12px; display: flex; position: relative}\n.biliup-ui .plan-chip-btn{border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--panel); color: var(--fg); font-style: inherit; font-variant: inherit; font-weight: inherit; font-stretch: inherit; line-height: inherit; font-family: inherit; font-optical-sizing: inherit; font-size-adjust: inherit; font-kerning: inherit; font-feature-settings: inherit; font-variation-settings: inherit; font-language-override: inherit; cursor: pointer; align-items: center; gap: 6px; padding: 3px 9px; font-size: 11.5px; display: inline-flex}\n.biliup-ui .plan-chip-btn svg{width: 12px; height: 12px; color: var(--muted); flex: 0 0 auto}\n.biliup-ui .plan-state{color: var(--muted); flex: 0 0 auto; font-size: 10px}\n.biliup-ui .plan-state.passed{color: var(--ok)}\n.biliup-ui .card-item{scroll-margin-top: 8px}\n.biliup-ui .review-by{padding: 0px 8px 6px}\n.biliup-ui .storyboard-card .approval-btns{flex-wrap: wrap}\n.biliup-ui .storyboard-card .approval-btns .btn{align-items: center; gap: 4px; display: inline-flex}\n.biliup-ui .storyboard-card .approval-btns .btn svg{width: 12px; height: 12px}\n.biliup-ui .agent-thread{overflow-x: hidden}\n.biliup-ui .storyboard-card,.biliup-ui .storyboard-card .beats,.biliup-ui .beat{min-width: 0px}\n.biliup-ui .step .grow{flex: 1 1 0%; min-width: 0px}\n.biliup-ui .step svg.lucide.step-chev{opacity: 0.6; width: 12px; height: 12px}\n.biliup-ui .agent-strip{align-items: center; gap: 8px}\n.biliup-ui .agent-strip .spacer{flex: 1 1 0%}\n.biliup-ui .agent-strip{justify-content: flex-start; padding-left: 8px}\n.biliup-ui .agent-new-chat svg.lucide{width: 15px; height: 15px}\n.biliup-ui .plan-chip-wrap{display: flex}\n.biliup-ui .edit-step{cursor: pointer; color: var(--fg)}\n.biliup-ui .edit-step svg.lucide:first-child{color: var(--select)}\n.biliup-ui .num-field .k{cursor: ew-resize; user-select: none}\n.biliup-ui .rail button{position: relative}\n.biliup-ui .provider-mark{width: 16px; height: 16px; color: var(--fg); flex: 0 0 auto; justify-content: center; align-items: center; display: inline-flex}\n.biliup-ui .provider-mark .provider-logo{width: 100%; height: 100%; display: block}\n.biliup-ui .provider-wordmark{letter-spacing: -0.01em; color: var(--fg); font-weight: 600}\n.biliup-ui .composer-choices .choice .provider-wordmark{font-size: 11.5px}\n.biliup-ui{text-align:left;-webkit-font-smoothing:antialiased}.biliup-ui .biliup-ui-caret{display:inline-block;width:1.5px;height:1.2em;margin:0 -1.5px 0 0;vertical-align:-0.25em;background:var(--fg)}.biliup-ui.biliup-ui-light{--bg:#fff;--panel:#fff;--panel-2:#fff;--raised:#efeeea;--border:#00000010;--border-strong:#0000001c;--fg:#111;--muted:#62625e;--dim:#9a9993;--select-soft:#6d8cff24;color-scheme:light;color:var(--fg)}.biliup-ui.biliup-ui-light .send:not(:disabled){color:#fff}.biliup-ui.biliup-ui-light .clip{border-color:#0000001a}","parts":{"composer":{"html":"\u003cdiv class=\"composer\" data-testid=\"composer\">\u003cdiv class=\"composer-ctx\" data-testid=\"agent-context\">\u00010\u0002\u003cspan class=\"ellipsis\">Playhead 00:00:00:00\u003c/span>\u003c/div>\u003cdiv class=\"composer-box\">\u003cdiv class=\"composer-hl\">Make a 28 s launch film for Biliup on the Electro Dreams beat, the drone clips as the film being edited.​\u003c/div>\u003ctextarea id=\"agent-input\" data-testid=\"agent-input\" rows=\"3\" placeholder=\"Ask for an edit; @ to point at media or a clip, / for a skill or a template\">Make a 28 s launch film for Biliup on the Electro Dreams beat, the drone clips as the film being edited.\u003c/textarea>\u003c/div>\u003cdiv class=\"composer-foot\">\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-attach\">\u00011\u0002\u003c/button>\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-at\">\u00012\u0002\u003c/button>\u003cdiv class=\"composer-choices\">\u003cbutton class=\"choice provider\" data-testid=\"agent-provider\" data-provider=\"claude\">\u003cspan class=\"provider-mark sm\" data-testid=\"provider-mark\" data-provider=\"claude\">\u00013\u0002\u003c/span>\u003c/button>\u003cbutton class=\"choice model\" data-testid=\"agent-model\" data-model=\"\">\u003cspan class=\"provider-wordmark\" data-testid=\"provider-wordmark\" data-provider=\"claude\">Claude\u003c/span>\u003cspan class=\"choice-label\">Default\u003c/span>\u00014\u0002\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-effort\" data-effort=\"\">\u00015\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-access\" data-access=\"auto\">\u00016\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003c/div>\u003cspan class=\"composer-spacer\">\u003c/span>\u003cbutton class=\"send\" data-testid=\"agent-send\">\u00017\u0002\u003c/button>\u003c/div>\u003c/div>","width":415,"height":127,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"aside","cls":"side","attrs":{},"vars":""},{"tag":"div","cls":"side-body","attrs":{},"vars":""},{"tag":"div","cls":"agent","attrs":{},"vars":""}]},"storyboard":{"html":"\u003cdiv class=\"approval storyboard-card\" data-testid=\"agent-storyboard\" data-id=\"ap-1\">\u003cdiv class=\"approval-text\">\u00018\u0002One take on Electro Dreams: the composer, the agent's plan, the timeline building, the preview flooding full-frame on the drop, the critic's card, the export.\u003c/div>\u003cdiv class=\"beats\">\u003cdiv class=\"beat\" data-testid=\"agent-storyboard-beat\">\u003cdiv class=\"beat-head\">\u003cspan class=\"beat-time\">1. 0 s to 2 s\u003c/span>\u003cb>Hook\u003c/b>\u003c/div>\u003cdiv class=\"beat-line\">\"Edit videos by talking to an agent\" huge on the canvas \u003cspan class=\"muted\">→\u003c/span> The empty composer under it\u003c/div>\u003cdiv class=\"beat-line muted\">Stop the scroll with the claim\u003c/div>\u003cdiv class=\"beat-line small muted\">Kit fonts, composer UI, on Downbeat 0\u003c/div>\u003c/div>\u003cdiv class=\"beat\" data-testid=\"agent-storyboard-beat\">\u003cdiv class=\"beat-head\">\u003cspan class=\"beat-time\">2. 2 s to 6 s\u003c/span>\u003cb>Typing the prompt\u003c/b>\u003c/div>\u003cdiv class=\"beat-line\">The cursor clicks into the composer \u003cspan class=\"muted\">→\u003c/span> Prompt typed; Send pressed at 6.0\u003c/div>\u003cdiv class=\"beat-line muted\">Show you typing this prompt\u003c/div>\u003cdiv class=\"beat-line small muted\">Composer UI, on Click on 2.5, send on 6.0\u003c/div>\u003c/div>\u003cdiv class=\"beat\" data-testid=\"agent-storyboard-beat\">\u003cdiv class=\"beat-head\">\u003cspan class=\"beat-time\">3. 6 s to 12 s\u003c/span>\u003cb>It edits the timeline\u003c/b>\u003c/div>\u003cdiv class=\"beat-line\">The panel morphs into the editor \u003cspan class=\"muted\">→\u003c/span> Camera pushes onto the preview\u003c/div>\u003cdiv class=\"beat-line muted\">The agent putting clips on real tracks\u003c/div>\u003cdiv class=\"beat-line small muted\">DJI 0055, DJI 0063, DJI 0068, on A clip lands on every beat\u003c/div>\u003c/div>\u003cdiv class=\"beat\" data-testid=\"agent-storyboard-beat\">\u003cdiv class=\"beat-head\">\u003cspan class=\"beat-time\">4. 12 s to 18 s\u003c/span>\u003cb>Drop: the film plays\u003c/b>\u003c/div>\u003cdiv class=\"beat-line\">The preview floods full-frame \u003cspan class=\"muted\">→\u003c/span> Shots grow out of each other\u003c/div>\u003cdiv class=\"beat-line muted\">The reveal: the edited film itself\u003c/div>\u003cdiv class=\"beat-line small muted\">DJI 0063, DJI 0050, on Drop 12.0\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"approval-sub\">The build waits for your approval.\u003c/div>\u003cdiv class=\"approval-btns\">\u003cbutton class=\"btn ghost\" data-testid=\"agent-storyboard-edit\">\u00019\u0002Edit\u003c/button>\u003cbutton class=\"btn ghost\" data-testid=\"agent-storyboard-change\">Ask for changes\u003c/button>\u003cbutton class=\"btn primary\" data-testid=\"agent-storyboard-approve\">Approve\u003c/button>\u003c/div>\u003c/div>","width":415,"height":533,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"aside","cls":"side","attrs":{},"vars":""},{"tag":"div","cls":"side-body","attrs":{},"vars":""},{"tag":"div","cls":"agent","attrs":{},"vars":""},{"tag":"div","cls":"agent-thread","attrs":{},"vars":""},{"tag":"div","cls":"turn-wrap","attrs":{"data-turn":"turn-1","data-status":"running","data-undo":"none"},"vars":""},{"tag":"div","cls":"turn running","attrs":{},"vars":""}],"backdrop":"rgb(17, 17, 17)","radius":"6px"},"editor":{"html":"\u003cdiv class=\"app\" data-testid=\"editor\" style=\"--side-col: clamp(240px, 440px, 40vw);\">\u003cheader class=\"topbar\">\u003cdiv class=\"wordmark\" data-testid=\"wordmark\" data-busy=\"false\">\u003cspan class=\"brand wordmark-name\">\u000110\u0002\u003cspan class=\"wordmark-text\" data-testid=\"wordmark-text\">biliup\u003c/span>\u003c/span>\u003cspan class=\"wordmark-slash\">\u003c/span>\u003cdiv class=\"project-title\" data-testid=\"project-title\">Launch film\u003c/div>\u003c/div>\u003cdiv class=\"topbar-spacer\">\u003c/div>\u003cdiv class=\"layout-toggles\">\u003cbutton class=\"icon-btn\" data-testid=\"toggle-lib\">\u000111\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-bottom\">\u000112\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-side\">\u000113\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-focus\">\u000114\u0002\u003c/button>\u003c/div>\u003cdiv class=\"saved\" data-testid=\"saved\">\u000115\u0002Saved\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"undo\">\u000116\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"redo\" disabled=\"\">\u000117\u0002\u003c/button>\u003cbutton class=\"btn export-btn\" data-testid=\"export-button\" data-running=\"false\">\u000118\u0002Export\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"open-settings\">\u000119\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"close-project\">\u000120\u0002\u003c/button>\u003c/header>\u003cdiv class=\"banners\">\u003c/div>\u003cnav class=\"rail\" data-testid=\"rail\" data-scrollbars=\"off\" data-order=\"media,transcript,text,elements,audio,sfx,transitions,effects,capture,generate,brand,skills\" data-hidden=\"\">\u003cbutton class=\"active\" data-testid=\"rail-media\" data-rail=\"media\">\u000121\u0002\u003cspan>Media\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-transcript\" data-rail=\"transcript\">\u000122\u0002\u003cspan>Transcript\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-text\" data-rail=\"text\">\u000123\u0002\u003cspan>Text\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-elements\" data-rail=\"elements\">\u000124\u0002\u003cspan>Elements\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-audio\" data-rail=\"audio\">\u000125\u0002\u003cspan>Audio\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-sfx\" data-rail=\"sfx\">\u000126\u0002\u003cspan>Sounds\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-transitions\" data-rail=\"transitions\">\u000127\u0002\u003cspan>Transitions\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-effects\" data-rail=\"effects\">\u000128\u0002\u003cspan>Effects\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-capture\" data-rail=\"capture\">\u000129\u0002\u003cspan>Capture\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-generate\" data-rail=\"generate\">\u00016\u0002\u003cspan>Generate\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-brand\" data-rail=\"brand\">\u000130\u0002\u003cspan>Brand\u003c/span>\u003c/button>\u003cbutton data-testid=\"rail-skills\" data-rail=\"skills\">\u000131\u0002\u003cspan>Skills\u003c/span>\u003c/button>\u003c/nav>\u003caside class=\"library\" data-testid=\"library\">\u003cdiv class=\"lib-head\">Media\u003cbutton class=\"icon-btn\" data-testid=\"import-plus\">\u000132\u0002\u003c/button>\u003c/div>\u003cdiv class=\"lib-section\">\u003cdiv class=\"lib-label media-bar\">\u003cspan>In this project\u003c/span>\u003cspan class=\"media-bar-tools\">\u003cselect class=\"media-sort\" data-testid=\"media-sort\">\u003coption value=\"date\">Date added\u003c/option>\u003coption value=\"name\">Name\u003c/option>\u003coption value=\"duration\">Length\u003c/option>\u003coption value=\"type\">Type\u003c/option>\u003c/select>\u003cbutton class=\"icon-btn active\" data-testid=\"media-view-grid\">\u000133\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"media-view-list\">\u000134\u0002\u003c/button>\u003c/span>\u003c/div>\u003cdiv class=\"grid2\" data-testid=\"media-grid\" data-view=\"grid\" data-sort=\"date\">\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-typing\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000135\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:02\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat second\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003cbutton class=\"tile-analyze\" data-testid=\"analyze\">\u000126\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">Keyboard typing\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">Audio\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-whoosh\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000135\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:00\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat second\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003cbutton class=\"tile-analyze\" data-testid=\"analyze\">\u000126\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">Airy whoosh\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">Audio\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-click\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000135\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:00\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat second\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003cbutton class=\"tile-analyze\" data-testid=\"analyze\">\u000126\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">Mouse click\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">Audio\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-dji-0068\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000121\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:22\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">DJI 0068\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">3840×2160, 50 fps\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-dji-0063\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000121\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:48\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">DJI 0063\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">3840×2160, 50 fps\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-dji-0055\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000121\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:38\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">DJI 0055\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">3840×2160, 50 fps\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-dji-0050\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000121\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:31\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">DJI 0050\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">3840×2160, 50 fps\u003c/div>\u003c/div>\u003cdiv class=\"tile\" data-testid=\"media-tile\" data-id=\"m-music\" data-used=\"true\">\u003cdiv class=\"thumb\">\u000135\u0002\u003cspan class=\"tile-used\" data-testid=\"media-used\">Used\u003c/span>\u003cspan class=\"dur\" data-testid=\"duration\">00:28\u003c/span>\u003cbutton class=\"tile-add\" data-testid=\"add-to-timeline\">\u000132\u0002\u003c/button>\u003cbutton class=\"tile-chat second\" data-testid=\"media-add-to-chat\">\u00012\u0002\u003c/button>\u003cbutton class=\"tile-analyze\" data-testid=\"analyze\">\u000126\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tile-name\" data-testid=\"tile-name\">electro-dreams.mp3\u003c/div>\u003cdiv class=\"tile-sub\" data-testid=\"tile-meta\">Audio\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/aside>\u003cmain class=\"stage\" data-testid=\"stage\">\u003cdiv class=\"stage-canvas\" data-testid=\"stage-canvas\" data-zoom=\"fit\">\u003cdiv class=\"frame\" data-testid=\"preview-frame\" data-frame=\"0\" data-playing=\"false\" style=\"width: 411px; height: 514px; background: rgb(0, 0, 0);\">\u003c/div>\u003c/div>\u003cdiv class=\"transport\">\u003cspan class=\"timecode\" data-testid=\"timecode\">00:00:00:00\u003cspan class=\"tc-end\"> / 00:00:28:00\u003c/span>\u003c/span>\u003cdiv class=\"transport-mid\">\u003cbutton class=\"icon-btn tp-jump\" data-testid=\"to-start\" disabled=\"\">\u000136\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-step\" data-testid=\"step-back\" disabled=\"\">\u000137\u0002\u003c/button>\u003cbutton class=\"icon-btn play\" data-testid=\"play\">\u000138\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-step\" data-testid=\"step-forward\">\u000139\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-jump\" data-testid=\"to-end\">\u000140\u0002\u003c/button>\u003c/div>\u003cspan class=\"muted small\" data-testid=\"canvas-info\">1080×1350, 60 fps\u003c/span>\u003cselect class=\"zoom-select\" data-testid=\"preview-zoom\">\u003coption value=\"fit\">Fit\u003c/option>\u003coption value=\"0.5\">50%\u003c/option>\u003coption value=\"1\">100%\u003c/option>\u003coption value=\"2\">200%\u003c/option>\u003c/select>\u003cbutton class=\"icon-btn tp-region\" data-testid=\"preview-region\">\u000141\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-hand\" data-testid=\"preview-hand\">\u000142\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"preview-fullscreen\">\u000143\u0002\u003c/button>\u003c/div>\u003c/main>\u003caside class=\"side\">\u003cdiv class=\"tabs\">\u003cbutton data-testid=\"tab-inspector\">Inspector\u003c/button>\u003cbutton class=\"active\" data-testid=\"tab-agent\">Agent\u003c/button>\u003cbutton data-testid=\"tab-history\">History\u003c/button>\u003c/div>\u003cdiv class=\"side-body\">\u003cdiv class=\"agent\" data-testid=\"agent-panel\">\u003cdiv class=\"plan-strip agent-strip\" data-testid=\"agent-strip\">\u003cbutton class=\"icon-btn agent-new-chat\" data-testid=\"agent-new-chat\">\u000144\u0002\u003c/button>\u003cspan class=\"spacer\">\u003c/span>\u003cdiv class=\"plan-chip-wrap\" data-testid=\"agent-plan\">\u003cbutton class=\"plan-chip-btn\" data-testid=\"plan-chip\">\u000145\u0002Plan\u003cspan class=\"plan-state passed\">Passed\u003c/span>\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"agent-thread\" data-testid=\"agent-thread\" data-ui-scroll=\"38\">\u003cdiv class=\"turn-wrap\" data-testid=\"agent-turn\" data-turn=\"turn-1\" data-status=\"done\" data-undo=\"available\">\u003cdiv class=\"msg-user\" data-testid=\"agent-prompt\">Make a 28 s launch film for Biliup on the Electro Dreams beat, the drone clips as the film being edited.\u003c/div>\u003cdiv class=\"turn done\" data-testid=\"agent-stream\">\u003cdiv class=\"worklog\">\u003cdiv class=\"step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"graphic_write\">\u000115\u0002\u003cspan class=\"grow\">Wrote graphic film.html\u003c/span>\u003c/div>\u003c/div>\u003cdiv class=\"edit-row\" data-testid=\"agent-edit-row\">\u003cdiv class=\"step edit-step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"edit\">\u000146\u0002\u003cspan class=\"grow\">Placed the clips on Main\u003c/span>\u000147\u0002\u003c/div>\u003c/div>\u003cdiv class=\"card-item open\" data-testid=\"agent-card\" data-type=\"review\" data-passed=\"true\">\u003cbutton class=\"card-head\">\u00015\u0002\u003cspan class=\"grow\">Review, round 2\u003c/span>\u003cspan class=\"card-outcome approved\">Passed\u003c/span>\u000148\u0002\u003c/button>\u003cdiv class=\"review-by small muted\" data-testid=\"agent-review-by\" data-by=\"critic\">Independent critic\u003c/div>\u003cdiv class=\"scores\" data-testid=\"agent-review-scores\">\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"hook\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Hook in 2 s\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"readability\" data-value=\"8\" data-applies=\"required\">\u003cspan class=\"score-name\">Phone readability\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"motion\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Motion\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"variety\" data-value=\"8\" data-applies=\"optional\">\u003cspan class=\"score-name\">Variety\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"sound\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Sound sync\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"brand\" data-value=\"9\" data-applies=\"optional\">\u003cspan class=\"score-name\">Brand\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"reply\" data-testid=\"agent-reply\">\u003cp>The film is built on the Electro Dreams beat: it opens on the claim, types your prompt, builds the timeline, floods the preview on the drop and ends on the export.\u003c/p>\u003cbutton type=\"button\" class=\"icon-btn copy-btn reply-copy\" data-testid=\"agent-copy\">\u000149\u0002\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"turn-foot\">\u003cdiv class=\"turn-edits\" data-testid=\"agent-edits\">\u003cbutton class=\"turn-edits-label\" data-testid=\"agent-edits-toggle\">\u000146\u00022 edits\u003c/button>\u003cdiv class=\"sr-only\">\u003cdiv class=\"turn-edits-list\">\u003cdiv class=\"turn-edit\" data-testid=\"agent-edit\">\u003cdiv class=\"turn-edit-summary\">Placed the clips on Main\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0055\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0063\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0068\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0050\u003c/div>\u003c/div>\u003cdiv class=\"turn-edit\" data-testid=\"agent-edit\">\u003cdiv class=\"turn-edit-summary\">Added sound effects\u003c/div>\u003cdiv class=\"turn-edit-line\">Added Keyboard typing\u003c/div>\u003cdiv class=\"turn-edit-line\">Added Mouse click\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003cspan class=\"turn-status\">Claude Code\u003c/span>\u003cspan class=\"spacer\">\u003c/span>\u003cbutton class=\"link-btn\" data-testid=\"agent-undo-turn\" data-turn=\"turn-1\">\u000116\u0002Undo\u003cspan class=\"undo-word\"> turn\u003c/span>\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"composer\" data-testid=\"composer\">\u003cdiv class=\"composer-ctx\" data-testid=\"agent-context\">\u00010\u0002\u003cspan class=\"ellipsis\">Playhead 00:00:00:00\u003c/span>\u003c/div>\u003cdiv class=\"composer-box\">\u003cdiv class=\"composer-hl\">​\u003c/div>\u003ctextarea id=\"agent-input\" data-testid=\"agent-input\" rows=\"3\" placeholder=\"Ask for an edit; @ to point at media or a clip, / for a skill or a template\">\u003c/textarea>\u003c/div>\u003cdiv class=\"composer-foot\">\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-attach\">\u00011\u0002\u003c/button>\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-at\">\u00012\u0002\u003c/button>\u003cdiv class=\"composer-choices\">\u003cbutton class=\"choice provider\" data-testid=\"agent-provider\" data-provider=\"claude\">\u003cspan class=\"provider-mark sm\" data-testid=\"provider-mark\" data-provider=\"claude\">\u00013\u0002\u003c/span>\u003c/button>\u003cbutton class=\"choice model\" data-testid=\"agent-model\" data-model=\"\">\u003cspan class=\"provider-wordmark\" data-testid=\"provider-wordmark\" data-provider=\"claude\">Claude\u003c/span>\u003cspan class=\"choice-label\">Default\u003c/span>\u00014\u0002\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-effort\" data-effort=\"\">\u00015\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-access\" data-access=\"auto\">\u00016\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003c/div>\u003cspan class=\"composer-spacer\">\u003c/span>\u003cbutton class=\"send\" data-testid=\"agent-send\" disabled=\"\">\u00017\u0002\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/aside>\u003csection class=\"bottom\">\u003cdiv class=\"bottom-toolbar\">\u003cdiv class=\"seg\" data-testid=\"bottom-tabs\">\u003cbutton class=\"active\" data-testid=\"bottom-timeline\">Timeline\u003c/button>\u003cbutton data-testid=\"bottom-mixer\">Mixer\u003c/button>\u003c/div>\u003cspan class=\"muted small\">5 tracks\u003c/span>\u003cdiv class=\"toolbar-spacer\">\u003c/div>\u003cdiv class=\"tool-picker\" data-testid=\"tool-picker\">\u003cbutton class=\"icon-btn on\" data-testid=\"tool-select\">\u00010\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"tool-blade\">\u000150\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"tool-hand\">\u000142\u0002\u003c/button>\u003c/div>\u003cdiv class=\"toolbar-sep\">\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"split\">\u000151\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"delete-item\" disabled=\"\">\u000152\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"snap-toggle\">\u000153\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"skim-toggle\">\u000154\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"link-overlays\">\u000155\u0002\u003c/button>\u003cdiv class=\"toolbar-sep\">\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-out\">\u000156\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-in\">\u000157\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-fit\">\u000158\u0002\u003c/button>\u003c/div>\u003cdiv class=\"bottom-body\">\u003cdiv class=\"tl\" data-testid=\"timeline\" data-ppf=\"1.0095238095238095\" data-zoom=\"1\" style=\"--tl-view-h: 231px;\">\u003cdiv class=\"tl-heads\" data-testid=\"track-heads\">\u003cdiv class=\"tl-corner\">\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-text\" data-kind=\"text\">\u003cdiv class=\"name\">\u000123\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Text\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-gfx\" data-kind=\"video\">\u003cdiv class=\"name\">\u000161\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Graphics\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head main\" data-testid=\"track\" data-id=\"t-main\" data-kind=\"video\" data-main=\"true\">\u003cdiv class=\"name\">\u000121\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Main\u003c/span>\u003cspan class=\"badge\">\u000153\u0002\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-music\" data-kind=\"audio\">\u003cdiv class=\"name\">\u000135\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Audio\u003c/span>\u003c/div>\u003cdiv class=\"track-volume\">\u003cbutton class=\"icon-btn\" data-testid=\"track-volume\">\u000162\u0002\u003c/button>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-mute\">\u000163\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-sfx\" data-kind=\"audio\">\u003cdiv class=\"name\">\u000135\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Sound effects\u003c/span>\u003c/div>\u003cdiv class=\"track-volume\">\u003cbutton class=\"icon-btn\" data-testid=\"track-volume\">\u000162\u0002\u003c/button>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-mute\">\u000163\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lanes tool-select\" data-testid=\"lanes\" data-tool=\"select\" style=\"width: 1896px;\">\u003cdiv class=\"tl-ruler\" data-testid=\"ruler\">\u003cdiv class=\"tick\" style=\"left: 0px;\">\u003cspan>0s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 121.143px;\">\u003cspan>2s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 242.286px;\">\u003cspan>4s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 363.429px;\">\u003cspan>6s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 484.571px;\">\u003cspan>8s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 605.714px;\">\u003cspan>10s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 726.857px;\">\u003cspan>12s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 848px;\">\u003cspan>14s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 969.143px;\">\u003cspan>16s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1090.29px;\">\u003cspan>18s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1211.43px;\">\u003cspan>20s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1332.57px;\">\u003cspan>22s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1453.71px;\">\u003cspan>24s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1574.86px;\">\u003cspan>26s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1696px;\">\u003cspan>28s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1817.14px;\">\u003cspan>30s\u003c/span>\u003c/div>\u003cdiv class=\"playhead\" data-testid=\"playhead\" data-frame=\"0\" style=\"left: 0px;\">\u003cdiv class=\"playhead-handle\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-text\">\u003cdiv class=\"clip text\" data-type=\"text\" data-testid=\"timeline-item\" data-item=\"title\" data-start=\"30\" data-duration=\"330\" style=\"left: 30.2857px; width: 331.143px;\">\u003cspan class=\"nm\">Edit videos by talking\u003c/span>\u003cspan class=\"meta\">5.5s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-gfx\">\u003cdiv class=\"clip graphic\" data-type=\"graphic\" data-testid=\"timeline-item\" data-item=\"end\" data-start=\"1320\" data-duration=\"360\" style=\"left: 1332.57px; width: 361.429px;\">\u003cspan class=\"nm\">End card\u003c/span>\u003cspan class=\"meta\">6.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane main\" data-testid=\"lane\" data-track=\"t-main\">\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c1\" data-start=\"0\" data-duration=\"420\" style=\"left: 0px; width: 422px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0055\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0055\u003c/span>\u003cspan class=\"meta\">7.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c2\" data-start=\"420\" data-duration=\"420\" style=\"left: 424px; width: 422px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0063\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0063\u003c/span>\u003cspan class=\"meta\">7.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c3\" data-start=\"840\" data-duration=\"360\" style=\"left: 848px; width: 361.429px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0068\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0068\u003c/span>\u003cspan class=\"meta\">6.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c4\" data-start=\"1200\" data-duration=\"480\" style=\"left: 1211.43px; width: 482.571px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0050\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0050\u003c/span>\u003cspan class=\"meta\">8.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c2\" data-frame=\"420\" style=\"left: 424px;\">\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c3\" data-frame=\"840\" style=\"left: 848px;\">\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c4\" data-frame=\"1200\" style=\"left: 1211.43px;\">\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-music\">\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"music\" data-start=\"0\" data-duration=\"1680\" style=\"left: 0px; width: 1694px;\">\u003cimg class=\"wave\" style=\";width:1692px;height:29px\" src=\"\u000164\u0002\">\u000165\u0002\u003cspan class=\"nm\">electro-dreams.mp3\u003c/span>\u003cspan class=\"meta\">28.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-sfx\">\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx1\" data-start=\"150\" data-duration=\"174\" style=\"left: 151.429px; width: 173.657px;\">\u003cimg class=\"wave\" style=\";width:171.65625px;height:29px\" src=\"\u000166\u0002\">\u000167\u0002\u003cspan class=\"nm\">Keyboard typing\u003c/span>\u003cspan class=\"meta\">2.9s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx2\" data-start=\"360\" data-duration=\"18\" style=\"left: 363.429px; width: 16.1714px;\">\u003cimg class=\"wave\" style=\";width:14.15625px;height:29px\" src=\"\u000168\u0002\">\u000169\u0002\u003cspan class=\"nm\">Mouse click\u003c/span>\u003cspan class=\"meta\">0.3s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 3.17143px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 3.17143px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx3\" data-start=\"696\" data-duration=\"48\" style=\"left: 702.629px; width: 46.4571px;\">\u003cimg class=\"wave\" style=\";width:44.453125px;height:29px\" src=\"\u000170\u0002\">\u000171\u0002\u003cspan class=\"nm\">Airy whoosh\u003c/span>\u003cspan class=\"meta\">0.8s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx4\" data-start=\"1230\" data-duration=\"18\" style=\"left: 1241.71px; width: 16.1714px;\">\u003cimg class=\"wave\" style=\";width:14.15625px;height:29px\" src=\"\u000168\u0002\">\u000169\u0002\u003cspan class=\"nm\">Mouse click\u003c/span>\u003cspan class=\"meta\">0.3s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 3.17143px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 3.17143px;\">\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/section>\u003cdiv class=\"resizer v resizer-lib\" data-testid=\"resize-lib\">\u003c/div>\u003cdiv class=\"resizer v resizer-side\" data-testid=\"resize-side\">\u003c/div>\u003cdiv class=\"resizer h resizer-bottom\" data-testid=\"resize-bottom\">\u003c/div>\u003c/div>","width":1920,"height":900,"ancestors":[],"backdrop":"rgb(10, 10, 10)","radius":"0px"},"topbar":{"html":"\u003cheader class=\"topbar\">\u003cdiv class=\"wordmark\" data-testid=\"wordmark\" data-busy=\"false\">\u003cspan class=\"brand wordmark-name\">\u000110\u0002\u003cspan class=\"wordmark-text\" data-testid=\"wordmark-text\">biliup\u003c/span>\u003c/span>\u003cspan class=\"wordmark-slash\">\u003c/span>\u003cdiv class=\"project-title\" data-testid=\"project-title\">Launch film\u003c/div>\u003c/div>\u003cdiv class=\"topbar-spacer\">\u003c/div>\u003cdiv class=\"layout-toggles\">\u003cbutton class=\"icon-btn\" data-testid=\"toggle-lib\">\u000111\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-bottom\">\u000112\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-side\">\u000113\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"toggle-focus\">\u000114\u0002\u003c/button>\u003c/div>\u003cdiv class=\"saved\" data-testid=\"saved\">\u000115\u0002Saved\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"undo\">\u000116\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"redo\" disabled=\"\">\u000117\u0002\u003c/button>\u003cbutton class=\"btn export-btn\" data-testid=\"export-button\" data-running=\"false\">\u000118\u0002Export\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"open-settings\">\u000119\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"close-project\">\u000120\u0002\u003c/button>\u003c/header>","width":1920,"height":40,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}],"backdrop":"rgb(10, 10, 10)","radius":"0px"},"agent":{"html":"\u003caside class=\"side\">\u003cdiv class=\"tabs\">\u003cbutton data-testid=\"tab-inspector\">Inspector\u003c/button>\u003cbutton class=\"active\" data-testid=\"tab-agent\">Agent\u003c/button>\u003cbutton data-testid=\"tab-history\">History\u003c/button>\u003c/div>\u003cdiv class=\"side-body\">\u003cdiv class=\"agent\" data-testid=\"agent-panel\">\u003cdiv class=\"plan-strip agent-strip\" data-testid=\"agent-strip\">\u003cbutton class=\"icon-btn agent-new-chat\" data-testid=\"agent-new-chat\">\u000144\u0002\u003c/button>\u003cspan class=\"spacer\">\u003c/span>\u003cdiv class=\"plan-chip-wrap\" data-testid=\"agent-plan\">\u003cbutton class=\"plan-chip-btn\" data-testid=\"plan-chip\">\u000145\u0002Plan\u003cspan class=\"plan-state passed\">Passed\u003c/span>\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"agent-thread\" data-testid=\"agent-thread\" data-ui-scroll=\"38\">\u003cdiv class=\"turn-wrap\" data-testid=\"agent-turn\" data-turn=\"turn-1\" data-status=\"done\" data-undo=\"available\">\u003cdiv class=\"msg-user\" data-testid=\"agent-prompt\">Make a 28 s launch film for Biliup on the Electro Dreams beat, the drone clips as the film being edited.\u003c/div>\u003cdiv class=\"turn done\" data-testid=\"agent-stream\">\u003cdiv class=\"worklog\">\u003cdiv class=\"step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"graphic_write\">\u000115\u0002\u003cspan class=\"grow\">Wrote graphic film.html\u003c/span>\u003c/div>\u003c/div>\u003cdiv class=\"edit-row\" data-testid=\"agent-edit-row\">\u003cdiv class=\"step edit-step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"edit\">\u000146\u0002\u003cspan class=\"grow\">Placed the clips on Main\u003c/span>\u000147\u0002\u003c/div>\u003c/div>\u003cdiv class=\"card-item open\" data-testid=\"agent-card\" data-type=\"review\" data-passed=\"true\">\u003cbutton class=\"card-head\">\u00015\u0002\u003cspan class=\"grow\">Review, round 2\u003c/span>\u003cspan class=\"card-outcome approved\">Passed\u003c/span>\u000148\u0002\u003c/button>\u003cdiv class=\"review-by small muted\" data-testid=\"agent-review-by\" data-by=\"critic\">Independent critic\u003c/div>\u003cdiv class=\"scores\" data-testid=\"agent-review-scores\">\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"hook\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Hook in 2 s\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"readability\" data-value=\"8\" data-applies=\"required\">\u003cspan class=\"score-name\">Phone readability\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"motion\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Motion\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"variety\" data-value=\"8\" data-applies=\"optional\">\u003cspan class=\"score-name\">Variety\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"sound\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Sound sync\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"brand\" data-value=\"9\" data-applies=\"optional\">\u003cspan class=\"score-name\">Brand\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"reply\" data-testid=\"agent-reply\">\u003cp>The film is built on the Electro Dreams beat: it opens on the claim, types your prompt, builds the timeline, floods the preview on the drop and ends on the export.\u003c/p>\u003cbutton type=\"button\" class=\"icon-btn copy-btn reply-copy\" data-testid=\"agent-copy\">\u000149\u0002\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"turn-foot\">\u003cdiv class=\"turn-edits\" data-testid=\"agent-edits\">\u003cbutton class=\"turn-edits-label\" data-testid=\"agent-edits-toggle\">\u000146\u00022 edits\u003c/button>\u003cdiv class=\"sr-only\">\u003cdiv class=\"turn-edits-list\">\u003cdiv class=\"turn-edit\" data-testid=\"agent-edit\">\u003cdiv class=\"turn-edit-summary\">Placed the clips on Main\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0055\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0063\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0068\u003c/div>\u003cdiv class=\"turn-edit-line\">Added DJI 0050\u003c/div>\u003c/div>\u003cdiv class=\"turn-edit\" data-testid=\"agent-edit\">\u003cdiv class=\"turn-edit-summary\">Added sound effects\u003c/div>\u003cdiv class=\"turn-edit-line\">Added Keyboard typing\u003c/div>\u003cdiv class=\"turn-edit-line\">Added Mouse click\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003cspan class=\"turn-status\">Claude Code\u003c/span>\u003cspan class=\"spacer\">\u003c/span>\u003cbutton class=\"link-btn\" data-testid=\"agent-undo-turn\" data-turn=\"turn-1\">\u000116\u0002Undo\u003cspan class=\"undo-word\"> turn\u003c/span>\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"composer\" data-testid=\"composer\">\u003cdiv class=\"composer-ctx\" data-testid=\"agent-context\">\u00010\u0002\u003cspan class=\"ellipsis\">Playhead 00:00:00:00\u003c/span>\u003c/div>\u003cdiv class=\"composer-box\">\u003cdiv class=\"composer-hl\">​\u003c/div>\u003ctextarea id=\"agent-input\" data-testid=\"agent-input\" rows=\"3\" placeholder=\"Ask for an edit; @ to point at media or a clip, / for a skill or a template\">\u003c/textarea>\u003c/div>\u003cdiv class=\"composer-foot\">\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-attach\">\u00011\u0002\u003c/button>\u003cbutton class=\"icon-btn composer-at\" data-testid=\"agent-at\">\u00012\u0002\u003c/button>\u003cdiv class=\"composer-choices\">\u003cbutton class=\"choice provider\" data-testid=\"agent-provider\" data-provider=\"claude\">\u003cspan class=\"provider-mark sm\" data-testid=\"provider-mark\" data-provider=\"claude\">\u00013\u0002\u003c/span>\u003c/button>\u003cbutton class=\"choice model\" data-testid=\"agent-model\" data-model=\"\">\u003cspan class=\"provider-wordmark\" data-testid=\"provider-wordmark\" data-provider=\"claude\">Claude\u003c/span>\u003cspan class=\"choice-label\">Default\u003c/span>\u00014\u0002\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-effort\" data-effort=\"\">\u00015\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003cbutton class=\"choice\" data-testid=\"agent-access\" data-access=\"auto\">\u00016\u0002\u003cspan class=\"choice-label\">Auto\u003c/span>\u003c/button>\u003c/div>\u003cspan class=\"composer-spacer\">\u003c/span>\u003cbutton class=\"send\" data-testid=\"agent-send\" disabled=\"\">\u00017\u0002\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/aside>","width":440,"height":590,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}]},"message":{"html":"\u003cdiv class=\"msg-user\" data-testid=\"agent-prompt\">Make a 28 s launch film for Biliup on the Electro Dreams beat, the drone clips as the film being edited.\u003c/div>","width":365,"height":50,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"aside","cls":"side","attrs":{},"vars":""},{"tag":"div","cls":"side-body","attrs":{},"vars":""},{"tag":"div","cls":"agent","attrs":{},"vars":""},{"tag":"div","cls":"agent-thread","attrs":{},"vars":""},{"tag":"div","cls":"turn-wrap","attrs":{"data-turn":"turn-1","data-status":"done","data-undo":"available"},"vars":""}]},"turn":{"html":"\u003cdiv class=\"turn done\" data-testid=\"agent-stream\">\u003cdiv class=\"worklog\">\u003cdiv class=\"step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"graphic_write\">\u000115\u0002\u003cspan class=\"grow\">Wrote graphic film.html\u003c/span>\u003c/div>\u003c/div>\u003cdiv class=\"edit-row\" data-testid=\"agent-edit-row\">\u003cdiv class=\"step edit-step\" data-testid=\"agent-step\" data-status=\"done\" data-tool=\"edit\">\u000146\u0002\u003cspan class=\"grow\">Placed the clips on Main\u003c/span>\u000147\u0002\u003c/div>\u003c/div>\u003cdiv class=\"card-item open\" data-testid=\"agent-card\" data-type=\"review\" data-passed=\"true\">\u003cbutton class=\"card-head\">\u00015\u0002\u003cspan class=\"grow\">Review, round 2\u003c/span>\u003cspan class=\"card-outcome approved\">Passed\u003c/span>\u000148\u0002\u003c/button>\u003cdiv class=\"review-by small muted\" data-testid=\"agent-review-by\" data-by=\"critic\">Independent critic\u003c/div>\u003cdiv class=\"scores\" data-testid=\"agent-review-scores\">\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"hook\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Hook in 2 s\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"readability\" data-value=\"8\" data-applies=\"required\">\u003cspan class=\"score-name\">Phone readability\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"motion\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Motion\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"variety\" data-value=\"8\" data-applies=\"optional\">\u003cspan class=\"score-name\">Variety\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"sound\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Sound sync\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"brand\" data-value=\"9\" data-applies=\"optional\">\u003cspan class=\"score-name\">Brand\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"reply\" data-testid=\"agent-reply\">\u003cp>The film is built on the Electro Dreams beat: it opens on the claim, types your prompt, builds the timeline, floods the preview on the drop and ends on the export.\u003c/p>\u003cbutton type=\"button\" class=\"icon-btn copy-btn reply-copy\" data-testid=\"agent-copy\">\u000149\u0002\u003c/button>\u003c/div>\u003c/div>","width":415,"height":297,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"aside","cls":"side","attrs":{},"vars":""},{"tag":"div","cls":"side-body","attrs":{},"vars":""},{"tag":"div","cls":"agent","attrs":{},"vars":""},{"tag":"div","cls":"agent-thread","attrs":{},"vars":""},{"tag":"div","cls":"turn-wrap","attrs":{"data-turn":"turn-1","data-status":"done","data-undo":"available"},"vars":""}],"backdrop":"rgb(17, 17, 17)","radius":"0px"},"review":{"html":"\u003cdiv class=\"card-item open\" data-testid=\"agent-card\" data-type=\"review\" data-passed=\"true\">\u003cbutton class=\"card-head\">\u00015\u0002\u003cspan class=\"grow\">Review, round 2\u003c/span>\u003cspan class=\"card-outcome approved\">Passed\u003c/span>\u000148\u0002\u003c/button>\u003cdiv class=\"review-by small muted\" data-testid=\"agent-review-by\" data-by=\"critic\">Independent critic\u003c/div>\u003cdiv class=\"scores\" data-testid=\"agent-review-scores\">\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"hook\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Hook in 2 s\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"readability\" data-value=\"8\" data-applies=\"required\">\u003cspan class=\"score-name\">Phone readability\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"motion\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Motion\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"variety\" data-value=\"8\" data-applies=\"optional\">\u003cspan class=\"score-name\">Variety\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 80%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">8\u003c/span>\u003c/div>\u003cdiv class=\"score ok required\" data-testid=\"agent-review-score\" data-score=\"sound\" data-value=\"9\" data-applies=\"required\">\u003cspan class=\"score-name\">Sound sync\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003cdiv class=\"score ok optional\" data-testid=\"agent-review-score\" data-score=\"brand\" data-value=\"9\" data-applies=\"optional\">\u003cspan class=\"score-name\">Brand\u003cspan class=\"muted\"> (optional)\u003c/span>\u003c/span>\u003cspan class=\"score-bar\">\u003cspan style=\"width: 90%;\">\u003c/span>\u003c/span>\u003cspan class=\"score-value\">9\u003c/span>\u003c/div>\u003c/div>\u003c/div>","width":415,"height":172,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"aside","cls":"side","attrs":{},"vars":""},{"tag":"div","cls":"side-body","attrs":{},"vars":""},{"tag":"div","cls":"agent","attrs":{},"vars":""},{"tag":"div","cls":"agent-thread","attrs":{},"vars":""},{"tag":"div","cls":"turn-wrap","attrs":{"data-turn":"turn-1","data-status":"done","data-undo":"available"},"vars":""},{"tag":"div","cls":"turn done","attrs":{},"vars":""}]},"timeline":{"html":"\u003csection class=\"bottom\">\u003cdiv class=\"bottom-toolbar\">\u003cdiv class=\"seg\" data-testid=\"bottom-tabs\">\u003cbutton class=\"active\" data-testid=\"bottom-timeline\">Timeline\u003c/button>\u003cbutton data-testid=\"bottom-mixer\">Mixer\u003c/button>\u003c/div>\u003cspan class=\"muted small\">5 tracks\u003c/span>\u003cdiv class=\"toolbar-spacer\">\u003c/div>\u003cdiv class=\"tool-picker\" data-testid=\"tool-picker\">\u003cbutton class=\"icon-btn on\" data-testid=\"tool-select\">\u00010\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"tool-blade\">\u000150\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"tool-hand\">\u000142\u0002\u003c/button>\u003c/div>\u003cdiv class=\"toolbar-sep\">\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"split\">\u000151\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"delete-item\" disabled=\"\">\u000152\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"snap-toggle\">\u000153\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"skim-toggle\">\u000154\u0002\u003c/button>\u003cbutton class=\"icon-btn on\" data-testid=\"link-overlays\">\u000155\u0002\u003c/button>\u003cdiv class=\"toolbar-sep\">\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-out\">\u000156\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-in\">\u000157\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"zoom-fit\">\u000158\u0002\u003c/button>\u003c/div>\u003cdiv class=\"bottom-body\">\u003cdiv class=\"tl\" data-testid=\"timeline\" data-ppf=\"1.0095238095238095\" data-zoom=\"1\" style=\"--tl-view-h: 231px;\">\u003cdiv class=\"tl-heads\" data-testid=\"track-heads\">\u003cdiv class=\"tl-corner\">\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-text\" data-kind=\"text\">\u003cdiv class=\"name\">\u000123\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Text\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-gfx\" data-kind=\"video\">\u003cdiv class=\"name\">\u000161\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Graphics\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head main\" data-testid=\"track\" data-id=\"t-main\" data-kind=\"video\" data-main=\"true\">\u003cdiv class=\"name\">\u000121\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Main\u003c/span>\u003cspan class=\"badge\">\u000153\u0002\u003c/span>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-hide\">\u000159\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-music\" data-kind=\"audio\">\u003cdiv class=\"name\">\u000135\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Audio\u003c/span>\u003c/div>\u003cdiv class=\"track-volume\">\u003cbutton class=\"icon-btn\" data-testid=\"track-volume\">\u000162\u0002\u003c/button>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-mute\">\u000163\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003cdiv class=\"tl-head\" data-testid=\"track\" data-id=\"t-sfx\" data-kind=\"audio\">\u003cdiv class=\"name\">\u000135\u0002\u003cspan class=\"ellipsis\" data-testid=\"track-name\">Sound effects\u003c/span>\u003c/div>\u003cdiv class=\"track-volume\">\u003cbutton class=\"icon-btn\" data-testid=\"track-volume\">\u000162\u0002\u003c/button>\u003c/div>\u003cbutton class=\"icon-btn\" data-testid=\"track-mute\">\u000163\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"track-lock\">\u000160\u0002\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lanes tool-select\" data-testid=\"lanes\" data-tool=\"select\" style=\"width: 1896px;\">\u003cdiv class=\"tl-ruler\" data-testid=\"ruler\">\u003cdiv class=\"tick\" style=\"left: 0px;\">\u003cspan>0s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 121.143px;\">\u003cspan>2s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 242.286px;\">\u003cspan>4s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 363.429px;\">\u003cspan>6s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 484.571px;\">\u003cspan>8s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 605.714px;\">\u003cspan>10s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 726.857px;\">\u003cspan>12s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 848px;\">\u003cspan>14s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 969.143px;\">\u003cspan>16s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1090.29px;\">\u003cspan>18s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1211.43px;\">\u003cspan>20s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1332.57px;\">\u003cspan>22s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1453.71px;\">\u003cspan>24s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1574.86px;\">\u003cspan>26s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1696px;\">\u003cspan>28s\u003c/span>\u003c/div>\u003cdiv class=\"tick\" style=\"left: 1817.14px;\">\u003cspan>30s\u003c/span>\u003c/div>\u003cdiv class=\"playhead\" data-testid=\"playhead\" data-frame=\"0\" style=\"left: 0px;\">\u003cdiv class=\"playhead-handle\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-text\">\u003cdiv class=\"clip text\" data-type=\"text\" data-testid=\"timeline-item\" data-item=\"title\" data-start=\"30\" data-duration=\"330\" style=\"left: 30.2857px; width: 331.143px;\">\u003cspan class=\"nm\">Edit videos by talking\u003c/span>\u003cspan class=\"meta\">5.5s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-gfx\">\u003cdiv class=\"clip graphic\" data-type=\"graphic\" data-testid=\"timeline-item\" data-item=\"end\" data-start=\"1320\" data-duration=\"360\" style=\"left: 1332.57px; width: 361.429px;\">\u003cspan class=\"nm\">End card\u003c/span>\u003cspan class=\"meta\">6.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane main\" data-testid=\"lane\" data-track=\"t-main\">\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c1\" data-start=\"0\" data-duration=\"420\" style=\"left: 0px; width: 422px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0055\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0055\u003c/span>\u003cspan class=\"meta\">7.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c2\" data-start=\"420\" data-duration=\"420\" style=\"left: 424px; width: 422px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0063\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0063\u003c/span>\u003cspan class=\"meta\">7.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c3\" data-start=\"840\" data-duration=\"360\" style=\"left: 848px; width: 361.429px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0068\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0068\u003c/span>\u003cspan class=\"meta\">6.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"clip video\" data-type=\"clip\" data-testid=\"timeline-item\" data-item=\"c4\" data-start=\"1200\" data-duration=\"480\" style=\"left: 1211.43px; width: 482.571px;\">\u003cdiv class=\"strip filmstrip\" data-testid=\"filmstrip\" data-media=\"m-dji-0050\" data-ready=\"false\">\u003c/div>\u003cspan class=\"nm\">DJI 0050\u003c/span>\u003cspan class=\"meta\">8.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c2\" data-frame=\"420\" style=\"left: 424px;\">\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c3\" data-frame=\"840\" style=\"left: 848px;\">\u003c/div>\u003cdiv class=\"cut-point\" data-testid=\"cut-point\" data-item=\"c4\" data-frame=\"1200\" style=\"left: 1211.43px;\">\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-music\">\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"music\" data-start=\"0\" data-duration=\"1680\" style=\"left: 0px; width: 1694px;\">\u003cimg class=\"wave\" style=\";width:1692px;height:29px\" src=\"\u000164\u0002\">\u000165\u0002\u003cspan class=\"nm\">electro-dreams.mp3\u003c/span>\u003cspan class=\"meta\">28.0s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003c/div>\u003cdiv class=\"tl-lane\" data-testid=\"lane\" data-track=\"t-sfx\">\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx1\" data-start=\"150\" data-duration=\"174\" style=\"left: 151.429px; width: 173.657px;\">\u003cimg class=\"wave\" style=\";width:171.65625px;height:29px\" src=\"\u000166\u0002\">\u000167\u0002\u003cspan class=\"nm\">Keyboard typing\u003c/span>\u003cspan class=\"meta\">2.9s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx2\" data-start=\"360\" data-duration=\"18\" style=\"left: 363.429px; width: 16.1714px;\">\u003cimg class=\"wave\" style=\";width:14.15625px;height:29px\" src=\"\u000168\u0002\">\u000169\u0002\u003cspan class=\"nm\">Mouse click\u003c/span>\u003cspan class=\"meta\">0.3s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 3.17143px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 3.17143px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx3\" data-start=\"696\" data-duration=\"48\" style=\"left: 702.629px; width: 46.4571px;\">\u003cimg class=\"wave\" style=\";width:44.453125px;height:29px\" src=\"\u000170\u0002\">\u000171\u0002\u003cspan class=\"nm\">Airy whoosh\u003c/span>\u003cspan class=\"meta\">0.8s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 13px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 13px;\">\u003c/div>\u003c/div>\u003cdiv class=\"clip audio\" data-type=\"audio\" data-testid=\"timeline-item\" data-item=\"sx4\" data-start=\"1230\" data-duration=\"18\" style=\"left: 1241.71px; width: 16.1714px;\">\u003cimg class=\"wave\" style=\";width:14.15625px;height:29px\" src=\"\u000168\u0002\">\u000169\u0002\u003cspan class=\"nm\">Mouse click\u003c/span>\u003cspan class=\"meta\">0.3s\u003c/span>\u003cdiv class=\"edge start\" data-edge=\"start\" data-testid=\"trim-start\">\u003c/div>\u003cdiv class=\"edge end\" data-edge=\"end\" data-testid=\"trim-end\">\u003c/div>\u003cdiv class=\"fade-handle in\" data-fade=\"in\" data-testid=\"fade-in-handle\" style=\"left: 3.17143px;\">\u003c/div>\u003cdiv class=\"fade-handle out\" data-fade=\"out\" data-testid=\"fade-out-handle\" style=\"right: 3.17143px;\">\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/section>","width":1920,"height":270,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}],"pxPerSecond":60.285714285714285,"laneLeft":184},"preview":{"html":"\u003cmain class=\"stage\" data-testid=\"stage\">\u003cdiv class=\"stage-canvas\" data-testid=\"stage-canvas\" data-zoom=\"fit\">\u003cdiv class=\"frame\" data-testid=\"preview-frame\" data-frame=\"0\" data-playing=\"false\" style=\"width: 411px; height: 514px; background: rgb(0, 0, 0);\">\u003c/div>\u003c/div>\u003cdiv class=\"transport\">\u003cspan class=\"timecode\" data-testid=\"timecode\">00:00:00:00\u003cspan class=\"tc-end\"> / 00:00:28:00\u003c/span>\u003c/span>\u003cdiv class=\"transport-mid\">\u003cbutton class=\"icon-btn tp-jump\" data-testid=\"to-start\" disabled=\"\">\u000136\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-step\" data-testid=\"step-back\" disabled=\"\">\u000137\u0002\u003c/button>\u003cbutton class=\"icon-btn play\" data-testid=\"play\">\u000138\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-step\" data-testid=\"step-forward\">\u000139\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-jump\" data-testid=\"to-end\">\u000140\u0002\u003c/button>\u003c/div>\u003cspan class=\"muted small\" data-testid=\"canvas-info\">1080×1350, 60 fps\u003c/span>\u003cselect class=\"zoom-select\" data-testid=\"preview-zoom\">\u003coption value=\"fit\">Fit\u003c/option>\u003coption value=\"0.5\">50%\u003c/option>\u003coption value=\"1\">100%\u003c/option>\u003coption value=\"2\">200%\u003c/option>\u003c/select>\u003cbutton class=\"icon-btn tp-region\" data-testid=\"preview-region\">\u000141\u0002\u003c/button>\u003cbutton class=\"icon-btn tp-hand\" data-testid=\"preview-hand\">\u000142\u0002\u003c/button>\u003cbutton class=\"icon-btn\" data-testid=\"preview-fullscreen\">\u000143\u0002\u003c/button>\u003c/div>\u003c/main>","width":1152,"height":590,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}],"backdrop":"rgb(10, 10, 10)","radius":"0px"},"export":{"html":"\u003cdiv class=\"modal wiz\">\u003cdiv class=\"wiz-panel\">\u003cdiv class=\"export-head\">\u003ch1>Export video\u003c/h1>\u003cbutton class=\"icon-btn\" data-testid=\"export-close\">\u000120\u0002\u003c/button>\u003c/div>\u003cform data-testid=\"export-form\">\u003cdiv class=\"wiz-cards\">\u003cbutton type=\"button\" class=\"wiz-card\" data-testid=\"quality-draft\" data-active=\"false\" data-codec=\"h264\">Draft\u003cspan>720×900, 60 fps\u003c/span>\u003cspan data-testid=\"quality-codec-draft\">H.264, hardware\u003c/span>\u003cspan>Fast, for a quick look\u003c/span>\u003c/button>\u003cbutton type=\"button\" class=\"wiz-card active\" data-testid=\"quality-standard\" data-active=\"true\" data-codec=\"h264\">Standard\u003cspan>1080×1350, 60 fps\u003c/span>\u003cspan data-testid=\"quality-codec-standard\">H.264, hardware\u003c/span>\u003cspan>For sharing anywhere\u003c/span>\u003c/button>\u003cbutton type=\"button\" class=\"wiz-card\" data-testid=\"quality-high\" data-active=\"false\" data-codec=\"h264\">High\u003cspan>1080×1350, 60 fps\u003c/span>\u003cspan data-testid=\"quality-codec-high\">H.264, software\u003c/span>\u003cspan>Full size, the best copy\u003c/span>\u003c/button>\u003c/div>\u003cbutton type=\"button\" class=\"export-advanced-toggle\" data-testid=\"export-advanced\">\u000172\u0002Advanced\u003c/button>\u003cdiv class=\"wiz-list form-fields export-fields\">\u003clabel class=\"wiz-field\">\u003cspan>File\u003c/span>\u003cinput data-testid=\"export-name\" value=\"Launch film.mp4\">\u003c/label>\u003cdiv class=\"wiz-field\">\u003cspan>Folder\u003c/span>\u003cb class=\"ellipsis\" data-testid=\"export-folder\">~/Movies\u003c/b>\u003cbutton type=\"button\" class=\"btn ghost\" data-testid=\"export-change\">Change…\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"group-label export-label\">Range\u003c/div>\u003cdiv class=\"seg export-range\" data-testid=\"export-range\">\u003cbutton type=\"button\" class=\"active\" data-testid=\"export-range-all\">Whole timeline\u003c/button>\u003cbutton type=\"button\" data-testid=\"export-range-marks\" disabled=\"\">In to Out\u003c/button>\u003cbutton type=\"button\" data-testid=\"export-range-selection\" disabled=\"\">Selected clips\u003c/button>\u003cbutton type=\"button\" data-testid=\"export-range-custom\">Custom\u003c/button>\u003c/div>\u003cp class=\"wiz-meta\" data-testid=\"export-range-text\" data-from=\"0\" data-to=\"1680\">00:00:00:00 to 00:00:28:00, 28.0 s\u003c/p>\u003cdiv class=\"group-label export-label\">Loudness\u003c/div>\u003cdiv class=\"wiz-cards\">\u003cbutton type=\"button\" class=\"wiz-card active\" data-testid=\"loudness-normal\" data-active=\"true\">Normal\u003cspan>−14 LUFS\u003c/span>\u003c/button>\u003cbutton type=\"button\" class=\"wiz-card\" data-testid=\"loudness-quiet\" data-active=\"false\">Quieter\u003cspan>−16 LUFS\u003c/span>\u003c/button>\u003cbutton type=\"button\" class=\"wiz-card\" data-testid=\"loudness-broadcast\" data-active=\"false\">Broadcast\u003cspan>−23 LUFS\u003c/span>\u003c/button>\u003cbutton type=\"button\" class=\"wiz-card\" data-testid=\"loudness-off\" data-active=\"false\">As mixed\u003cspan>No change\u003c/span>\u003c/button>\u003c/div>\u003cp class=\"wiz-meta\" data-testid=\"loudness-hint\">Evens out the volume so it plays as loud as other videos on YouTube, Instagram and TikTok. Peaks stay under −1 dBTP (the master limiter).\u003c/p>\u003cdiv class=\"group-label export-label\">Motion\u003c/div>\u003cdiv class=\"export-subs\" data-testid=\"export-motion\">\u003clabel class=\"export-sub-row\">\u003cinput data-testid=\"export-motion-blur\" type=\"checkbox\" checked=\"\">\u003cspan>Motion blur\u003c/span>\u003cspan class=\"muted\">Graphics: 4 sub-frames a frame, a 90° shutter. The preview shows them sharp.\u003c/span>\u003c/label>\u003c/div>\u003cdiv class=\"wiz-foot\">\u003cp class=\"export-note\">Opens in Finder when it's done.\u003c/p>\u003cbutton type=\"button\" class=\"btn ghost\">Cancel\u003c/button>\u003cbutton type=\"submit\" class=\"btn primary\" data-testid=\"export-start\">Export\u003c/button>\u003c/div>\u003c/form>\u003c/div>\u003c/div>","width":640,"height":789,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"div","cls":"modal-layer","attrs":{},"vars":""}]},"exportProgress":{"html":"\u003cdiv class=\"modal wiz\">\u003cdiv class=\"wiz-panel\">\u003cdiv class=\"export-head\">\u003ch1>Export video\u003c/h1>\u003cbutton class=\"icon-btn\" data-testid=\"export-close\">\u000120\u0002\u003c/button>\u003c/div>\u003cdiv data-testid=\"export-job\" data-phase=\"rendering\" data-progress=\"0.42\" data-output=\"/Users/you/Movies/Launch film.mp4\">\u003cp class=\"wiz-lead\" data-testid=\"export-job-where\">\u003cb class=\"mono\">Launch film.mp4\u003c/b> in \u003cb data-testid=\"export-job-folder\">Movies\u003c/b>, 1080×1350, 60 fps, H.264, 28.0 s, motion blur\u003c/p>\u003cdiv class=\"export-status\" data-testid=\"export-status\">Rendering… 42%\u003cspan class=\"muted\">About 38 s left\u003c/span>\u003c/div>\u003cdiv class=\"export-bar\" data-testid=\"export-progress\">\u003cdiv style=\"width: 42%;\">\u003c/div>\u003c/div>\u003cdiv class=\"wiz-foot\">\u003cbutton class=\"btn ghost\">Hide\u003c/button>\u003cbutton class=\"btn\" data-testid=\"export-cancel\">Cancel export\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003c/div>","width":640,"height":217,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"},{"tag":"div","cls":"modal-layer","attrs":{},"vars":""}]},"inspector":{"html":"\u003caside class=\"side\">\u003cdiv class=\"tabs\">\u003cbutton class=\"active\" data-testid=\"tab-inspector\">Inspector\u003c/button>\u003cbutton data-testid=\"tab-agent\">Agent\u003c/button>\u003cbutton data-testid=\"tab-history\">History\u003c/button>\u003c/div>\u003cdiv class=\"side-body\">\u003cdiv data-testid=\"inspector-item\" data-item=\"c2\">\u003cdiv class=\"insp-title\">\u000121\u0002\u003cspan class=\"ellipsis grow\">DJI 0063\u003c/span>\u003cbutton class=\"small-btn insp-chat\" data-testid=\"insp-add-to-chat\">\u00012\u0002Add to chat\u003c/button>\u003c/div>\u003cdiv class=\"insp-sub\">Clip on Main\u003c/div>\u003csection class=\"psec\" data-testid=\"insp-sec-transform\" data-section=\"transform\" data-open=\"true\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-sec-transform-toggle\">\u000148\u0002Transform\u003c/button>\u003c/div>\u003cdiv class=\"psec-body\">\u003cdiv data-testid=\"clip-box-editor\">\u003cdiv class=\"group\">\u003cdiv class=\"group-label\">Position &amp; size\u003c/div>\u003cdiv class=\"field-row key-row\">\u003clabel class=\"field num-field\" data-scrub=\"insp-box-x\">\u003cspan class=\"k\">X\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-box-x\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003cbutton class=\"icon-btn key-btn none\" data-testid=\"insp-key\" data-prop=\"x\" data-state=\"none\" disabled=\"\">\u000173\u0002\u003c/button>\u003clabel class=\"field num-field\" data-scrub=\"insp-box-y\">\u003cspan class=\"k\">Y\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-box-y\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003cbutton class=\"icon-btn key-btn none\" data-testid=\"insp-key\" data-prop=\"y\" data-state=\"none\" disabled=\"\">\u000173\u0002\u003c/button>\u003c/div>\u003cdiv class=\"field-row\">\u003clabel class=\"field num-field\" data-scrub=\"insp-box-w\">\u003cspan class=\"k\">W\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-box-w\" inputmode=\"decimal\" type=\"text\" value=\"1080\">\u003c/label>\u003clabel class=\"field num-field\" data-scrub=\"insp-box-h\">\u003cspan class=\"k\">H\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-box-h\" inputmode=\"decimal\" type=\"text\" value=\"1350\">\u003c/label>\u003c/div>\u003cdiv class=\"seg fit\">\u003cbutton class=\"active\" data-testid=\"insp-fit\" data-fit=\"contain\">Contain\u003c/button>\u003cbutton data-testid=\"insp-fit\" data-fit=\"cover\">Cover\u003c/button>\u003cbutton data-testid=\"insp-fit\" data-fit=\"fill\">Fill\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"group\">\u003cdiv class=\"group-label\">Scale and opacity\u003c/div>\u003cdiv class=\"slider-row key-row\">\u003cinput class=\"slider\" data-testid=\"insp-scale\" min=\"10\" max=\"400\" type=\"range\" value=\"100\">\u003clabel class=\"field num-field\" data-scrub=\"insp-scale-num\">\u003cspan class=\"k\">Scale %\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-scale-num\" inputmode=\"decimal\" type=\"text\" value=\"100\">\u003c/label>\u003cbutton class=\"icon-btn key-btn none\" data-testid=\"insp-key\" data-prop=\"scale\" data-state=\"none\" disabled=\"\">\u000173\u0002\u003c/button>\u003c/div>\u003cdiv class=\"slider-row key-row\">\u003cinput class=\"slider\" data-testid=\"insp-opacity\" min=\"0\" max=\"100\" type=\"range\" value=\"100\">\u003clabel class=\"field num-field\" data-scrub=\"insp-opacity-num\">\u003cspan class=\"k\">Opacity %\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-opacity-num\" inputmode=\"decimal\" type=\"text\" value=\"100\">\u003c/label>\u003cbutton class=\"icon-btn key-btn none\" data-testid=\"insp-key\" data-prop=\"opacity\" data-state=\"none\" disabled=\"\">\u000173\u0002\u003c/button>\u003c/div>\u003cdiv class=\"field-row\">\u003clabel class=\"field num-field\" data-scrub=\"insp-radius\">\u003cspan class=\"k\">Radius\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-radius\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cdiv class=\"group\">\u003clabel class=\"check-row\">\u003cinput data-testid=\"insp-loop\" type=\"checkbox\">Loop the footage when the clip is longer\u003c/label>\u003clabel class=\"check-row\">\u003cinput data-testid=\"insp-hold\" type=\"checkbox\">Hold the last frame when the clip is longer\u003c/label>\u003cbutton class=\"btn\" data-testid=\"insp-box-reset\" disabled=\"\">\u000174\u0002Reset position and size\u003c/button>\u003c/div>\u003c/div>\u003c/div>\u003c/section>\u003cdiv data-testid=\"effects-editor\">\u003csection class=\"psec\" data-testid=\"insp-filter\" data-section=\"filter\" data-open=\"true\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-filter-toggle\">\u000148\u0002Filter\u003c/button>\u003cdiv class=\"psec-actions\">\u003cspan class=\"fx-power\">\u003cbutton class=\"ptoggle on\" data-testid=\"insp-filter-power\">\u003cspan>\u003c/span>\u003c/button>\u003c/span>\u003cbutton class=\"icon-btn\" data-testid=\"insp-filter-remove\">\u000120\u0002\u003c/button>\u003c/div>\u003c/div>\u003cdiv class=\"psec-body\">\u003cdiv class=\"prop\" data-testid=\"insp-filter-intensity\">\u003cspan class=\"prop-k\">Warm\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-filter-amount-slider\" min=\"0\" max=\"100\" step=\"1\" type=\"range\" value=\"100\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-filter-amount\" inputmode=\"decimal\" type=\"text\" value=\"100\">\u003cspan class=\"nfield-u\">%\u003c/span>\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003c/div>\u003c/section>\u003csection class=\"psec\" data-testid=\"insp-adjust\" data-section=\"adjust\" data-open=\"false\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-adjust-toggle\">\u000172\u0002Adjust\u003c/button>\u003c/div>\u003cdiv class=\"psec-body\" hidden=\"\">\u003cdiv class=\"prop\" data-testid=\"insp-adjust-exposure\">\u003cspan class=\"prop-k\">Exposure\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-exposure-value-slider\" min=\"-2\" max=\"2\" step=\"0.05\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-exposure-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003cspan class=\"nfield-u\">EV\u003c/span>\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-contrast\">\u003cspan class=\"prop-k\">Contrast\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-contrast-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-contrast-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-saturation\">\u003cspan class=\"prop-k\">Saturation\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-saturation-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-saturation-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-temperature\">\u003cspan class=\"prop-k\">Temperature\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-temperature-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-temperature-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-tint\">\u003cspan class=\"prop-k\">Tint\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-tint-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-tint-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-highlights\">\u003cspan class=\"prop-k\">Highlights\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-highlights-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-highlights-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-shadows\">\u003cspan class=\"prop-k\">Shadows\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-shadows-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-shadows-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-adjust-vibrance\">\u003cspan class=\"prop-k\">Vibrance\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-adjust-vibrance-value-slider\" min=\"-100\" max=\"100\" step=\"1\" type=\"range\" value=\"0\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-adjust-vibrance-value\" inputmode=\"decimal\" type=\"text\" value=\"0\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003c/div>\u003c/section>\u003csection class=\"psec\" data-testid=\"insp-effects\" data-section=\"effects\" data-open=\"true\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-effects-toggle\">\u000148\u0002Effects\u003c/button>\u003c/div>\u003cdiv class=\"psec-body\">\u003cdiv class=\"fx-entry\" data-testid=\"insp-effect\" data-module=\"effect.vignette\" data-off=\"false\">\u003cdiv class=\"fx-entry-head\">\u003cspan class=\"grow\">Vignette\u003c/span>\u003cbutton class=\"icon-btn\" data-testid=\"insp-effect-add-to-chat\">\u00012\u0002\u003c/button>\u003cspan class=\"fx-power\">\u003cbutton class=\"ptoggle on\" data-testid=\"insp-effect-power\">\u003cspan>\u003c/span>\u003c/button>\u003c/span>\u003cbutton class=\"icon-btn\" data-testid=\"insp-effect-remove\">\u000120\u0002\u003c/button>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-effect-amount\">\u003cspan class=\"prop-k\">Amount\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-effect-amount-value-slider\" min=\"0\" max=\"1\" step=\"0.01\" type=\"range\" value=\"0.5\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-effect-amount-value\" inputmode=\"decimal\" type=\"text\" value=\"0.5\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003cdiv class=\"prop\" data-testid=\"insp-effect-size\">\u003cspan class=\"prop-k\">Size\u003c/span>\u003cdiv class=\"prop-v\">\u003cdiv class=\"slider-num\">\u003cinput class=\"slider\" data-testid=\"insp-effect-size-value-slider\" min=\"0\" max=\"1\" step=\"0.01\" type=\"range\" value=\"0.5\">\u003clabel class=\"nfield\">\u003cinput data-testid=\"insp-effect-size-value\" inputmode=\"decimal\" type=\"text\" value=\"0.5\">\u003c/label>\u003c/div>\u003c/div>\u003cspan class=\"prop-reset-gap\">\u003c/span>\u003c/div>\u003c/div>\u003c/div>\u003c/section>\u003c/div>\u003csection class=\"psec\" data-testid=\"insp-sec-speed\" data-section=\"speed\" data-open=\"true\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-sec-speed-toggle\">\u000148\u0002Speed\u003c/button>\u003c/div>\u003cdiv class=\"psec-body\">\u003cdiv data-testid=\"speed-editor\">\u003cdiv class=\"group\">\u003cdiv class=\"speed-row\">\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-0.25\">0.25×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-0.5\">0.5×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-0.75\">0.75×\u003c/button>\u003cbutton class=\"small-btn active\" data-testid=\"insp-speed-1\">1×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-1.25\">1.25×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-1.5\">1.5×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-2\">2×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-3\">3×\u003c/button>\u003cbutton class=\"small-btn\" data-testid=\"insp-speed-4\">4×\u003c/button>\u003c/div>\u003cdiv class=\"field-row\">\u003clabel class=\"field num-field\" data-scrub=\"insp-speed-custom\">\u003cspan class=\"k\">×\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-speed-custom\" inputmode=\"decimal\" type=\"text\" value=\"1\">\u003c/label>\u003c/div>\u003clabel class=\"check-row\">\u003cinput data-testid=\"insp-keep-pitch\" type=\"checkbox\" checked=\"\">Keep pitch\u003c/label>\u003c/div>\u003c/div>\u003c/div>\u003c/section>\u003csection class=\"psec\" data-testid=\"insp-sec-advanced\" data-section=\"advanced\" data-open=\"false\">\u003cdiv class=\"psec-head\">\u003cbutton class=\"psec-title\" data-testid=\"insp-sec-advanced-toggle\">\u000172\u0002Advanced\u003c/button>\u003c/div>\u003cdiv class=\"psec-body\" hidden=\"\">\u003cdiv class=\"group\">\u003cdiv class=\"group-label\">Name\u003c/div>\u003clabel class=\"field num-field\">\u003cspan class=\"k\">Item\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-name\" placeholder=\"DJI 0063\" type=\"text\" value=\"\">\u003c/label>\u003clabel class=\"field num-field\">\u003cspan class=\"k\">Media\u003c/span>\u003cinput class=\"v\" data-testid=\"insp-media-name\" type=\"text\" value=\"DJI 0063\">\u003c/label>\u003c/div>\u003cdiv class=\"group\">\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Type\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-type\">Clip\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Track\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-track\">Main\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Start\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-start\">00:00:07:00\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Duration\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-duration\">7.00s\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Media\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-media\">DJI 0063\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Source in\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-in\">00:00:09:42\u003c/span>\u003c/div>\u003cdiv class=\"kv-row\">\u003cspan class=\"muted\">Source out\u003c/span>\u003cspan class=\"mono small\" data-testid=\"insp-out\">00:00:16:42\u003c/span>\u003c/div>\u003c/div>\u003c/div>\u003c/section>\u003c/div>\u003c/div>\u003c/aside>","width":440,"height":590,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}]},"effects":{"html":"\u003caside class=\"library\" data-testid=\"effects-library\" data-source=\"sample\">\u003cdiv class=\"lib-head\">Effects\u003c/div>\u003cdiv class=\"lib-section\">\u003cdiv class=\"music-filters\">\u003cdiv class=\"tr-search\">\u003cinput class=\"field input\" data-testid=\"effects-search\" placeholder=\"Search filters and effects\" value=\"\">\u003c/div>\u003cdiv class=\"muted small\" data-testid=\"effects-target\">On DJI 0063\u003c/div>\u003c/div>\u003cdiv class=\"lib-label\">Filters\u003c/div>\u003cdiv class=\"grid2 fx-grid\" data-testid=\"filters-grid\">\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"none\">\u003cdiv class=\"thumb fx-thumb fx-none\" data-testid=\"filter-thumb\">\u000175\u0002\u003c/div>\u003cdiv class=\"tile-name\">None\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"natural\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000176\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Natural\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile active\" data-testid=\"filter-tile\" data-look=\"warm\" data-active=\"true\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000177\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Warm\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"cool\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000178\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Cool\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"vintage\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000179\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Vintage\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"film\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000180\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Film\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"teal-orange\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000181\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Teal &amp; Orange\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"bw\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000182\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">B&amp;W\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"noir\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000183\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Noir\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"fade\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000184\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Fade\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"vivid\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000185\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Vivid\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"moody\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000186\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Moody\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"sepia\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000187\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Sepia\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"cinematic\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000188\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Cinematic\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"pastel\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000189\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Pastel\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"golden\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000190\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Golden hour\u003c/div>\u003c/div>\u003cdiv class=\"tile fx-tile\" data-testid=\"filter-tile\" data-look=\"arctic\">\u003cdiv class=\"thumb fx-thumb\" data-testid=\"filter-thumb\">\u003cimg style=\";width:115.5px;height:64.96875px\" src=\"\u000191\u0002\">\u003c/div>\u003cdiv class=\"tile-name\">Arctic\u003c/div>\u003c/div>\u003c/div>\u003c/div>\u003c/aside>","width":272,"height":590,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}]},"menu":{"html":"\u003cdiv class=\"ctx-menu\" data-testid=\"context-menu\" style=\"\">\u003cdiv data-testid=\"menu-add-to-chat\" class=\"ctx-item active\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Add to chat\u003c/span>\u003cspan class=\"ctx-key\">⌘L\u003c/span>\u003c/div>\u003cdiv class=\"ctx-sep\">\u003c/div>\u003cdiv data-testid=\"menu-split\" class=\"ctx-item disabled\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Split\u003c/span>\u003cspan class=\"ctx-key\">S\u003c/span>\u003c/div>\u003cdiv class=\"ctx-sep\">\u003c/div>\u003cdiv data-testid=\"menu-copy\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Copy\u003c/span>\u003cspan class=\"ctx-key\">⌘C\u003c/span>\u003c/div>\u003cdiv data-testid=\"menu-paste\" class=\"ctx-item disabled\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Paste\u003c/span>\u003cspan class=\"ctx-key\">⌘V\u003c/span>\u003c/div>\u003cdiv data-testid=\"menu-duplicate\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Duplicate\u003c/span>\u003cspan class=\"ctx-key\">⌘D\u003c/span>\u003c/div>\u003cdiv class=\"ctx-sep\">\u003c/div>\u003cdiv data-testid=\"menu-delete\" class=\"ctx-item danger\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Delete\u003c/span>\u003cspan class=\"ctx-key\">⌫\u003c/span>\u003c/div>\u003cdiv data-testid=\"menu-ripple-delete\" class=\"ctx-item danger\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Ripple delete\u003c/span>\u003cspan class=\"ctx-key\">⇧⌫\u003c/span>\u003c/div>\u003cdiv class=\"ctx-sep\">\u003c/div>\u003cdiv data-testid=\"menu-speed\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Speed\u003c/span>\u000192\u0002\u003c/div>\u003cdiv data-testid=\"menu-rename\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Rename\u003c/span>\u003c/div>\u003cdiv data-testid=\"menu-replace-media\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Replace media\u003c/span>\u000192\u0002\u003c/div>\u003cdiv data-testid=\"menu-show-in-media\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Show in Media\u003c/span>\u003c/div>\u003cdiv data-testid=\"menu-reveal\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Reveal in Finder\u003c/span>\u003c/div>\u003cdiv class=\"ctx-sep\">\u003c/div>\u003cdiv data-testid=\"menu-lock\" class=\"ctx-item\">\u003cspan class=\"ctx-check\">\u003c/span>\u003cspan class=\"ctx-label\">Lock\u003c/span>\u003c/div>\u003c/div>","width":220,"height":369,"ancestors":[{"tag":"div","cls":"app","attrs":{},"vars":"--side-col:clamp(240px, 440px, 40vw)"}],"backdrop":"rgb(10, 10, 10)","radius":"9px"}},"pool":["\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-mouse-pointer2\">\u003cpath d=\"M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-paperclip\">\u003cpath d=\"m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-at-sign\">\u003ccircle cx=\"12\" cy=\"12\" r=\"4\">\u003c/circle>\u003cpath d=\"M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8\">\u003c/path>\u003c/svg>","\u003csvg class=\"provider-logo\" preserveAspectRatio=\"xMidYMid\" viewBox=\"0 0 256 257\" fill=\"#d97757\">\u003cpath d=\"m50.228 170.321 50.357-28.257.843-2.463-.843-1.361h-2.462l-8.426-.518-28.775-.778-24.952-1.037-24.175-1.296-6.092-1.297L0 125.796l.583-3.759 5.12-3.434 7.324.648 16.202 1.101 24.304 1.685 17.629 1.037 26.118 2.722h4.148l.583-1.685-1.426-1.037-1.101-1.037-25.147-17.045-27.22-18.017-14.258-10.37-7.713-5.25-3.888-4.925-1.685-10.758 7-7.713 9.397.649 2.398.648 9.527 7.323 20.35 15.75L94.817 91.9l3.889 3.24 1.555-1.102.195-.777-1.75-2.917-14.453-26.118-15.425-26.572-6.87-11.018-1.814-6.61c-.648-2.723-1.102-4.991-1.102-7.778l7.972-10.823L71.42 0 82.05 1.426l4.472 3.888 6.61 15.101 10.694 23.786 16.591 32.34 4.861 9.592 2.592 8.879.973 2.722h1.685v-1.556l1.36-18.211 2.528-22.36 2.463-28.776.843-8.1 4.018-9.722 7.971-5.25 6.222 2.981 5.12 7.324-.713 4.73-3.046 19.768-5.962 30.98-3.889 20.739h2.268l2.593-2.593 10.499-13.934 17.628-22.036 7.778-8.749 9.073-9.657 5.833-4.601h11.018l8.1 12.055-3.628 12.443-11.342 14.388-9.398 12.184-13.48 18.147-8.426 14.518.778 1.166 2.01-.194 30.46-6.481 16.462-2.982 19.637-3.37 8.88 4.148.971 4.213-3.5 8.62-20.998 5.184-24.628 4.926-36.682 8.685-.454.324.519.648 16.526 1.555 7.065.389h17.304l32.21 2.398 8.426 5.574 5.055 6.805-.843 5.184-12.962 6.611-17.498-4.148-40.83-9.721-14-3.5h-1.944v1.167l11.666 11.406 21.387 19.314 26.767 24.887 1.36 6.157-3.434 4.86-3.63-.518-23.526-17.693-9.073-7.972-20.545-17.304h-1.36v1.814l4.73 6.935 25.017 37.59 1.296 11.536-1.814 3.76-6.481 2.268-7.13-1.297-14.647-20.544-15.1-23.138-12.185-20.739-1.49.843-7.194 77.448-3.37 3.953-7.778 2.981-6.48-4.925-3.436-7.972 3.435-15.749 4.148-20.544 3.37-16.333 3.046-20.285 1.815-6.74-.13-.454-1.49.194-15.295 20.999-23.267 31.433-18.406 19.702-4.407 1.75-7.648-3.954.713-7.064 4.277-6.286 25.47-32.405 15.36-20.092 9.917-11.6-.065-1.686h-.583L44.07 198.125l-12.055 1.555-5.185-4.86.648-7.972 2.463-2.593 20.35-13.999-.064.065Z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevron-down choice-caret\">\u003cpath d=\"m6 9 6 6 6-6\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-gauge\">\u003cpath d=\"m12 14 4-4\">\u003c/path>\u003cpath d=\"M3.34 19a10 10 0 1 1 17.32 0\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-sparkles\">\u003cpath d=\"M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z\">\u003c/path>\u003cpath d=\"M20 3v4\">\u003c/path>\u003cpath d=\"M22 5h-4\">\u003c/path>\u003cpath d=\"M4 17v2\">\u003c/path>\u003cpath d=\"M5 18H3\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-arrow-up\">\u003cpath d=\"m5 12 7-7 7 7\">\u003c/path>\u003cpath d=\"M12 19V5\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-clapperboard\">\u003cpath d=\"M20.2 6 3 11l-.9-2.4c-.3-1.1.3-2.2 1.3-2.5l13.5-4c1.1-.3 2.2.3 2.5 1.3Z\">\u003c/path>\u003cpath d=\"m6.2 5.3 3.1 3.9\">\u003c/path>\u003cpath d=\"m12.4 3.4 3.1 4\">\u003c/path>\u003cpath d=\"M3 11h18v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-pencil\">\u003cpath d=\"M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z\">\u003c/path>\u003cpath d=\"m15 5 4 4\">\u003c/path>\u003c/svg>","","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-panel-left\">\u003crect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\">\u003c/rect>\u003cpath d=\"M9 3v18\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-panel-bottom\">\u003crect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\">\u003c/rect>\u003cpath d=\"M3 15h18\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-panel-right\">\u003crect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\">\u003c/rect>\u003cpath d=\"M15 3v18\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-focus\">\u003ccircle cx=\"12\" cy=\"12\" r=\"3\">\u003c/circle>\u003cpath d=\"M3 7V5a2 2 0 0 1 2-2h2\">\u003c/path>\u003cpath d=\"M17 3h2a2 2 0 0 1 2 2v2\">\u003c/path>\u003cpath d=\"M21 17v2a2 2 0 0 1-2 2h-2\">\u003c/path>\u003cpath d=\"M7 21H5a2 2 0 0 1-2-2v-2\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-check\">\u003cpath d=\"M20 6 9 17l-5-5\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-undo2\">\u003cpath d=\"M9 14 4 9l5-5\">\u003c/path>\u003cpath d=\"M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-redo2\">\u003cpath d=\"m15 14 5-5-5-5\">\u003c/path>\u003cpath d=\"M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-share\">\u003cpath d=\"M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8\">\u003c/path>\u003cpolyline points=\"16 6 12 2 8 6\">\u003c/polyline>\u003cline x1=\"12\" x2=\"12\" y1=\"2\" y2=\"15\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-settings\">\u003cpath d=\"M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z\">\u003c/path>\u003ccircle cx=\"12\" cy=\"12\" r=\"3\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-x\">\u003cpath d=\"M18 6 6 18\">\u003c/path>\u003cpath d=\"m6 6 12 12\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-film\">\u003crect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\">\u003c/rect>\u003cpath d=\"M7 3v18\">\u003c/path>\u003cpath d=\"M3 7.5h4\">\u003c/path>\u003cpath d=\"M3 12h18\">\u003c/path>\u003cpath d=\"M3 16.5h4\">\u003c/path>\u003cpath d=\"M17 3v18\">\u003c/path>\u003cpath d=\"M17 7.5h4\">\u003c/path>\u003cpath d=\"M17 16.5h4\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-scroll-text\">\u003cpath d=\"M15 12h-5\">\u003c/path>\u003cpath d=\"M15 8h-5\">\u003c/path>\u003cpath d=\"M19 17V5a2 2 0 0 0-2-2H4\">\u003c/path>\u003cpath d=\"M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-type\">\u003cpolyline points=\"4 7 4 4 20 4 20 7\">\u003c/polyline>\u003cline x1=\"9\" x2=\"15\" y1=\"20\" y2=\"20\">\u003c/line>\u003cline x1=\"12\" x2=\"12\" y1=\"4\" y2=\"20\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-shapes\">\u003cpath d=\"M8.3 10a.7.7 0 0 1-.626-1.079L11.4 3a.7.7 0 0 1 1.198-.043L16.3 8.9a.7.7 0 0 1-.572 1.1Z\">\u003c/path>\u003crect x=\"3\" y=\"14\" width=\"7\" height=\"7\" rx=\"1\">\u003c/rect>\u003ccircle cx=\"17.5\" cy=\"17.5\" r=\"3.5\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-music\">\u003cpath d=\"M9 18V5l12-2v13\">\u003c/path>\u003ccircle cx=\"6\" cy=\"18\" r=\"3\">\u003c/circle>\u003ccircle cx=\"18\" cy=\"16\" r=\"3\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-audio-waveform\">\u003cpath d=\"M2 13a2 2 0 0 0 2-2V7a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0V4a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0v-4a2 2 0 0 1 2-2\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-blend\">\u003ccircle cx=\"9\" cy=\"9\" r=\"7\">\u003c/circle>\u003ccircle cx=\"15\" cy=\"15\" r=\"7\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-palette\">\u003ccircle cx=\"13.5\" cy=\"6.5\" r=\".5\" fill=\"currentColor\">\u003c/circle>\u003ccircle cx=\"17.5\" cy=\"10.5\" r=\".5\" fill=\"currentColor\">\u003c/circle>\u003ccircle cx=\"8.5\" cy=\"7.5\" r=\".5\" fill=\"currentColor\">\u003c/circle>\u003ccircle cx=\"6.5\" cy=\"12.5\" r=\".5\" fill=\"currentColor\">\u003c/circle>\u003cpath d=\"M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-circle-dot\">\u003ccircle cx=\"12\" cy=\"12\" r=\"10\">\u003c/circle>\u003ccircle cx=\"12\" cy=\"12\" r=\"1\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-swatch-book\">\u003cpath d=\"M11 17a4 4 0 0 1-8 0V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2Z\">\u003c/path>\u003cpath d=\"M16.7 13H19a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H7\">\u003c/path>\u003cpath d=\"M 7 17h.01\">\u003c/path>\u003cpath d=\"m11 8 2.3-2.3a2.4 2.4 0 0 1 3.404.004L18.6 7.6a2.4 2.4 0 0 1 .026 3.434L9.9 19.8\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-book-open\">\u003cpath d=\"M12 7v14\">\u003c/path>\u003cpath d=\"M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-plus\">\u003cpath d=\"M5 12h14\">\u003c/path>\u003cpath d=\"M12 5v14\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-layout-grid\">\u003crect width=\"7\" height=\"7\" x=\"3\" y=\"3\" rx=\"1\">\u003c/rect>\u003crect width=\"7\" height=\"7\" x=\"14\" y=\"3\" rx=\"1\">\u003c/rect>\u003crect width=\"7\" height=\"7\" x=\"14\" y=\"14\" rx=\"1\">\u003c/rect>\u003crect width=\"7\" height=\"7\" x=\"3\" y=\"14\" rx=\"1\">\u003c/rect>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-list\">\u003cpath d=\"M3 12h.01\">\u003c/path>\u003cpath d=\"M3 18h.01\">\u003c/path>\u003cpath d=\"M3 6h.01\">\u003c/path>\u003cpath d=\"M8 12h13\">\u003c/path>\u003cpath d=\"M8 18h13\">\u003c/path>\u003cpath d=\"M8 6h13\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-audio-lines\">\u003cpath d=\"M2 10v3\">\u003c/path>\u003cpath d=\"M6 6v11\">\u003c/path>\u003cpath d=\"M10 3v18\">\u003c/path>\u003cpath d=\"M14 8v7\">\u003c/path>\u003cpath d=\"M18 5v13\">\u003c/path>\u003cpath d=\"M22 10v3\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-skip-back\">\u003cpolygon points=\"19 20 9 12 19 4 19 20\">\u003c/polygon>\u003cline x1=\"5\" x2=\"5\" y1=\"19\" y2=\"5\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-step-back\">\u003cline x1=\"18\" x2=\"18\" y1=\"20\" y2=\"4\">\u003c/line>\u003cpolygon points=\"14,20 4,12 14,4\">\u003c/polygon>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-play\">\u003cpolygon points=\"6 3 20 12 6 21 6 3\">\u003c/polygon>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-step-forward\">\u003cline x1=\"6\" x2=\"6\" y1=\"4\" y2=\"20\">\u003c/line>\u003cpolygon points=\"10,4 20,12 10,20\">\u003c/polygon>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-skip-forward\">\u003cpolygon points=\"5 4 15 12 5 20 5 4\">\u003c/polygon>\u003cline x1=\"19\" x2=\"19\" y1=\"5\" y2=\"19\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-square-dashed-mouse-pointer\">\u003cpath d=\"M12.034 12.681a.498.498 0 0 1 .647-.647l9 3.5a.5.5 0 0 1-.033.943l-3.444 1.068a1 1 0 0 0-.66.66l-1.067 3.443a.5.5 0 0 1-.943.033z\">\u003c/path>\u003cpath d=\"M5 3a2 2 0 0 0-2 2\">\u003c/path>\u003cpath d=\"M19 3a2 2 0 0 1 2 2\">\u003c/path>\u003cpath d=\"M5 21a2 2 0 0 1-2-2\">\u003c/path>\u003cpath d=\"M9 3h1\">\u003c/path>\u003cpath d=\"M9 21h2\">\u003c/path>\u003cpath d=\"M14 3h1\">\u003c/path>\u003cpath d=\"M3 9v1\">\u003c/path>\u003cpath d=\"M21 9v2\">\u003c/path>\u003cpath d=\"M3 14v1\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-hand\">\u003cpath d=\"M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2\">\u003c/path>\u003cpath d=\"M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2\">\u003c/path>\u003cpath d=\"M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8\">\u003c/path>\u003cpath d=\"M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-maximize\">\u003cpath d=\"M8 3H5a2 2 0 0 0-2 2v3\">\u003c/path>\u003cpath d=\"M21 8V5a2 2 0 0 0-2-2h-3\">\u003c/path>\u003cpath d=\"M3 16v3a2 2 0 0 0 2 2h3\">\u003c/path>\u003cpath d=\"M16 21h3a2 2 0 0 0 2-2v-3\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-square-pen\">\u003cpath d=\"M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7\">\u003c/path>\u003cpath d=\"M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-clipboard-list\">\u003crect width=\"8\" height=\"4\" x=\"8\" y=\"2\" rx=\"1\" ry=\"1\">\u003c/rect>\u003cpath d=\"M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2\">\u003c/path>\u003cpath d=\"M12 11h4\">\u003c/path>\u003cpath d=\"M12 16h4\">\u003c/path>\u003cpath d=\"M8 11h.01\">\u003c/path>\u003cpath d=\"M8 16h.01\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-pencil-line\">\u003cpath d=\"M12 20h9\">\u003c/path>\u003cpath d=\"M16.376 3.622a1 1 0 0 1 3.002 3.002L7.368 18.635a2 2 0 0 1-.855.506l-2.872.838a.5.5 0 0 1-.62-.62l.838-2.872a2 2 0 0 1 .506-.854z\">\u003c/path>\u003cpath d=\"m15 5 3 3\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevron-right step-chev\">\u003cpath d=\"m9 18 6-6-6-6\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevron-down\">\u003cpath d=\"m6 9 6 6 6-6\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-copy\">\u003crect width=\"14\" height=\"14\" x=\"8\" y=\"8\" rx=\"2\" ry=\"2\">\u003c/rect>\u003cpath d=\"M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-slice\">\u003cpath d=\"M11 16.586V19a1 1 0 0 1-1 1H2L18.37 3.63a1 1 0 1 1 3 3l-9.663 9.663a1 1 0 0 1-1.414 0L8 14\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-scissors\">\u003ccircle cx=\"6\" cy=\"6\" r=\"3\">\u003c/circle>\u003cpath d=\"M8.12 8.12 12 12\">\u003c/path>\u003cpath d=\"M20 4 8.12 15.88\">\u003c/path>\u003ccircle cx=\"6\" cy=\"18\" r=\"3\">\u003c/circle>\u003cpath d=\"M14.8 14.8 20 20\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-trash2\">\u003cpath d=\"M3 6h18\">\u003c/path>\u003cpath d=\"M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6\">\u003c/path>\u003cpath d=\"M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2\">\u003c/path>\u003cline x1=\"10\" x2=\"10\" y1=\"11\" y2=\"17\">\u003c/line>\u003cline x1=\"14\" x2=\"14\" y1=\"11\" y2=\"17\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-magnet\">\u003cpath d=\"m6 15-4-4 6.75-6.77a7.79 7.79 0 0 1 11 11L13 22l-4-4 6.39-6.36a2.14 2.14 0 0 0-3-3L6 15\">\u003c/path>\u003cpath d=\"m5 8 4 4\">\u003c/path>\u003cpath d=\"m12 15 4 4\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-scan-line\">\u003cpath d=\"M3 7V5a2 2 0 0 1 2-2h2\">\u003c/path>\u003cpath d=\"M17 3h2a2 2 0 0 1 2 2v2\">\u003c/path>\u003cpath d=\"M21 17v2a2 2 0 0 1-2 2h-2\">\u003c/path>\u003cpath d=\"M7 21H5a2 2 0 0 1-2-2v-2\">\u003c/path>\u003cpath d=\"M7 12h10\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-link2\">\u003cpath d=\"M9 17H7A5 5 0 0 1 7 7h2\">\u003c/path>\u003cpath d=\"M15 7h2a5 5 0 1 1 0 10h-2\">\u003c/path>\u003cline x1=\"8\" x2=\"16\" y1=\"12\" y2=\"12\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-zoom-out\">\u003ccircle cx=\"11\" cy=\"11\" r=\"8\">\u003c/circle>\u003cline x1=\"21\" x2=\"16.65\" y1=\"21\" y2=\"16.65\">\u003c/line>\u003cline x1=\"8\" x2=\"14\" y1=\"11\" y2=\"11\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-zoom-in\">\u003ccircle cx=\"11\" cy=\"11\" r=\"8\">\u003c/circle>\u003cline x1=\"21\" x2=\"16.65\" y1=\"21\" y2=\"16.65\">\u003c/line>\u003cline x1=\"11\" x2=\"11\" y1=\"8\" y2=\"14\">\u003c/line>\u003cline x1=\"8\" x2=\"14\" y1=\"11\" y2=\"11\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevrons-left-right\">\u003cpath d=\"m9 7-5 5 5 5\">\u003c/path>\u003cpath d=\"m15 7 5 5-5 5\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-eye\">\u003cpath d=\"M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0\">\u003c/path>\u003ccircle cx=\"12\" cy=\"12\" r=\"3\">\u003c/circle>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-lock-open\">\u003crect width=\"18\" height=\"11\" x=\"3\" y=\"11\" rx=\"2\" ry=\"2\">\u003c/rect>\u003cpath d=\"M7 11V7a5 5 0 0 1 9.9-1\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-layers\">\u003cpath d=\"m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z\">\u003c/path>\u003cpath d=\"m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65\">\u003c/path>\u003cpath d=\"m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-sliders-horizontal\">\u003cline x1=\"21\" x2=\"14\" y1=\"4\" y2=\"4\">\u003c/line>\u003cline x1=\"10\" x2=\"3\" y1=\"4\" y2=\"4\">\u003c/line>\u003cline x1=\"21\" x2=\"12\" y1=\"12\" y2=\"12\">\u003c/line>\u003cline x1=\"8\" x2=\"3\" y1=\"12\" y2=\"12\">\u003c/line>\u003cline x1=\"21\" x2=\"16\" y1=\"20\" y2=\"20\">\u003c/line>\u003cline x1=\"12\" x2=\"3\" y1=\"20\" y2=\"20\">\u003c/line>\u003cline x1=\"14\" x2=\"14\" y1=\"2\" y2=\"6\">\u003c/line>\u003cline x1=\"8\" x2=\"8\" y1=\"10\" y2=\"14\">\u003c/line>\u003cline x1=\"16\" x2=\"16\" y1=\"18\" y2=\"22\">\u003c/line>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-volume2\">\u003cpath d=\"M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z\">\u003c/path>\u003cpath d=\"M16 9a5 5 0 0 1 0 6\">\u003c/path>\u003cpath d=\"M19.364 18.364a9 9 0 0 0 0-12.728\">\u003c/path>\u003c/svg>","data:image/png;base64,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","\u003csvg class=\"envelope\" data-testid=\"envelope\" data-keys=\"0\" viewBox=\"0 0 1694 100\" preserveAspectRatio=\"none\">\u003cpath class=\"env-line\" d=\"M0.0,25.00L1696.0,25.00\">\u003c/path>\u003c/svg>","data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAK4AAAAdCAYAAAAzUlc1AAAFgklEQVR4AezZTXIbNxAFYNA5RZwD+Aa6ib3zDZJ1UJV1qpx1cgPtpJvoBj5AnFOkGHxsteZHJDVlzgyl1Ez5EUB34z10AwOa9ruyPVsF3mAFtoP7BjdtW3Ip7+q3+9tf/77/Ze1i0P3tn7ubtXWvkascr5Er3f8r2o27v9nt9j+vn+D+5t99ubWhsJa+XOu3u69raspNrrVdEvprorZc61V0729r014q13Zwg5oIxGi9TxsK6ymGEs31b9/9jRqDFwdiNUt/XkN3f/g2lWtd4MV5OrhZOpsJOV6rlRyspUcnb9+64M1A5xi8OHDMt6SNJiypMeQ26l4coznw7ODaTMjbYL1D7A2FSGs93aGevCGsy3/SguWVhgrqC0Pr8iOacKnSs4ObhN5KN5FDzEYsC6wF9rlBE1K3tq+ZpbT6a6dX280rb9BfQ5cW1Jan9dS2Bu3SkC/YV1rX1lVrsJYpOHlw+5NrK6okFViiWiBUe4U2Nq+2eNDvw9z++KV+xHc/4mrTAnbteH59QTfXN553aizH0PJDI3H3VXyfax7d/U1y1kOeodfXp2usBf2EcSJ5sk37sda+4uCrh/rdfa1N39x6GN/fpk8L4hPGY5g7to3HdPGz41JrMGaH6Ee99RN8kw5uKd1XOMEkSCHCyIxrSzri/b0mil9bAcwxV78eYljOQ3xG4M5+2hUIV0CBrbPTtS5zxOv3OdinwNzMJ9pS6OKqLS8IO93YdFpizOUXO0VrGBe54Ej+PicNvkRtNeUHPMbaKbo4Io5m9GKu8f6GljXUx3zFJ2rT5dcmYm7wnP/EXwqu8vjgoAW16TFrA1FfvokH1/TTCOFYxDCKLUA4fMbRu/RzWKA+r/7+8M98tRWWTqxR73J0uvv2yxmGnLSGMUP/9476nDTGPPwwtl867rTkCkPGzj+0Xz5KLS10jLMc3I7uXG8ofC5y812nAm9JdcWD+5bKsq31tVdgO7ivfYe29R2twHZwj5ZlM772CmwH97Xv0La+oxXYDu7RsmzG116B7eC+9h06ur7NuOLB3T1s5d4qMFcFZjm4+/3ur1KOHUy2wJf3Hz+Xw2N86Fz88cOuPHKi6vPq7x6s68v7Tx949bVzoNMNnTEnrWHMOOL7xn1OGmMWfhjbLx13WqfzvVTj+Hx6PFrQD0w8uN2kfmGy/8dPH/90MI3joIjfPbAlyClAjOMwsZ2D+PTjzn7af//xU9P49IEm3nJ4eTpd6yrtEa/f52jmSX/MLY+80ZZCFxdNCDvdWAstMebyiy0TnmFc1BBHedTvc9LgA/PUgB9yrC0THhwRRjN6Mdd490CrtDXIBfjAPLr82gRfmfTgb/+Be7j4YgKO0rSAVmmPNhD1Lc0/6eCaZJEWlIXJfgiVw8OnIx70+5Bgf/xSP+J3D30teuza8XyaMLaLZ8v16U8BXXNxdoiXrs+VvjGnuWz9WOPj2D1knNySE0f2zTPWgj7kPDbIcbZsp2BfcfCHThwOc2Mc35T6YoAPch5bH3z98bE+3eTEo9Yglh2iH/XWT/CdPLhIFJCACchzQVpgnxs0IXUtcimt/trp0e1jDV11BnlaD33t0pAv2Fda19ZVa7CWKXh2cCUDSZKJTSG7LMbXBgTLerrL6gX76U91htMRy3jUF5ZhP81KE05HTPM8O7hIYdr0+aLcODAf48tMXlA3DbwcPW+EWxbmZX2ZjSa8HDlnxO7pt8hcrE8H1+bBXMRTeRQRpsbPFUdz/Re020C3LMyVz3mea+jGt6cztcSF1A5u/LPR+cTn97rtHB6bB/MrHGekq5hralqJXJfYQNznINfr6H78TPvc2i7xvZPU+jdPKTTXPjwKRVe7Nq6R69o5rqnXbtw15TatrQLzVOA/AAAA//863zCpAAAABklEQVQDACqfInfmcXlGAAAAAElFTkSuQmCC","\u003csvg class=\"envelope\" data-testid=\"envelope\" data-keys=\"0\" viewBox=\"0 0 173.65714285714284 100\" preserveAspectRatio=\"none\">\u003cpath class=\"env-line\" d=\"M0.0,25.00L175.7,25.00\">\u003c/path>\u003c/svg>","data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAdCAYAAACjbey/AAABAElEQVR4AeySwQ2CMBSGW51CHMAN2ARubKBnm3g2wbNu4A026QYOIE5haj8JpqYgxR6V5Oe9pu99/V9hJiKfeIBq6vP2Wm++NWIdmFRKs44AtK2qqS6oXYW/rYP34qnjeADGUfZewIbAPACNQphU2ZGAsQa0u1UpeRfJ0QCArVbKugF0N+L5tYhAlD2AilEAbihEgIhAiDgLAFDaL4BRALB/gBC/fgfGyFPAHUjND4PmUhRuPCyz4yigTLKCk2jeL3LdxTLJV8AGAFJTQCNFnEQzeRfJkQegiVPZpJH4SR4gpMkFvgBYRu5mSG4BUmM7pLivZsa8U227IOvAXU7PHwAAAP//1JOfpQAAAAZJREFUAwAFl2U70GFD7wAAAABJRU5ErkJggg==","\u003csvg class=\"envelope\" data-testid=\"envelope\" data-keys=\"0\" viewBox=\"0 0 16.17142857142857 100\" preserveAspectRatio=\"none\">\u003cpath class=\"env-line\" d=\"M0.0,25.00L18.2,25.00\">\u003c/path>\u003c/svg>","data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAC4AAAAdCAYAAADVV140AAAB9ElEQVR4AeyW0U3DMBCG7XYKygDdIJu0b90AnrHEM1J5hg361m6SDToAZQpk/JH+aSEkjZ0QqSIRH76zfXe/Lw5iYq70uV7h7rDbPLzt7odqPPWga73QcZ9Z6++6Jmof7zNjfPb4vv2ifdz3nUF4MeEO2z0U3t///vBmA6k1S+GSyrUB+UOMLlzX2DoV4Vwb4FWSbJhD+EziqQfUbqIiXJuL17jbcAgSgdZ0KPn9jIjf7qkHHATqctcKLwL4kIwhESAeOBTi3dl3gW/C48Jrh2CWP8RAOdHKoHbxEbtjHXJgE35BOFtOIB6YQTwjuCAW34UC/MUAF+YEMYDP/hjIq/3kwKZJUcIJ+h2683OFOaE1fNnpI4fpSXi6iNTIUXhq51Ljxo6ndi41bux4audS4/57x21e7RxzQqv4stPHqTWrqI4TAJTUiL2eLVb469lybgzibM6c8N6+Ar6JfMirEHJgP90s8wvCERH+8zgWJgBIxohQUDJGxAG2eL5dvID8diO1bX5ehxyqVyt8Gl4HAjglAaCCJJPd38hbWs6pB9SGuvwV4QSBxJ0LrkvSfR7RixV5qAfYTVSEEwRNQf2unUTH5C2Fc3cgJrjLXr4TaLoOTfmDcJtzNZo29blGLeAqQmruCSce8mpQC1IFKy50XOZ1jZ8AAAD//4EPVbwAAAAGSURBVAMAlaAjSs0k+HcAAAAASUVORK5CYII=","\u003csvg class=\"envelope\" data-testid=\"envelope\" data-keys=\"0\" viewBox=\"0 0 46.457142857142856 100\" preserveAspectRatio=\"none\">\u003cpath class=\"env-line\" d=\"M0.0,25.00L48.5,25.00\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevron-right\">\u003cpath d=\"m9 18 6-6-6-6\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-diamond\">\u003cpath d=\"M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-rotate-ccw\">\u003cpath d=\"M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8\">\u003c/path>\u003cpath d=\"M3 3v5h5\">\u003c/path>\u003c/svg>","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-ban\">\u003ccircle cx=\"12\" cy=\"12\" r=\"10\">\u003c/circle>\u003cpath d=\"m4.9 4.9 14.2 14.2\">\u003c/path>\u003c/svg>","data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/4gHYSUNDX1BST0ZJTEUAAQEAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADb/2wBDAAkGBwgHBgkIBwgKCgkLDRYPDQwMDRsUFRAWIB0iIiAdHx8kKDQsJCYxJx8fLT0tMTU3Ojo6Iys/RD84QzQ5Ojf/2wBDAQoKCg0MDRoPDxo3JR8lNzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzf/wAARCABBAHQDASIAAhEBAxEB/8QAHAABAQADAQEBAQAAAAAAAAAAAAUEBgcBAwII/8QAORAAAQMCBAMGBAQEBwAAAAAAAQACAwQRBRIUYRMhMQZBUnGBoQciUZEXIzJyFRZTsVRlk5TS4eL/xAAaAQACAwEBAAAAAAAAAAAAAAAAAwIEBQEG/8QALREAAQMBBgUDBAMAAAAAAAAAAQACAxEEBRIUIVETFTGB8DJBcWGRocEiseH/2gAMAwEAAhEDEQA/AMTTbJptlZ02yabZX+ZDdV8gVG02yabZWdNsmm2RzIboyBUbTbJptlZ02yabZHMhujIFRtNsmm2VnTbJptkcyG6MgVG02yabZWdNsmm2RzIboyBUbTbJptlZ02yabZHMhujIFRtNsmm2VnTbJptkcyG67kCo2m2RWdNsiOZDdcyBVjT7Jp9lU4Gy8dEGgudYAC5J7l4jMPXp8DFM0+yafZScR7Y0FLVthp43VLAbSSMNgP2/VV8KxbDsVA0lQ3if0n/K8enf6JrnztGIjRJbLZ3uwtdqvNPsmn2VTgbJwNkrMPTsDFL0+yafZVOBspGM43h+EAtnkzz90MfN3r9PVSbNK40Ci/hMFXGgX70+yafZaPW9rsSnqWyU7m08bDdsbQDf9xPVbbgXxEw6Zgp8ew+OE2tx4GXafNvUel1bEU9Kkqhn7PioAsvT7Jp9lUcKKoPFw6piqKdwu10bg62x+hTgbKo6eRpoVfaI3AOCl6fZNPsqnA2XnA2Ucw9SwMUzT7IlRjeD08hjlr4Q8dQ05rfZFPiTbFLL4B1cPuquM4pQ4LT8aulDSf0Rt5vf5Bcw7RdqKzGXGNv5FJflCw/q/ce/+ywK2qfXVDqiskdNK7q57iV8LReH3K3obldHqSCfPovNT3mZdBUBY69BLSCCQRzBCrU+CVtTBx4KCV8drh3Pn5fX0WE+NjHFj4y1wNiDcEJ4sL3EgOH3/wAVYyFoBLSKqzhHbLE8PysmcKuEcssp+YeTuv3utypO22CzUr5ZnyQSMFzC5ly79pHI+y5laLw+5S0Xh9ykvuUv10HnwrEd5yRinX5Wx4722rK7NDh7TSQHlmB/McPPu9PutVcS4kuJJPMk96+9ovD7lLReH3Kay63MFG087JElsMhq6pWOiyLReH3KWi8PuVPl8u487JfHbsvKOsqaGYTUk74ZB3sNr+f1W5YN2+czLFi9PnHQzRCzvVvT7WWnWi8PuUtF4fcpcl0uk9VPOydFbnRemq37Fe39HECzC6d07v6kvyt+3U+y03FO0GJ4oSKqpdwzf8qP5W/YdfW6w7ReH3KWi8PuVGO5yzpTzspS3i+T1E0WOiyLReH3KJ3L5dx52Vfjt2WBx1n4DwqnGqKCosY3zNDge/n09VrvH3XraktcHNcQQbgg9FoOcS0gFdYwNcCQv6DAAFgLALn3xIZBBiFNLGQJpYzxGj6Aix/uPRTKT4k4jDSiKengnkAsJXXBPmB19lrOKYxU4pWPq6yTPK7ly5Bo7gB3BZVjsskUuJ3T+1r221xTQ4GjU/hZPHWRRN1Uxjz5flve11E4+6zMLxGOkqTJMHFuUizevctKV7sBwdVQsUUJtDBP6K6/Cs1NEYIHy8a+UdMv/a3X8Nv83P8Atv8A0uf12O0tRSSRRslDnDkXAW6+a6F+K2Bf4XEf9Nn/ADWJa5rzDW8Ktda6D6U/a2bbBdQmpDTDQe561Nff4WpOwa1XWQam+mqHw5sn6sp69eSxayhNNLDHxs3Fzc8trW9V7/M9FrcQnMc+Wpq5JmDKLhrjyB59ViYhj1LU1NLJGyUNiz5g4C5uBa3NbtmeS5nF6aV/a8oGyZmlP4V/C+0lOWRufxL5QT0XQMI+FoxLCaKv/jRj1VPHNk0t8uZoNr5+fVc0nxmnkhkY1sl3NIFwPp5rp+A/F3AMOwPDqGekxJ0tNSxQvLI4y0ua0A2u/pyVm8HRtLcv38K0hHDiXOO3MR7J9oJ8KD9Vwsv5pGS92Nd05+L69yj0OLGqlcwxZLNvfNdZnxFxaDtR2pqsUoGyRwS5MrZwA4WY1puASOrT3qBRROp5S9zmm7bclRD5MQr0XDHFgNOq2DjopnH3RPxqtw1ioiJSeiIiEIiIhCIiIQiIiEIiIhCIiIQiIiEL/9k=","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/4gHYSUNDX1BST0ZJTEUAAQEAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADb/2wBDAAkGBwgHBgkIBwgKCgkLDRYPDQwMDRsUFRAWIB0iIiAdHx8kKDQsJCYxJx8fLT0tMTU3Ojo6Iys/RD84QzQ5Ojf/2wBDAQoKCg0MDRoPDxo3JR8lNzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzf/wAARCABBAHQDASIAAhEBAxEB/8QAHAABAQADAQEBAQAAAAAAAAAAAAUDBAYHAgEI/8QAORAAAQMCBAMGBAQEBwAAAAAAAQACAwQRBRIUYRMhMQZBUVKBoQcicZEjMnKxFRYXU1RVk5TB0uL/xAAbAQACAwEBAQAAAAAAAAAAAAAAAwIEBQYBB//EADARAAEDAgMGBQIHAAAAAAAAAAEAAgMEERIVUQUUITFhoRMycYHRBkEiI5HB4fDx/9oADAMBAAIRAxEAPwDT02yabZWdNsmm2Wnm41VbLeijabZNNsrOm2TTbIzcaoy3oo2m2TTbKzptk02yM3GqMt6KNptk02ys6bZNNsjNxqjLeijabZNNsrOm2TTbIzcaoy3oo2m2TTbKzptk02yM3GqMt6KNptk02ys6bZNNsjNxqjLeijabZFZ02yIzcaoy3orGnCacKhwNl+OhDQXO5AC5J7l873qRddhjWhpwmnCjYj2voKWqbDTxuqWA2kkYbAfp8VXwvFcOxUDSTtMn9p/yvHp3+ia59Q1uIjgkMqKV7ixpF19acJpwqHA2TgbJW9SJ+GPRT9OE04VDgbKTjONYfhALZ5M83dDHzd6+HqpNnmebN4qL3QsbidwCzacJpwuFre1uIz1DZKctp42G7Y2i9/1E9V12BfEXDpmCnx7Do4Ta3Hp2Xafq3qPS6uCGotclZ2aUuKwBW5pwmnCpPbRTni4bUx1FO8Xa6N2a2x8CvngbKk6olaSCtFhie0OHIqfpwmnCocDZOBsvN6kUsMein6cIsdRjWEU8hjlr4Q8dQ05rfZFMS1B+xSzLTDgXD9Qq+NYnQ4LT8aulDSfyRjm9/wBAvMe0Paisxlxjb+BSX5RMP5v1Hv8A2U+tq319Q6orJHTTO6ue4lYLReX3K6WD6ffFxJBPv8Lj6nbBm/CLgLAgJaQWkgjmCO5V6fBK2pg48FBK+K1w6x5/Tx9FovYxjyx8Za4GxBuCFYbs57iQHtNuv8Kq6UsALmkA9FZwjtlieH5WTOFXCOWWU/MPo7r97rsqTttgk1K+aZ8kEjBcwuZcu/SRyPsvMrReX3KWi8vuUmT6fLze4Hpf4ViLbEkYtxI6/wCrpMd7bVlbmhw9ppIDyzA/iOH17vT7rlHEuJc4kk8yT3rPaLy+5S0Xl9ymx7FfGLNI7/CRLXmU3fcrAiz2i8vuUtF5fcpmVTajv8JW8s0KUdZU0MwmpJ3xSDvYbX+viuxwbt85mWLF6fOOhmiFj6t6fay460Xl9ylovL7lKk2G6TzW7/CfDtJ0PkJXfYr2+o4rswundO7+5L8rft1PsuNxTtBieKEiqqXcM3/Cj+Vv2HX1utO0Xl9ylovL7lRj2CY+Vr+/wpy7VfLwcTZYEWe0Xl9yiflU2o7/AAqu8s0K0eOt/AOFVY1RQVFjE+ZocD38+nqud46/W1Ba4Oa4gg3BB6LUe4uaQDZTZFhcHEXsv6FAAFgLALzz4ksggxGlljIE0sZ4jR4Aix/ceinUnxJxKGlEU9PBPK0WErrgn6gdfZcximMVOK1j6utkzyu5chYNHcAO4LFoKKaCfG48B3W7tGthqKfw2C5PZbHHWejGplMebLyve11F4628Mr46WoMkocW5SPl6rYmkf4Zwc1l0FPAapgqPJfj6eyr1FGYYXScW+UdMq7j+mg/zc/7b/wBLz6txqmnpZIo2ShzhyJAt+69F/qtgX+FxH/TZ/wB1zlfUbXaxm73vc34DpbmPVbtdS7HE9oLYbD7nnc3++llx7sGtV1kGpvpqh8ObJ+bKevXktasoTTTQx8bNxc3PLa1vVfX8z0WuxCcxz5amrkmYMouGuPIHn1WniGO01TU00kbJQ2LPmDgLm4Frc10lJI4lni9L/uuNEcu92I/Lv2WaSlLI3P4l8oJ6L0HCPhYMSwmir/40Y9VTxzZNLfLmaDa+fn1Xmk+M08kMjGtku5pAuB4fVen4D8XsAw7AsOoZ6TEnS01LFC8sjjLS5rQDa7+nJWtpOjaW7sfX+la4gp8XLh7rzXt5GeyXaCfCQ/V8LL+KRkvdjXdOfm8e5RsPxg1kzozFks2981/+FvfEjFIe1XaqqxXD2vjglyZWzgBwsxrTcAkdWnvUCggfTSue5zSC23IrPbJNjF+Sg6CDAbDiuh46KZx0VrxFU8FaqIirq4iIiEIiIhCIiIQiIiEIiIhCIiIQiIiEL//Z","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,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","data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/4gHYSUNDX1BST0ZJTEUAAQEAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADb/2wBDAAkGBwgHBgkIBwgKCgkLDRYPDQwMDRsUFRAWIB0iIiAdHx8kKDQsJCYxJx8fLT0tMTU3Ojo6Iys/RD84QzQ5Ojf/2wBDAQoKCg0MDRoPDxo3JR8lNzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzf/wAARCABBAHQDASIAAhEBAxEB/8QAGwABAQADAQEBAAAAAAAAAAAAAAUDBAYBBwL/xAA7EAABAwIEAwYEBAILAAAAAAABAAIDBBEFEhRhEyExBkFScYGhByJRkRUXMrEjJDRDU1RlcpOUwdLi/8QAGgEAAgMBAQAAAAAAAAAAAAAAAAMCBAUBBv/EACgRAAIBAwMEAQQDAAAAAAAAAAABAgMRFAQTIQUSMVFhIzJCwYHh8P/aAAwDAQACEQMRAD8A80+yafZVdPsmn2WhmlXGJWn2TT7Krp9k0+yM0MYlafZNPsqun2TT7IzQxiVp9k0+yq6fZNPsjNDGJWn2TT7Krp9k0+yM0MYlafZNPsqun2TT7IzQxiVp9k0+yq6fZNPsjNDGJWn2RVdPsiM0MYq6fZNPsqfB2R0QaCXcgBck9y8vvzNzbgTNPsmn2UzEO1lDTVQigjdUMBs+RpsB5fVVsMxTD8TA0s44n9m7k77d/opudVK7QuMqMnZPk/Gn2TT7Knwdk4OyhvzGbcCZp9k0+yp8HZS8XxigwoFs8mebuiZzd6/T1XVWqSdkRlGnFXbPdPsmn2XHVnanEJ6hskDmwRtN2xtF7+ZPVdTgnb6glYIMboI4ja3GgZdp8x1HpdWFGrbllV6mjeyRsafZNPsqjhRzni4fURzwOF2uY69tjuvODsq7rTTsy3GMJK6Jmn2TT7Knwdk4Oy5vzO7cCZp9kXs+M4TBIY5a6IOHUNOa32RS3Kvoh9Ffkipi+JUWD0/FrZQ0n9EY5uf5BfN8f7S1eLuMY/gUvdE09f8AMe/9lo1lU+tqHT1cjpZXdXOcVhtF4fcrVp9NlHm6uY1XXOfC8GBASCCCQR0IVSDB6uoh40NFI6PqHc+fl9VpvYxji18Za4GxBuCExaWT4TQlzty0yxhXa7EqDKyZwqoRyyyn5h5O6/e662l7ZYPNSvllfJDIwXMTm3LvK3I+y+c2i8PuUtF4fcpcumuXr/fwNhrpR4OgxvtlV1uaKgBpYDyzA/O4efd6fdcwSXElxJJ6krPaLw+5S0Xh9ymR0EoqysLnqXN3lcwIs9ovD7lLReH3Klh1PaIbq9HlJV1FFMJaWZ8Tx3tNr+f1XW4R25czLHisGcdONFyPq3p9lydovD7lLReH3KhLp8pebDIapw+253GJ9uaSIFmGwOnd45Plb9up9lyWJY7iWJXFTUu4Z/q2fK37Dr6rUtF4fcpaLw+5XI9OcfFjs9ZKfm5gRZ7ReH3KJmHU9oVur0aHH3W9gZiqcXo4Z7GN8rQ4Hv59FznH3XraktcHNcQQbgg9FblK6sEY2aZ97AAFhyAXB/EJsMFfTSsIE0rDxAPoOh/ceim0vxFxCGmEc1PBPIBYSm4J8wOvsubxLF6jE6t9VVyZpHcuXIAfQD6Kjp6M6c+5l/UV4VKfajb4+6zUn8zKWB2Xle9rqJx91t4biEdLUGSUOLcpHyq5UnLtfb5K2lp03Wiqv235LNRSmGF0nFvl7sq7L8u/8VP+3/8AS4GtxumnpZImNlDnDkSBb913/wCaOB/3XEP9Nn/dZeoq65Rjt+eb8L4t+zS1VLQKr9Lxb2/PP9HL/hH81Vw6n+j1D4b5P1ZT168lr1lEaWWFnGzcXNzy2tb1Xje09EKyvmMc+Woq5JmDKLhrjyB59VqYjj1LU1NNJGyUNiz5g4C5uBa3Na9CV3Hc+L/swFGe/b8bmaSAsY53E/SCei7vCvhr+IYXR1v4uY9TAyXJpr5czQbXzc+q+czYzTvhkY1sl3NIFwPp5r6RgfxXwLD8Fw+inpcRdLT00cTyyNhaXNaAbXf05J+udNNbBcUKdz5723aeyuOzYWHarh5f4hGS92td05+L2Umgxg1crozFks2981/+Fs/ELE4e03aapxOhD44JcmVswAcLMa03sSOrT3qHQwuppXPc5pBbbkVSU6ncr+DrhT7fk6Lj7opXH3RO7xHYaqIiWNCIiACIiACIiACIiACIiACIiACIiAP/2Q==","\u003csvg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-chevron-right ctx-more\">\u003cpath d=\"m9 18 6-6-6-6\">\u003c/path>\u003c/svg>"]};

const SLOTS = {
  composer: { box: ".composer-box", input: ".composer-box > textarea", text: ".composer-hl", context: ".composer-ctx", send: ".send", attach: ".composer-at", model: ".choice.model" },
  message: { text: ".msg-user" },
  turn: { reply: ".reply", steps: ".worklog", step: ".worklog .step", edit: ".edit-row", review: ".card-item" },
  storyboard: { summary: ".approval-text", beats: ".beats", beat: ".beat", approve: ".btn.primary", edit: ".approval-btns .btn.ghost" },
  review: { head: ".card-head", status: ".card-outcome", scores: ".scores", score: ".score", bar: ".score-bar > span" },
  timeline: { toolbar: ".bottom-toolbar", ruler: ".tl-ruler", lanes: ".tl-lanes", heads: ".tl-heads", playhead: ".playhead", clip: ".clip", lane: ".tl-lane", main: ".tl-lane.main" },
  preview: { frame: ".frame", transport: ".transport", timecode: ".timecode", play: ".icon-btn.play" },
  inspector: { title: ".insp-title", filter: '[data-testid="insp-filter"]', amount: '[data-testid="insp-filter-intensity"]', effects: '[data-testid="insp-effects"]', effect: '[data-testid="insp-effect"]', transform: '[data-testid="insp-sec-transform"]' },
  effects: { search: ".tr-search", target: '[data-testid="effects-target"]', filters: '[data-testid="filters-grid"]', tile: '[data-testid="filter-tile"]', none: '[data-look="none"]', natural: '[data-look="natural"]', warm: '[data-look="warm"]', cool: '[data-look="cool"]', vintage: '[data-look="vintage"]', film: '[data-look="film"]', bw: '[data-look="bw"]', vivid: '[data-look="vivid"]' },
  export: { close: ".export-head .icon-btn", draft: '[data-testid="quality-draft"]', standard: '[data-testid="quality-standard"]', high: '[data-testid="quality-high"]', file: '[data-testid="export-name"]', motionBlur: 'input[type="checkbox"]', cancel: ".wiz-foot .btn.ghost", export: ".wiz-foot .btn.primary" },
  exportProgress: { job: '[data-testid="export-job"]', progress: ".export-status", bar: ".export-bar", fill: ".export-bar > div", cancel: '[data-testid="export-cancel"]' },
  topbar: { export: ".export-btn", saved: ".saved", title: ".project-title" },
  agent: { tabs: ".tabs", thread: ".agent-thread", composer: ".composer", send: ".send", plan: ".plan-chip-btn" },
  menu: { item: ".ctx-item", addToChat: '[data-testid="menu-add-to-chat"]', label: ".ctx-label", key: ".ctx-key", sep: ".ctx-sep" },
  editor: { topbar: ".topbar", export: ".export-btn", rail: ".rail", library: ".library", preview: ".stage", frame: ".frame", side: ".side", composer: ".composer", send: ".send", thread: ".agent-thread", timeline: ".bottom", playhead: ".playhead" },
};

function parts() {
  return Object.keys(DATA.parts);
}

function slots(name) {
  part(name);
  return Object.keys(SLOTS[name] || {});
}

function size(name) {
  const p = part(name);
  return { width: p.width, height: p.height };
}

function part(name) {
  const p = Object.hasOwn(DATA.parts, name) ? DATA.parts[name] : undefined;
  if (!p) throw new Error(`biliupUI: no part "${name}" (parts: ${parts().join(", ")})`);
  return p;
}

function css() {
  return DATA.css;
}

function ensureCss(doc) {
  if (doc.querySelector("style[data-biliup-ui]")) return;
  const style = doc.createElement("style");
  style.setAttribute("data-biliup-ui", "");

  style.textContent = `${DATA.css}\n[data-biliup-fly-hidden],[data-biliup-fly-hidden] *{visibility:hidden !important}`;
  (doc.head || doc.documentElement).appendChild(style);
}

const SHELL = "display:block;position:relative;width:100%;height:100%;margin:0;padding:0;border:0;background:none;box-shadow:none;outline:0;overflow:visible;transform:none;opacity:1;filter:none;backdrop-filter:none;inset:auto;min-width:0;min-height:0;max-width:none;max-height:none;";

function mount(host , name , props) {
  const p = part(name);
  const o = props || {};
  const doc = host.ownerDocument;
  ensureCss(doc);

  const stable = { ...o, scale: undefined, blur: undefined, dim: undefined, time: undefined };
  if (name === "editor") for (const k of EDITOR_LIVE_PROPS) stable[k] = undefined;
  const key = `${name}\n${keyOf(stable)}`;
  let wrap = host.firstElementChild ;
  if (!wrap || wrap.__uiKey !== key) {
    const width = num(o.width, p.width);
    const height = num(o.height, p.height);
    wrap = doc.createElement("div") ;
    wrap.className = "biliup-ui";
    wrap.__uiKey = key;
    wrap.__uiName = name;
    wrap.setAttribute("data-part", name);
    let at = wrap;
    for (const a of p.ancestors) {
      const shell = doc.createElement(a.tag);
      if (a.cls) shell.className = a.cls;
      for (const k of Object.keys(a.attrs)) shell.setAttribute(k, a.attrs[k] );
      shell.setAttribute("style", SHELL + a.vars);
      at.appendChild(shell);
      at = shell;
    }
    const tpl = doc.createElement("template");
    tpl.innerHTML = expand(p.html);
    const root = tpl.content.firstElementChild ;
    root.style.width = `${width}px`;
    root.style.height = `${height}px`;
    root.style.margin = "0";
    root.style.position = root.style.position || "relative";
    at.appendChild(root);
    const back = typeof o.background === "string" ? o.background : p.backdrop;
    wrap.style.cssText = `position:relative;width:${width}px;height:${height}px;transform-origin:0 0${back ? `;background:${back};border-radius:${p.radius || "0"}` : ""}`;
    neutral(root);
    applyProps(name, root, name === "editor" ? { media: o.media } : o);
    host.replaceChildren(wrap);

    for (const el of wrap.querySelectorAll("[data-ui-scroll]")) el.scrollTop = Number(el.getAttribute("data-ui-scroll"));
  }
  const scale = num(o.scale, 1);
  wrap.style.transform = `scale(${scale})`;

  const blur = Math.max(0, num(o.blur, 0));
  const dim = Math.max(0, Math.min(1, num(o.dim, 0)));
  wrap.style.filter = `${blur > 0.05 ? `blur(${(blur / Math.max(scale, 0.01)).toFixed(2)}px) ` : ""}${dim > 0.001 ? `brightness(${(1 - dim).toFixed(3)})` : ""}`.trim() || "none";
  const root = rootOf(wrap);
  if (name === "editor") liveEditor(root, o);
  placeElements(name, root, o);
  spin(root, o.time);
  return root;
}

function spin(root , time) {
  const turn = typeof time === "number" && Number.isFinite(time) ? ((time / 0.9) % 1) * 360 : 0;
  for (const el of root.querySelectorAll(".spin")) {
    const st = (el).style;
    st.animation = "none";
    st.transformOrigin = "50% 50%";
    st.transform = `rotate(${turn.toFixed(2)}deg)`;
  }
}

const EDITOR_LIVE_PROPS = ["side", "library", "composer", "turn", "message", "review", "inspector", "effects", "timeline", "preview", "topbar", "export", "exportProgress", "text", "press", "highlight", "hide", "remove"];

function liveEditor(root , o) {
  const doc = root.ownerDocument;
  if (!root.__pristine) {
    root.__pristine = new Map();
    root.__liveKeys = new Map();
    for (const sel of [".side", ".library", ".bottom", ".stage", ".topbar"]) {
      const el = root.querySelector(sel) ;
      if (el) root.__pristine.set(sel, el.cloneNode(true));
    }
  }
  const sub = (k) => (o[k] && typeof o[k] === "object" ? (o[k]) : undefined);
  const fresh = (name) => {
    const tpl = doc.createElement("template");
    tpl.innerHTML = expand(part(name).html);
    const el = tpl.content.firstElementChild ;
    neutral(el);
    return el;
  };
  const clips = (sub("timeline")?.clips) ?? [];
  const length = clips.reduce((m, c) => Math.max(m, num(c.start, 0) + num(c.duration, 0)), 0);
  const common = keyOf({ text: o.text, press: o.press, highlight: o.highlight, hide: o.hide, remove: o.remove });

  const pristine = (sel) => root.__pristine .get(sel) .cloneNode(true) ;
  const units = [
    {
      sel: ".side",
      spec: [o.side, o.composer, o.turn, o.message, o.review, o.inspector],
      base: () => (o.side === "inspector" ? fresh("inspector") : pristine(".side")),
      apply: (el) => (o.side === "inspector" ? applyProps("inspector", el, sub("inspector") || {}) : nested(el, o, ["composer", "turn", "message", "review"])),
    },
    {
      sel: ".library",
      spec: [o.library, o.effects],
      base: () => (o.library === "effects" ? fresh("effects") : pristine(".library")),
      apply: (el) => {
        if (o.library === "effects") applyProps("effects", el, sub("effects") || {});
        for (const b of root.querySelectorAll(".rail [data-rail]")) b.classList.toggle("active", b.getAttribute("data-rail") === (o.library === "effects" ? "effects" : "media"));
      },
    },
    { sel: ".bottom", spec: [o.timeline], base: () => pristine(".bottom"), apply: (el) => applyProps("timeline", el, sub("timeline") || {}) },
    { sel: ".stage", spec: [o.preview, length], base: () => pristine(".stage"), apply: (el) => applyProps("preview", el, { duration: length, ...(sub("preview") || {}) }) },
    { sel: ".topbar", spec: [o.topbar], base: () => pristine(".topbar"), apply: (el) => applyProps("topbar", el, sub("topbar") || {}) },
  ];
  let changed = false;
  for (const u of units) {
    if (!root.__pristine.has(u.sel)) continue;
    const key = `${keyOf({ s: u.spec })}\n${common}`;
    if (root.__liveKeys .get(u.sel) === key) continue;
    const next = u.base();
    const cur = root.querySelector(u.sel);
    if (cur) {
      cur.replaceWith(next);
      u.apply(next);
    }
    root.__liveKeys .set(u.sel, key);
    changed = true;
  }

  const dialog = sub("exportProgress") ? "exportProgress" : sub("export") ? "export" : "";
  const dkey = `${dialog}\n${keyOf({ d: dialog ? o[dialog] : null })}\n${common}`;
  if (root.__liveKeys .get("modal") !== dkey) {
    root.querySelector(":scope > .modal-layer")?.remove();
    if (dialog) {
      const layer = doc.createElement("div");
      const anc = part(dialog).ancestors.at(-1);
      layer.className = anc?.cls || "modal-layer";
      if (anc?.vars) layer.setAttribute("style", anc.vars);
      const el = fresh(dialog);
      layer.appendChild(el);
      root.appendChild(layer);
      applyProps(dialog, el, { duration: length, ...(sub(dialog) || {}) });
    }
    root.__liveKeys .set("modal", dkey);
    changed = true;
  }
  if (changed) applyProps("", root, { text: o.text, press: o.press, highlight: o.highlight, hide: o.hide, remove: o.remove });
}

function rootOf(wrap) {
  let at = wrap;
  while (at.firstElementChild) {
    const next = at.firstElementChild;
    if (next.getAttribute("style")?.startsWith(SHELL)) at = next;
    else return next ;
  }
  return at ;
}

function keyOf(o) {
  return JSON.stringify(o, (k, v) => (k && typeof v === "object" && v !== null && "nodeType" in v ? "[element]" : v));
}

function num(v , fallback) {
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}

function find(root , slot , name) {
  const owner = name || partName(root);
  const sel = (owner && SLOTS[owner]?.[slot]) || slot;
  if (root.matches(sel)) return root ;
  return root.querySelector(sel) ;
}

function findAll(root , slot , name) {
  const owner = name || partName(root);
  const sel = (owner && SLOTS[owner]?.[slot]) || slot;
  return [...(root.matches(sel) ? [root] : []), ...root.querySelectorAll(sel)] ;
}

function partName(el) {
  const wrap = el.closest(".biliup-ui") ;
  return wrap?.__uiName;
}

function list(v) {
  return v === undefined || v === null ? [] : Array.isArray(v) ? v.map(String) : [String(v)];
}

function setText(el , text) {
  if (el && text !== undefined && text !== null) el.textContent = String(text);
}

function setLead(el , text) {
  if (!el) return;
  const node = [...el.childNodes].find((n) => n.nodeType === 3);
  if (node) node.textContent = text;
  else el.insertBefore(el.ownerDocument.createTextNode(text), el.firstChild);
}

function blankReview(card) {
  card.removeAttribute("data-passed");
  setText(card.querySelector(".card-head .grow"), "Review");
  const out = card.querySelector(".card-outcome");
  if (out) {
    out.className = "card-outcome";
    out.textContent = "Reviewing";
  }
  for (const row of card.querySelectorAll(".score")) {
    row.removeAttribute("data-value");
    row.classList.remove("low", "ok");
    const bar = row.querySelector(".score-bar > span") ;
    if (bar) bar.style.width = "0%";
    setText(row.querySelector(".score-value"), "");
  }
}

function neutral(root) {
  for (const card of findAll(root, '.card-item[data-type="review"]')) blankReview(card);
  for (const el of findAll(root, ".plan-state")) el.remove();
  for (const el of findAll(root, ".turn-status")) {
    el.textContent = "";
    el.removeAttribute("title");
  }
  for (const el of findAll(root, ".export-status .muted")) el.textContent = "";

  for (const el of findAll(root, ".export-status")) setLead(el, "Rendering…");
  for (const el of root.querySelectorAll             (".export-bar > div")) el.style.width = "0%";

  for (const el of findAll(root, '[data-testid="export-job-where"]')) el.remove();

  for (const el of root.querySelectorAll('.turn .worklog .step, .turn .reply, .turn-edits, .turn .card-item, .turn .edit-row, [data-testid="media-tile"], .tl-lane .clip')) el.remove();

  for (const el of findAll(root, ".msg-user")) el.textContent = "";
  for (const el of findAll(root, ".composer-hl")) el.textContent = "\u200b";
  for (const el of findAll(root, ".composer-box > textarea")) el.textContent = "";
  for (const el of findAll(root, ".composer .send")) el.setAttribute("disabled", "");

  for (const el of findAll(root, ".project-title")) el.textContent = "";
  for (const el of root.querySelectorAll(".timecode .tc-end")) el.textContent = ` / ${timecode(0)}`;
  for (const el of findAll(root, '[data-testid="canvas-info"]')) el.textContent = "";
  for (const el of root.querySelectorAll('.insp-title .grow, [data-testid="insp-media"], [data-testid="insp-duration"]')) el.textContent = "";
  for (const el of root.querySelectorAll('[data-testid="insp-name"], [data-testid="insp-media-name"]')) {
    el.removeAttribute("placeholder");
    el.setAttribute("value", "");
  }
  for (const el of findAll(root, '[data-testid="effects-target"]')) el.textContent = "";
  for (const el of findAll(root, '[data-testid="export-range-text"]')) el.textContent = "";

  for (const el of root.querySelectorAll('[data-testid="insp-start"], [data-testid="insp-in"], [data-testid="insp-out"]')) el.textContent = "";
  for (const el of findAll(root, '[data-testid="export-name"]')) el.setAttribute("value", "");

  for (const el of findAll(root, ".approval-text")) setLead(el, "");
  for (const el of root.querySelectorAll(".beats .beat-time, .beats .beat-head b, .beats .beat-line")) el.textContent = "";
}

function applyProps(name , root , o) {
  const at = (slot) => find(root, slot, name);

  const own = PART_PROPS[name];
  if (own) own(root, o, name);
  const text = o.text ;
  if (text && typeof text === "object") for (const k of Object.keys(text)) setText(at(k), text[k]);
  const press = o.press ;
  if (press && typeof press === "object")
    for (const k of Object.keys(press)) {
      const el = at(k);
      if (el) {
        el.style.transform = `scale(${num(press[k], 1)})`;
        el.style.transformOrigin = "50% 50%";
      }
    }
  for (const k of list(o.highlight)) {
    const el = at(k);
    if (el) {
      el.style.boxShadow = "0 0 0 2px var(--select), 0 0 0 6px rgb(109 140 255 / 22%)";
      el.style.borderRadius = el.style.borderRadius || getRadius(el);
    }
  }
  for (const k of list(o.hide)) for (const el of findAll(root, k, name)) el.style.visibility = "hidden";
  for (const k of list(o.remove)) for (const el of findAll(root, k, name)) if (el !== root) el.remove();
}

function getRadius(el) {
  const view = el.ownerDocument.defaultView;
  return (view && view.getComputedStyle(el).borderRadius) || "6px";
}

function placeElements(name , root , o) {
  const frame = name === "preview" || name === "editor" ? find(root, "frame", name) : null;
  const pic = name === "editor" ? (o.preview)?.frame : o.frame;
  if (frame && pic && typeof pic === "object" && "nodeType" in pic) {
    if ((pic).parentNode !== frame) {
      const el = pic ;
      el.style.width = el.style.width || "100%";
      el.style.height = el.style.height || "100%";
      el.style.display = el.style.display || "block";
      frame.replaceChildren(el);
    }
  } else if (frame && name === "editor") {
    if (pic !== false && pic !== null) stageFootage(frame, o);
    else frame.querySelector(":scope > canvas[data-biliup-footage]")?.remove();
  }
}

function footageAt(o) {
  const tl = o.timeline ;
  const pv = o.preview ;
  const clips = (tl && Array.isArray(tl.clips) ? tl.clips : []) ;
  const ph = num(tl?.playhead, num(pv?.time, 0));
  const picture = (tr) => (typeof tr === "number" ? tr === 1 || tr === 2 : !(typeof tr === "string" && /audio|sound|music|sfx|text|caption|subtitle/i.test(tr)));
  const shots = clips.filter((c) => c && (typeof c.media === "string" || typeof c.image === "string") && picture(c.track));
  const end = (c) => num(c.start, 0) + Math.max(0.05, num(c.duration, 1));

  const hit = shots.findLast((c) => ph >= num(c.start, 0) && ph < end(c)) ?? shots.findLast((c) => ph > 0 && ph === end(c));
  if (!hit) return undefined;
  const at = Math.max(0, num(hit.in, 0) + Math.min(ph, num(hit.start, 0) + Math.max(0.05, num(hit.duration, 1))) - num(hit.start, 0));
  return typeof hit.media === "string" ? { media: hit.media, at } : { image: hit.image , at };
}

function hold(doc , p) {
  const media = (doc.defaultView)?.__biliupMedia;
  if (media && typeof media.hold === "function") media.hold(p);
}

function stageFootage(frame , o) {
  const doc = frame.ownerDocument;
  const f = footageAt(o);
  let cv = frame.querySelector(":scope > canvas[data-biliup-footage]") ;
  if (!f) {
    if (cv) cv.remove();
    return;
  }
  const w = Math.max(2, Math.round(frame.clientWidth || frame.offsetWidth || 320));
  const h = Math.max(2, Math.round(frame.clientHeight || frame.offsetHeight || 180));
  if (!cv) {
    cv = doc.createElement("canvas") ;
    cv.setAttribute("data-biliup-footage", "");
    cv.style.cssText = "display:block;width:100%;height:100%";
    frame.replaceChildren(cv);
  }

  const k = Math.min(2, 1920 / Math.max(w, h));
  const cw = Math.max(2, Math.round(w * k));
  const ch = Math.max(2, Math.round(h * k));
  const want = `${f.media ?? f.image}|${f.media ? f.at : 0}|${cw}x${ch}`;
  if (cv.__want === want) return;
  cv.__want = want;
  const target = cv;
  const draw = (img) => {
    if (target.__want !== want) return;
    target.width = cw;
    target.height = ch;
    const ctx = target.getContext("2d");
    if (!ctx) return;
    const s = Math.max(cw / img.width, ch / img.height);
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, cw, ch);
    ctx.drawImage(img, (cw - img.width * s) / 2, (ch - img.height * s) / 2, img.width * s, img.height * s);
    target.setAttribute("data-biliup-footage", "");
  };

  const clear = () => {
    if (target.__want !== want) return;
    target.width = cw;
    target.height = ch;
    target.setAttribute("data-biliup-footage", "failed");
  };
  const api = (doc.defaultView)?.biliup;
  let job ;
  if (f.media) {
    if (!api || typeof api.frame !== "function") {
      clear();
      return;
    }
    job = api.frame(f.media, f.at, { width: Math.min(1920, cw) }).then(draw, (e) => {
      clear();
      console.error(`biliupUI: the preview's clip ${f.media} at ${f.at} s couldn't be loaded: ${e instanceof Error ? e.message : String(e)}`);
    });
  } else {
    const img = doc.createElement("img");
    img.src = picture(f.image );
    job = img.decode().then(() => draw(img), () => clear());
  }
  hold(doc, job);
}

function timecode(seconds , fps) {
  const f = fps || 60;
  const total = Math.max(0, Math.round(seconds * f));
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(Math.floor(total / f / 3600))}:${pad(Math.floor(total / f / 60) % 60)}:${pad(Math.floor(total / f) % 60)}:${pad(total % f)}`;
}

const PART_PROPS = {
  composer(root, o) {
    if (o.text !== undefined && typeof o.text !== "object") {
      const value = String(o.text);
      const input = find(root, "input", "composer");
      const hl = find(root, "text", "composer");
      if (input) input.textContent = value;
      if (hl) {
        hl.textContent = `${value}\u200b`;
        chipify(hl, o.chips);
      }
      const send = find(root, "send", "composer");
      if (send) value.trim() ? send.removeAttribute("disabled") : send.setAttribute("disabled", "");
    }
    if (o.placeholder !== undefined) find(root, "input", "composer")?.setAttribute("placeholder", String(o.placeholder));
    if (o.caret) {
      const hl = find(root, "text", "composer");
      if (hl) {
        const caret = root.ownerDocument.createElement("span");
        caret.className = "biliup-ui-caret";
        const zw = hl.lastChild;
        if (zw && zw.nodeType === 3 && zw.textContent?.endsWith("\u200b")) zw.textContent = zw.textContent.slice(0, -1);
        hl.appendChild(caret);
        hl.appendChild(root.ownerDocument.createTextNode("\u200b"));
      }
    }
    if (o.context !== undefined) setText(find(root, "context", "composer")?.querySelector("span") ?? null, o.context);
  },
  message(root, o) {
    if (typeof o.text === "string") {
      root.textContent = o.text;
      chipify(root, o.chips);
    }
  },
  menu(root, o) {

    if (o.active !== undefined) {
      for (const el of findAll(root, ".ctx-item")) el.classList.remove("active");
      const id = String(o.active);
      const row = root.querySelector(`[data-testid="menu-${id}"]`) || (id ? find(root, id, "menu") : null);
      row?.classList.add("active");
    }
  },
  turn(root, o) {
    if (typeof o.reply === "string") addCaptured(root, "turn", ".reply", (el) => setText(el.querySelector("p"), o.reply));
    if (typeof o.edit === "string") addCaptured(root, "turn", ".edit-row", (el) => setText(el.querySelector(".grow"), o.edit), ".worklog");

    if (o.review)
      addCaptured(root, "turn", ".card-item", (el) => {
        blankReview(el);
        if (typeof o.review === "object") applyProps("review", el, o.review);
      }, ".edit-row, .worklog");
    if (Array.isArray(o.steps)) fillList(root.querySelector(".worklog"), ".step", o.steps.map(String), (el, s) => setText(el.querySelector(".grow"), s));

    if (typeof o.done === "number")
      [...root.querySelectorAll(".worklog .step")].forEach((el, i) => {
        const running = i >= (o.done);
        el.setAttribute("data-status", running ? "running" : "done");
        const icon = el.querySelector("svg");
        if (running && icon) icon.outerHTML = SPINNER_SVG;
      });
    if (typeof o.status === "string") setText(root.querySelector(".turn-status"), o.status);
  },
  inspector(root, o) {
    if (typeof o.title === "string") {
      setText(find(root, "title", "inspector")?.querySelector(".grow") ?? null, o.title);
      setText(root.querySelector('[data-testid="insp-media"]'), o.title);
    }
    if (typeof o.filter === "string") setText(find(root, "amount", "inspector")?.querySelector(".prop-k") ?? null, o.filter);
    if (typeof o.amount === "number") for (const el of find(root, "amount", "inspector")?.querySelectorAll("input") ?? []) el.setAttribute("value", String(Math.round(o.amount)));
    if (o.filter === false) find(root, "filter", "inspector")?.remove();
    if (o.effects === false) find(root, "effects", "inspector")?.remove();
  },
  effects(root, o) {
    if (o.active !== undefined) for (const tile of findAll(root, '[data-testid="filter-tile"]')) {
      const on = tile.getAttribute("data-look") === o.active;
      tile.classList.toggle("active", on);
      on ? tile.setAttribute("data-active", "true") : tile.removeAttribute("data-active");
    }
    if (typeof o.target === "string") setText(find(root, "target", "effects"), o.target);
  },
  storyboard(root, o) {
    if (typeof o.summary === "string") {
      const el = find(root, "summary", "storyboard");
      if (el) setLead(el, o.summary);
    }
    if (Array.isArray(o.beats)) {
      const beats = o.beats ;
      fillList(find(root, "beats", "storyboard"), ".beat", beats, (el, b, i) => {
        setText(el.querySelector(".beat-time"), `${i + 1}. ${num(b.start, 0)} s to ${num(b.end, 0)} s`);
        setText(el.querySelector(".beat-head b"), b.title ?? "");
        const lines = [...el.querySelectorAll(".beat-line")];
        lines.forEach((line, j) => (b.lines && j < b.lines.length ? (line.textContent = b.lines[j] ) : b.lines && line.remove()));
      });
    }
    if (typeof o.count === "number") [...root.querySelectorAll(".beat")].forEach((el, i) => i >= (o.count) && el.remove());
  },
  review(root, o) {
    if (typeof o.status === "string") setText(find(root, "status", "review"), o.status);
    if (typeof o.title === "string") setText(find(root, "head", "review")?.querySelector(".grow") ?? null, o.title);

    const scores = o.scores ;
    const fill = Math.max(0, Math.min(1, num(o.progress, 1)));
    if (scores && typeof scores === "object")
      for (const row of findAll(root, "score", "review")) {
        const value = scores[row.getAttribute("data-score") || ""];
        if (typeof value !== "number" || !Number.isFinite(value)) continue;
        const v = Math.max(0, Math.min(10, value));
        const bar = row.querySelector(".score-bar > span") ;
        if (bar) bar.style.width = `${fill * v * 10}%`;
        setText(row.querySelector(".score-value"), String(Math.round(v * fill)));
      }
  },
  timeline(root, o) {
    timelineProps(root, o, part("timeline").pxPerSecond || 60);
  },
  preview(root, o) {
    if (typeof o.time === "number") setLead(find(root, "timecode", "preview"), timecode(o.time));
    if (typeof o.duration === "number") setText(root.querySelector(".timecode .tc-end"), ` / ${timecode(o.duration)}`);
    if (typeof o.canvas === "string") setText(root.querySelector('[data-testid="canvas-info"]'), o.canvas);
  },
  export(root, o) {
    if (typeof o.quality === "string") {
      for (const q of ["draft", "standard", "high"]) {
        const card = find(root, q, "export");
        if (!card) continue;
        card.classList.toggle("active", q === o.quality);
        card.setAttribute("data-active", String(q === o.quality));
      }
    }
    if (typeof o.file === "string") find(root, "file", "export")?.setAttribute("value", o.file);
    if (typeof o.duration === "number") setText(root.querySelector('[data-testid="export-range-text"]'), `${timecode(0)} to ${timecode(o.duration)}, ${o.duration.toFixed(1)} s`);
    if (typeof o.motionBlur === "boolean") {
      const box = find(root, "motionBlur", "export");
      if (box) o.motionBlur ? box.setAttribute("checked", "") : box.removeAttribute("checked");
    }
  },
  exportProgress(root, o) {
    if (typeof o.progress === "number") {
      const p = Math.max(0, Math.min(1, o.progress));
      const fill = find(root, "fill", "exportProgress");
      if (fill) fill.style.width = `${p * 100}%`;
      const label = find(root, "bar", "exportProgress")?.previousElementSibling ?? null;
      if (label) setLead(label, `${p >= 1 ? "Done" : "Rendering…"} ${Math.round(p * 100)}%`);
    }
    if (typeof o.eta === "number") {
      const label = find(root, "bar", "exportProgress")?.previousElementSibling?.querySelector(".muted") ?? null;
      setText(label, `About ${Math.max(1, Math.round(o.eta))} s left`);
    }
  },
  topbar(root, o) {
    if (typeof o.exportLabel === "string") setLead(find(root, "export", "topbar"), o.exportLabel);
    if (typeof o.title === "string") setText(find(root, "title", "topbar"), o.title);
  },
  agent(root, o) {
    nested(root, o, ["composer", "turn", "message", "review"]);
  },
  editor(root, o) {
    if (Array.isArray(o.media)) {
      const grid = root.querySelector('[data-testid="media-grid"]');
      const tpl = DATA.parts.editor ? mediaTile(root.ownerDocument) : null;
      if (grid && tpl)
        for (const m of o.media) {
          const el = tpl.cloneNode(true) ;
          setText(el.querySelector(".tile-name"), m.name ?? "");
          setText(el.querySelector(".tile-sub"), m.meta ?? "");

          const badge = el.querySelector(".dur");
          if (typeof m.duration === "number") setText(badge, `${String(Math.floor(m.duration / 60)).padStart(2, "0")}:${String(Math.floor(m.duration % 60)).padStart(2, "0")}`);
          else badge?.remove();
          const thumb = el.querySelector(".thumb") ;
          if (thumb && m.image) thumb.style.cssText += `;background:url("${picture(m.image)}") center/cover`;
          grid.appendChild(el);
        }
    }
  },
};

function chipify(el , chips) {
  const labels = [...new Set(list(chips).filter(Boolean))].sort((a, b) => b.length - a.length);
  if (!labels.length) return;
  const text = el.textContent || "";
  const doc = el.ownerDocument;
  const out = [];
  let at = 0;
  let last = 0;
  while (at < text.length) {
    const hit = text[at] === "@" ? labels.find((l) => text.startsWith(l, at + 1)) : undefined;
    if (!hit) {
      at++;
      continue;
    }
    if (at > last) out.push(doc.createTextNode(text.slice(last, at)));
    const mark = doc.createElement("mark");
    mark.className = "chip";
    mark.setAttribute("data-testid", "agent-chip");
    mark.textContent = `@${hit}`;
    out.push(mark);
    at += hit.length + 1;
    last = at;
  }
  if (!out.length) return;
  if (last < text.length) out.push(doc.createTextNode(text.slice(last)));
  el.replaceChildren(...out);
}

function nested(root , o , names) {
  const sel = { composer: ".composer", turn: ".turn", message: ".msg-user", review: '.card-item[data-type="review"]', timeline: ".bottom", preview: ".stage", topbar: ".topbar" };
  for (const n of names) {
    const sub = o[n] ;
    const el = root.querySelector(sel[n] ) ;
    if (sub && typeof sub === "object" && el) applyProps(n, el, sub);
  }
}

function picture(src) {
  if (/^biliup:media\//.test(src)) throw new Error(`biliupUI: "${src}" can't load from a prop; pass await biliup.url(id, seconds) (a blob: URL, asked once before the first frame) instead`);
  return src;
}

function mediaTile(doc) {
  const tpl = doc.createElement("template");
  tpl.innerHTML = expand(part("editor").html);
  const tile = tpl.content.querySelector('[data-testid="media-tile"]');
  tile?.querySelector(".tile-used")?.remove();
  return tile;
}

function addCaptured(root , name , sel , fill , after) {
  let el = root.querySelector(sel) ;
  if (!el) {
    const tpl = root.ownerDocument.createElement("template");
    tpl.innerHTML = expand(part(name).html);
    const src = tpl.content.querySelector(sel) ;
    if (!src) return;
    el = src.cloneNode(true) ;
    const parent = (src.parentElement && root.querySelector(`.${[...src.parentElement.classList].join(".")}`)) || root;
    const anchor = after ? (parent.querySelector(after)) : null;
    anchor && anchor.parentElement === parent ? anchor.after(el) : parent.appendChild(el);
  }
  fill(el);
}

function expand(html) {
  return html.replace(/\u0001(\d+)\u0002/g, (_m, i) => DATA.pool[Number(i)] ?? "");
}

function fillList   (container , selector , entries , fill) {
  if (!container) return;
  const existing = [...container.querySelectorAll(selector)];

  let tpl = existing[0];
  let parent = tpl?.parentElement ?? null;
  if (!tpl) {
    const owner = partName(container);
    const t = container.ownerDocument.createElement("template");
    t.innerHTML = owner ? expand(part(owner).html) : "";
    tpl = t.content.querySelector(`${[...container.classList].map((c) => `.${c}`).join("")} ${selector}`) ?? undefined;
    parent = container;
  }
  if (!tpl || !parent) return;
  existing.forEach((e) => e.remove());
  entries.forEach((entry, i) => {
    const el = tpl.cloneNode(true) ;
    fill(el, entry, i);
    parent.appendChild(el);
  });
}

function timelineProps(root , o , pps) {
  const lanes = [...root.querySelectorAll(".tl-lane")] ;
  const heads = [...root.querySelectorAll(".tl-head")];
  const laneOf = (track) => {
    if (typeof track === "number") return lanes[track];
    if (typeof track === "string") {
      const i = heads.findIndex((h) => (h.querySelector(".name")?.textContent || "").trim().toLowerCase() === track.toLowerCase());
      return i >= 0 ? lanes[i] : lanes.find((l) => l.getAttribute("data-track") === track);
    }
    return lanes.find((l) => l.classList.contains("main"));
  };
  if (Array.isArray(o.clips)) {
    const clips = o.clips ;

    const templates = new Map                      ();
    const any = new Map                 ();
    const tpl0 = root.ownerDocument.createElement("template");
    tpl0.innerHTML = expand(part("timeline").html);
    const captured = [...tpl0.content.querySelectorAll(".tl-lane")];
    lanes.forEach((lane, i) => {
      const c = captured[i]?.querySelector(".clip");
      if (c) templates.set(lane, c);
      for (const k of ["video", "audio", "text"]) if (c?.classList.contains(k) && !any.has(k)) any.set(k, c);
    });

    for (const lane of lanes) for (const c of lane.querySelectorAll(".clip")) c.remove();
    for (const c of clips) {
      const lane = laneOf(c.track);
      if (!lane) continue;
      const tpl = templates.get(lane) || any.get(lane.classList.contains("main") ? "video" : "audio") || any.values().next().value;
      if (!tpl) continue;
      const el = tpl.cloneNode(true) ;
      const start = num(c.start, 0);
      const dur = Math.max(0.05, num(c.duration, 1));
      el.style.left = `${start * pps}px`;
      el.style.width = `${Math.max(2, dur * pps - 2)}px`;
      el.classList.toggle("selected", !!c.selected);
      if (c.name !== undefined) setText(el.querySelector(".nm"), c.name);
      setText(el.querySelector(".meta"), `${dur.toFixed(1)}s`);
      for (const img of el.querySelectorAll("img")) (img).style.width = "100%";
      const strip = el.querySelector(".strip") ;
      if (strip && c.image) {
        strip.style.backgroundImage = `url("${picture(c.image)}")`;
        strip.style.backgroundSize = "auto 100%";
        strip.style.backgroundRepeat = "repeat-x";
      }
      if (c.color) el.style.background = c.color;
      lane.appendChild(el);
    }
  }
  for (const i of list(o.selected)) root.querySelectorAll(".clip")[Number(i)]?.classList.add("selected");
  if (typeof o.playhead === "number") {
    const ph = root.querySelector(".playhead") ;
    if (ph) ph.style.left = `${o.playhead * pps}px`;
  }
}

const LINE_GROW = 0.22;

function typedCount(k , t) {
  const len = String(k.props?.text ?? "").length;
  const from = Math.max(0, Math.min(len, Math.round(num(k.typing?.from, 0))));
  const dur = num(k.typing?.dur, 0);
  const at = num(k.typing?.at, 0);
  const p = dur > 0 ? Math.max(0, Math.min(1, (t - at) / dur)) : t >= at ? 1 : 0;
  return Math.round(from + (len - from) * p);
}

function typedTime(k , n) {
  const len = String(k.props?.text ?? "").length;
  const from = Math.max(0, Math.min(len, Math.round(num(k.typing?.from, 0))));
  const dur = num(k.typing?.dur, 0);
  return num(k.typing?.at, 0) + (len > from && dur > 0 ? dur * Math.max(0, (n - 0.5 - from) / (len - from)) : 0);
}

function typedHeight(host , k , n) {
  let m = host.querySelector(":scope > .biliup-ui-measure") ;
  if (!m) {
    m = host.ownerDocument.createElement("div") ;
    m.className = "biliup-ui-measure";
    m.style.cssText = "position:absolute;left:0;top:0;width:0;height:0;overflow:visible;visibility:hidden;pointer-events:none";
    host.appendChild(m);
  }
  const key = { ...k, at: 0, typing: undefined, props: { ...(k.props || {}), text: String(k.props?.text ?? "").slice(0, n) } };
  const id = keyOf(key);
  m.__heights ??= new Map();
  const known = m.__heights.get(id);
  if (known !== undefined) return known;
  const b = buildPiece(m, key);
  const h = b.h - 2 * b.pad;
  m.__heights.set(id, h);
  return h;
}

function typedKey(k , t , host) {
  if (!k.typing || typeof k.props?.text !== "string") return k;
  const n = typedCount(k, t);
  if (typeof k.size?.height === "number") return { ...k, props: { ...k.props, text: k.props.text.slice(0, n) } };
  const n0 = typedCount(k, t - LINE_GROW);
  const h = (c) => typedHeight(host, k, c);
  let height = h(n0);
  const wraps = [];
  const split = (lo , hi , hl , hh) => {
    if (Math.abs(hl - hh) < 0.01) return;
    if (hi - lo <= 1) return void wraps.push(hi);
    const mid = (lo + hi) >> 1;
    const hm = h(mid);
    split(lo, mid, hl, hm);
    split(mid, hi, hm, hh);
  };
  if (n > n0) split(n0, n, height, h(n));
  for (const c of wraps) height += (h(c) - h(c - 1)) * settle((t - typedTime(k, c)) / LINE_GROW);
  return { ...k, props: { ...k.props, text: k.props.text.slice(0, n) }, size: { ...(k.size || {}), height } };
}

const PIECE_SHELL = "display:contents;";

function rgba(c) {
  const hex = /^#([0-9a-f]{6})$/i.exec(c.trim());
  if (hex) {
    const n = parseInt(hex[1] , 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const m = c.match(/[\d.]+/g);
  if (!m || m.length < 3 || !/^rgb/.test(c.trim())) return undefined;
  return [Number(m[0]), Number(m[1]), Number(m[2]), m.length >= 4 ? Number(m[3]) : 1];
}

function mixRgba(a , b , p) {
  const x = rgba(a) || [0, 0, 0, 0];
  const y = rgba(b) || [0, 0, 0, 0];
  const v = x.map((n, i) => n + (y[i]  - n) * p);
  return `rgba(${Math.round(v[0] )}, ${Math.round(v[1] )}, ${Math.round(v[2] )}, ${Math.round(v[3]  * 1000) / 1000})`;
}

function settle(p) {
  if (p <= 0) return 0;
  if (p >= 1) return 1;
  return p * p * (6 - 8 * p + 3 * p * p);
}

const smooth = (p) => {
  const q = Math.max(0, Math.min(1, p));
  return q * q * (3 - 2 * q);
};

function resolveKey(k , host) {

  const view = host.ownerDocument.defaultView;
  const W = host.offsetWidth || host.ownerDocument.documentElement.clientWidth || view?.innerWidth || 1080;
  const H = host.offsetHeight || host.ownerDocument.documentElement.clientHeight || view?.innerHeight || 1350;
  const own = typeof k.size?.width === "number";
  const width = typeof k.width === "number" ? k.width : typeof k.scale === "number" || own ? undefined : W * 0.86;
  const type = own || covers(k) || width === undefined ? 0 : num(k.type, H * 0.048);
  return { ...k, x: num(k.x, W / 2), y: num(k.y, H / 2), width, type };
}

function buildPiece(holder , k) {
  const [name, slot] = [k.piece.split(".")[0] , k.piece.split(".").slice(1).join(".")];
  const props = k.props || {};
  const typed = num(k.type, 0) > 0 && typeof k.width === "number";
  const key = `${k.piece}\n${k.index ?? 0}\n${k.pad ?? ""}\n${k.radius ?? ""}\n${k.background ?? ""}\n${k.theme ?? ""}\n${k.color ?? ""}\n${typed ? `${k.width}/${k.type}` : ""}\n${keyOf(k.size ?? {})}\n${keyOf(props)}`;
  if (holder.__pieceKey === key && holder.__built) return holder.__built;
  const scratch = holder.ownerDocument.createElement("div");
  scratch.style.cssText = "position:absolute;left:0;top:0;visibility:hidden;pointer-events:none";
  holder.replaceChildren(scratch);
  const root = mount(scratch, name, { ...props, background: "none", scale: 1 });
  const wrap = scratch.firstElementChild ;
  const theme = k.theme === "light";
  wrap.classList.toggle("biliup-ui-light", theme);
  const el = slot ? findAll(root, slot, name)[k.index ?? 0] : root;
  if (!el) throw new Error(`biliupUI: no piece "${k.piece}"${k.index ? ` #${k.index}` : ""} (slots of ${name}: ${slots(name).join(", ")})`);
  const view = holder.ownerDocument.defaultView ;
  const cs = view.getComputedStyle(el);

  const ownBg = cs.backgroundColor;
  const ownRadius = parseFloat(cs.borderTopLeftRadius) || 0;

  let lead = el;
  for (let n = el; n; n = nextEl(n, el))
    if ([...n.childNodes].some((c) => c.nodeType === 3 && /\S/.test(c.textContent || ""))) {
      lead = n;
      break;
    }
  const font = parseFloat(view.getComputedStyle(lead).fontSize) || 13;
  const own = rgba(ownBg);

  const opaque = !!own && own[3] >= 0.99 && ownRadius > 0;
  let behind = own && own[3] >= 0.99 ? ownBg : theme ? "rgb(255, 255, 255)" : part(name).backdrop || "rgb(10, 10, 10)";
  if (!(own && own[3] >= 0.99))
    for (let p = el.parentElement; p && p !== scratch; p = p.parentElement) {
      const c = rgba(view.getComputedStyle(p).backgroundColor);
      if (c && c[3] >= 0.99) {
        behind = view.getComputedStyle(p).backgroundColor;
        break;
      }
    }

  const line = parseFloat(view.getComputedStyle(lead).lineHeight) || font * 1.4;
  const radius = num(k.radius, opaque ? ownRadius : Math.min(18, Math.max(10, line * 0.6)));
  const pad = num(k.pad, opaque ? 0 : Math.round(Math.min(18, Math.max(10, line * 0.6))));

  const w = typed ? Math.max(48, Math.round((k.width  * font) / k.type  - pad * 2)) : num(k.size?.width, el.offsetWidth);
  let h = num(k.size?.height, el.offsetHeight);

  for (let at = el; at !== wrap; ) {
    const parent = at.parentElement ;
    for (const sib of [...parent.children]) if (sib !== at) sib.remove();
    if (parent.getAttribute("style")?.startsWith(SHELL)) parent.setAttribute("style", PIECE_SHELL + (parent.getAttribute("style") || "").slice(SHELL.length));
    else parent.style.display = "contents";
    at = parent;
  }
  wrap.style.cssText = "display:contents";
  const st = el.style;
  st.boxSizing = "border-box";
  st.width = `${w}px`;
  st.height = (typed || typeof k.size?.width === "number") && typeof k.size?.height !== "number" ? "auto" : `${h}px`;
  st.margin = "0";
  st.position = "relative";
  st.left = st.top = st.right = st.bottom = "auto";
  st.flex = "none";
  st.maxWidth = st.maxHeight = "none";
  if (opaque) st.boxShadow = "none";
  if (k.color) st.color = k.color;
  if (st.height === "auto") {

    const r = el.getBoundingClientRect();
    h = el.offsetHeight > 2 ? (r.width > 0 && el.offsetWidth > 0 ? (r.height * el.offsetWidth) / r.width : el.offsetHeight) : h;
    st.height = `${h}px`;
  }

  const typedText = k.typing && typeof props.text === "string" ? props.text : "";
  if (typedText && typeof k.size?.height === "number") {
    st.height = "auto";
    const natural = el.offsetHeight;
    const box = [...el.children].find((c) => (c.textContent || "").includes(typedText)) ;
    const short = natural - h;
    if (box && short > 0.01) {
      const bs = box.style;
      bs.height = `${Math.max(0, box.offsetHeight - short)}px`;
      bs.minHeight = "0";
      bs.overflow = "hidden";
      bs.flex = "none";
    }
    st.height = `${h}px`;
  }
  const node = holder.ownerDocument.createElement("div");
  node.className = "biliup-ui-piece";
  node.style.cssText = `position:absolute;left:0;top:0;width:${w + pad * 2}px;height:${h + pad * 2}px;padding:${pad}px;box-sizing:border-box;transform-origin:0 0`;
  node.appendChild(wrap);
  holder.replaceChildren(node);
  const built = { node, el, w: w + pad * 2, h: h + pad * 2, bg: k.background ?? (opaque ? ownBg : behind), radius, font, name, pad };
  holder.__pieceKey = key;
  holder.__built = built;
  return built;
}

function nextEl(n , root) {
  if (n.firstElementChild) return n.firstElementChild;
  for (let at = n; at && at !== root; at = at.parentElement) if (at.nextElementSibling) return at.nextElementSibling;
  return null;
}

function keyBox(k , b) {
  const s = typeof k.width === "number" ? k.width / b.w : num(k.scale, 1);
  const w = typeof k.width === "number" ? k.width : b.w * s;
  const h = typeof k.height === "number" ? k.height : b.h * s;
  return { x: typeof k.x === "number" ? k.x : w / 2, y: typeof k.y === "number" ? k.y : h / 2, w, h, r: b.radius * s };
}

const fitScale = (b , w , h , cover) => (cover ? Math.max(w / b.w, h / b.h) : Math.min(w / b.w, h / b.h));

function fitInto(b , w , h , alpha , blur , dx , dy , cover , z) {
  const s = fitScale(b, w, h, cover) * num(z, 1);
  const st = b.node.style;
  st.transform = `translate(${(w - b.w * s) / 2 + dx}px, ${(h - b.h * s) / 2 + dy}px) scale(${s})`;
  st.opacity = String(Math.round(alpha * 1000) / 1000);
  st.filter = blur > 0.05 ? `blur(${(blur / Math.max(s, 0.01)).toFixed(2)}px)` : "none";
  st.visibility = alpha > 0.001 ? "visible" : "hidden";
  st.maskImage = "";
}

const covers = (k) => (k.fit ? k.fit === "cover" : /\.frame$/.test(k.piece));

const DIRS = { up: [0, -1], down: [0, 1], left: [-1, 0], right: [1, 0], none: [0, 0] };

function localRect(el , node) {
  const a = el.getBoundingClientRect();
  const b = node.getBoundingClientRect();
  const s = node.offsetWidth ? b.width / node.offsetWidth : 1;
  return { x: (a.left - b.left) / s, y: (a.top - b.top) / s, w: a.width / s, h: a.height / s, r: 0 };
}

function restAt(b , box , cover) {
  const s = fitScale(b, box.w, box.h, cover);
  return { s, x: box.x - box.w / 2 + (box.w - b.w * s) / 2, y: box.y - box.h / 2 + (box.h - b.h * s) / 2 };
}

function carry(host , kf , kj , bf , bj , boxF , boxJ , p , raw , hidden) {
  const doc = host.ownerDocument;
  let layer = host.querySelector(":scope > .biliup-ui-carry") ;
  if (!layer) {
    layer = doc.createElement("div") ;
    layer.className = "biliup-ui-carry";
    host.appendChild(layer);
  }
  const pairs = (kj.carry || []).map((c) => (typeof c === "string" ? [c, c] : c));
  const found = pairs.map(([a, b]) => [find(bf.el, a, bf.name), find(bj.el, b, bj.name)]).filter((x) => !!x[0] && !!x[1]);
  if (!found.length || raw >= 1) {
    layer.style.display = "none";
    return;
  }
  layer.style.cssText = "position:absolute;left:0;top:0;width:0;height:0;overflow:visible;pointer-events:none;z-index:2";
  const spec = JSON.stringify(pairs);
  if (layer.__from !== bf || layer.__to !== bj || layer.__spec !== spec) {
    layer.replaceChildren();
    found.forEach(([fe, te], i) => {
      for (const [b, e] of [[bf, fe], [bj, te]]) {
        e.setAttribute("data-biliup-carry", String(i));
        const c = b.node.cloneNode(true) ;
        e.removeAttribute("data-biliup-carry");
        c.querySelector(`[data-biliup-carry="${i}"]`)?.setAttribute("style", `${c.querySelector(`[data-biliup-carry="${i}"]`)?.getAttribute("style") || ""};visibility:visible`);
        layer .appendChild(c);
      }
    });
    layer.__from = bf;
    layer.__to = bj;
    layer.__spec = spec;
  }
  const rf = restAt(bf, boxF, covers(kf));
  const rj = restAt(bj, boxJ, covers(kj));
  const hand = smooth((raw - 0.4) / 0.12);
  found.forEach(([fe, te], i) => {
    const lf = localRect(fe, bf.node);
    const lj = localRect(te, bj.node);
    const A = { x: rf.x + lf.x * rf.s, y: rf.y + lf.y * rf.s, w: lf.w * rf.s };
    const B = { x: rj.x + lj.x * rj.s, y: rj.y + lj.y * rj.s, w: lj.w * rj.s };
    const cur = { x: A.x + (B.x - A.x) * p, y: A.y + (B.y - A.y) * p, w: A.w + (B.w - A.w) * p };
    const sides = [[lf, (rf.s * cur.w) / Math.max(A.w, 0.01), 1 - hand], [lj, (rj.s * cur.w) / Math.max(B.w, 0.01), hand]];
    sides.forEach(([l, s, alpha], side) => {
      const c = layer .children[i * 2 + side] ;
      c.style.transform = `translate(${cur.x - l.x * s}px, ${cur.y - l.y * s}px) scale(${s})`;
      c.style.opacity = String(Math.round(alpha * 1000) / 1000);
      c.style.filter = "none";
      c.style.visibility = "hidden";
    });
    for (const e of [fe, te]) {
      e.style.visibility = "hidden";
      hidden.push(e);
    }
  });
  layer.style.display = "block";
}

function startOf(i , keys , holders , dur , depth = 0) {
  if (i < 1 || depth > 8) return null;
  const at = num(keys[i] .at, 0);
  const raw = (num(keys[i + 1] .at, 0) - at) / dur(keys[i] );
  if (raw >= 1) return null;
  const q = settle(raw);
  const built = (n) => {
    const hd = holders[n] ;
    const shown = hd.style.display;
    hd.style.display = "block";
    const b = buildPiece(hd, keys[n] );
    hd.style.display = shown;
    return b;
  };
  const bi = built(i);
  const rest = keyBox(keys[i] , bi);
  const prev = startOf(i - 1, keys, holders, dur, depth + 1);
  const bp = built(i - 1);
  const a = prev ? prev.box : keyBox(keys[i - 1] , bp);
  const abg = prev ? prev.bg : bp.bg;
  const pa = prev ? prev.at : placeOf(bp, a, covers(keys[i - 1] ));
  const pr = placeOf(bi, rest, covers(keys[i] ));
  return { box: { x: a.x + (rest.x - a.x) * q, y: a.y + (rest.y - a.y) * q, w: a.w + (rest.w - a.w) * q, h: a.h + (rest.h - a.h) * q, r: a.r + (rest.r - a.r) * q }, bg: mixRgba(abg, bi.bg, q), at: { s: pa.s + (pr.s - pa.s) * q, y: pa.y + (pr.y - pa.y) * q } };
}

function placeOf(b , box , cover) {
  const s = fitScale(b, box.w, box.h, cover);
  return { s, y: (box.h - b.h * s) / 2 };
}

function sequence(host , t , keyList , o) {
  if (!keyList.length) throw new Error("biliupUI.sequence: no keys");
  const doc = host.ownerDocument;
  ensureCss(doc);
  const keys = keyList.map((k) => typedKey(resolveKey(k, host), t, host));
  let shell = host.querySelector(":scope > .biliup-ui-shell") ;
  if (!shell) {
    shell = doc.createElement("div") ;
    shell.className = "biliup-ui-shell";
    host.insertBefore(shell, host.querySelector(":scope > .biliup-ui-carry"));
  }
  for (const e of shell.__hidden || []) e.style.visibility = "";
  const hidden = (shell.__hidden = []);
  const holders = [];
  for (let i = 0; i < keys.length; i++) {
    let hd = shell.children[i] ;
    if (!hd) {
      hd = doc.createElement("div") ;
      hd.style.cssText = "position:absolute;left:0;top:0";
      shell.appendChild(hd);
    }
    holders.push(hd);
  }
  while (shell.children.length > keys.length) shell.lastElementChild .remove();
  let j = 0;
  for (let i = 0; i < keys.length; i++) if (num(keys[i] .at, 0) <= t) j = i;
  const dur = (k) => Math.max(0.05, num(k.dur, num(o?.dur, 0.9)));
  const from = j > 0 && t < num(keys[j] .at, 0) + dur(keys[j] ) ? j - 1 : -1;
  const raw = from >= 0 ? (t - num(keys[j] .at, 0)) / dur(keys[j] ) : 1;
  const p = settle(raw);

  holders.forEach((hd, i) => {
    hd.style.display = i === j || i === from ? "block" : "none";
    hd.style.zIndex = "";
  });

  holders.forEach((hd, i) => {
    hd.setAttribute("data-biliup-piece", keys[i] .piece);
    hd.setAttribute("data-biliup-at", String(num(keys[i] .at, 0)));
    hd.setAttribute("data-biliup-dur", String(dur(keys[i] )));
    hd.setAttribute("data-biliup-swap", from < 0 ? "" : i === j ? "in" : i === from ? "out" : "");
  });
  const bj = buildPiece(holders[j] , keys[j] );
  const boxJ = keyBox(keys[j] , bj);
  let box = boxJ;
  let bg = bj.bg;
  let press = num(keys[j] .press, 1);
  let carried = false;
  if (from >= 0) {
    const kf = keys[from] ;
    const kj = keys[j] ;
    const bf = buildPiece(holders[from] , kf);
    const boxF = keyBox(kf, bf);

    const start = startOf(from, keys, holders, dur);
    const b0 = start ? start.box : boxF;
    box = { x: b0.x + (boxJ.x - b0.x) * p, y: b0.y + (boxJ.y - b0.y) * p, w: b0.w + (boxJ.w - b0.w) * p, h: b0.h + (boxJ.h - b0.h) * p, r: b0.r + (boxJ.r - b0.r) * p };
    bg = mixRgba(start ? start.bg : bf.bg, bj.bg, p);
    press = num(kf.press, 1) + (press - num(kf.press, 1)) * p;
    const same = kf.piece === kj.piece && (kf.index ?? 0) === (kj.index ?? 0) && kf.theme === kj.theme;
    holders[from] .style.display = same ? "none" : "block";
    if (!same) {

      const [dx, dy] = DIRS[kj.from || "down"] || DIRS.down ;
      const out = smooth((raw - 0.04) / 0.3);
      const go = out * out;
      const blurOut = 14 * smooth(raw / 0.3);
      if (covers(kf)) {
        const rf = restAt(bf, boxF, true);
        const st = bf.node.style;
        const fade = smooth((raw - 0.3) / 0.35);
        st.transform = `translate(${rf.x - (box.x - box.w / 2)}px, ${rf.y - (box.y - box.h / 2)}px) scale(${rf.s})`;
        st.opacity = String(Math.round((1 - fade) * 1000) / 1000);
        st.filter = "none";
        st.visibility = fade < 0.999 ? "visible" : "hidden";
        st.maskImage = "";
      } else fitInto(bf, box.w, box.h, 1 - out, blurOut, -dx * 0.35 * box.w * go, -dy * 0.35 * box.h * go, false, 1 - 0.06 * out);
      const pic = covers(kj);
      const inn = pic ? smooth((raw - 0.08) / 0.27) : smooth((raw - 0.18) / 0.3);
      const arrive = 1 - settle(Math.max(0, (raw - 0.08) / 0.92));
      fitInto(bj, box.w, box.h, inn, (pic ? 8 : 14) * (1 - smooth((raw - (pic ? 0.1 : 0.16)) / 0.45)), dx * 0.35 * box.w * arrive, dy * 0.35 * box.h * arrive, covers(kj), 1 + 0.05 * arrive);
      if (kj.carry?.length) {
        carry(host, kf, kj, bf, bj, boxF, boxJ, p, raw, hidden);
        carried = true;
      }
    } else {

      const pf = start ? start.at : placeOf(bf, boxF, covers(kf));
      const pj = placeOf(bj, boxJ, covers(kj));
      const s = pf.s + (pj.s - pf.s) * p;
      const oy = pf.y + (pj.y - pf.y) * p;
      const place = (b , sc) => {
        const st = b.node.style;
        st.transform = `translate(${(box.w - b.w * sc) / 2}px, ${oy}px) scale(${sc})`;
        st.filter = "none";
        st.visibility = "visible";
      };
      place(bj, s);
      const st = bj.node.style;
      st.opacity = "1";
      const end = (box.h - oy) / s - bj.pad;
      const old = Math.max(0, bf.h - bf.pad);
      const grew = bj.h > bf.h + 0.5 && Math.abs(bj.w - bf.w) < 0.5;
      const a = grew ? smooth((raw - 0.1) / 0.45) : 1;
      const cut = bj.pad > 0 && end < bj.h - bj.pad - 0.5;
      const stops = [];
      if (a < 0.999) stops.push(`#000 ${old.toFixed(2)}px`, `rgb(0 0 0 / ${a.toFixed(3)}) ${(old + 1).toFixed(2)}px`);
      if (cut) stops.push(`rgb(0 0 0 / ${a.toFixed(3)}) ${Math.max(end, old + 1).toFixed(2)}px`, `transparent ${(Math.max(end, old + 1) + bj.pad / 2).toFixed(2)}px`);
      st.maskImage = stops.length ? `linear-gradient(to bottom, ${a < 0.999 ? "" : "#000 0px, "}${stops.join(", ")})` : "";

      const fade = Math.abs(bj.w - bf.w) < 0.5 ? smooth(raw / 0.45) : 1;
      const hf = holders[from] ;
      if (fade < 0.999) {
        hf.style.display = "block";
        hf.style.zIndex = "1";
        place(bf, s * (bj.w / bf.w));
        bf.node.style.opacity = String(Math.round((1 - fade) * 1000) / 1000);
        bf.node.style.maskImage = bf.pad > 0 ? `linear-gradient(to bottom, #000 ${old.toFixed(2)}px, transparent ${(old + bf.pad / 2).toFixed(2)}px)` : "";
      }
    }
  } else fitInto(bj, box.w, box.h, 1, 0, 0, 0, covers(keys[j] ));
  if (!carried) {
    const layer = host.querySelector(":scope > .biliup-ui-carry") ;
    if (layer) layer.style.display = "none";
  }
  shell.style.cssText = `position:absolute;left:${box.x - box.w / 2}px;top:${box.y - box.h / 2}px;width:${box.w}px;height:${box.h}px;border-radius:${Math.min(box.r, box.w / 2, box.h / 2)}px;background:${bg};overflow:hidden;transform:scale(${press});transform-origin:50% 50%${keys[j] .shadow === false || (from >= 0 && keys[from] .shadow === false && raw < 0.6) ? "" : ";box-shadow:0 1px 2px rgb(0 0 0 / 8%), 0 14px 40px rgb(0 0 0 / 14%)"}`;
  return { x: box.x, y: box.y, w: box.w, h: box.h, r: box.r, p: from >= 0 ? p : 1, key: j, el: bj.el, type: (bj.font * boxJ.w) / bj.w };
}

function snapshot(el) {
  const view = el.ownerDocument.defaultView ;
  const copy = el.cloneNode(true) ;
  const src = [el, ...el.querySelectorAll("*")];
  const dst = [copy, ...copy.querySelectorAll("*")];
  src.forEach((s, i) => {
    const cs = view.getComputedStyle(s);
    let css = "";
    for (let j = 0; j < cs.length; j++) {
      const k = cs[j] ;
      if (k === "visibility" || k.startsWith("transition") || k.startsWith("animation")) continue;
      css += `${k}:${cs.getPropertyValue(k)};`;
    }
    dst[i] .setAttribute("style", css);
  });
  copy.style.position = "absolute";
  copy.style.left = copy.style.top = "0px";
  copy.style.margin = "0";
  copy.style.transform = "none";
  copy.style.transformOrigin = "0 0";
  copy.style.opacity = "1";
  copy.style.boxShadow = "none";
  copy.style.width = `${el.offsetWidth}px`;
  copy.style.height = `${el.offsetHeight}px`;

  copy.style.maxWidth = copy.style.maxHeight = "none";
  copy.style.minWidth = copy.style.minHeight = "0";
  copy.style.boxSizing = "border-box";
  copy.style.visibility = "visible";
  return copy;
}

const isPieceKey = (v) => !!v && typeof v === "object" && typeof (v).piece === "string";
const isElement = (v) => !!v && typeof v === "object" && "nodeType" in (v);

function fly(host , t , f) {
  const doc = host.ownerDocument;
  ensureCss(doc);
  const id = f.id ?? "0";
  let layer = [...host.children].find((c) => c.classList.contains("biliup-ui-fly") && c.getAttribute("data-fly") === id) ;
  if (!layer) {
    layer = doc.createElement("div") ;
    layer.className = "biliup-ui-fly";
    layer.setAttribute("data-fly", id);
    layer.innerHTML = '<div class="biliup-ui-fly-box"><div class="biliup-ui-fly-a"></div><div class="biliup-ui-fly-b"></div></div>';
    host.appendChild(layer);
  }
  layer.__snaps ??= new Map();
  layer.style.cssText = "position:absolute;left:0;top:0;width:0;height:0;overflow:visible;pointer-events:none;z-index:50";
  const box = layer.firstElementChild ;
  const slotA = box.children[0] ;
  const slotB = box.children[1] ;
  const dur = Math.max(0.05, num(f.dur, 0.9));
  const raw = (t - f.at) / dur;
  const to = isElement(f.to) ? f.to : null;
  const from = isElement(f.from) ? f.from : null;

  if (to) {
    if (raw < 1 && (raw >= 0 || isPieceKey(f.from))) to.setAttribute("data-biliup-fly-hidden", "");
    else to.removeAttribute("data-biliup-fly-hidden");
  }
  if (from && f.leave) from.style.visibility = raw >= 0 ? "hidden" : "";

  if (raw >= 1 || !f.to || !f.from || (raw < 0 && !isPieceKey(f.from))) {
    layer.style.display = "none";
    return { x: 0, y: 0, w: 0, h: 0, p: raw >= 1 ? 1 : 0, el: null };
  }

  let A ;
  let built = null;
  let press = 1;
  if (isPieceKey(f.from)) {
    const k = typedKey(resolveKey(f.from, host), t, host);
    built = buildPiece(slotA, k);
    const b = keyBox(k, built);
    A = { x: b.x - b.w / 2, y: b.y - b.h / 2, w: b.w, h: b.h };
    press = num(k.press, 1);
  } else if (from) A = rectIn(from, host);
  else A = f.from ;
  const B = to ? rectIn(to, host) : (f.to);

  if ((to && !(B.w > 0 && B.h > 0)) || (from && !(A.w > 0 && A.h > 0))) {
    layer.style.display = "none";
    return { x: 0, y: 0, w: 0, h: 0, p: 0, el: null };
  }

  if (raw < 0 && !built) {
    layer.style.display = "none";
    return { x: A.x + A.w / 2, y: A.y + A.h / 2, w: A.w, h: A.h, p: 0, el: null };
  }
  layer.style.display = "block";
  const q = Math.max(0, raw);
  const p = settle(q);
  const arc = num(f.arc, 0) * 4 * p * (1 - p);
  const cur = { x: A.x + (B.x - A.x) * p, y: A.y + (B.y - A.y) * p + arc, w: A.w + (B.w - A.w) * p, h: A.h + (B.h - A.h) * p };

  const snap = (slot , el) => {
    const key = `${el.outerHTML.length}:${el.offsetWidth}x${el.offsetHeight}:${el.outerHTML.slice(0, 400)}:${el.textContent}`;
    let s = layer .__snaps .get(slot);
    if (!s || s.key !== key) {
      const view = doc.defaultView ;
      const cs = view.getComputedStyle(el);
      const node = snapshot(el);
      node.removeAttribute("data-biliup-fly-hidden");
      node.style.backgroundColor = "transparent";
      node.style.borderRadius = "0";
      node.style.border = "0";
      s = { key, node, w: el.offsetWidth || 1, h: el.offsetHeight || 1, bg: cs.backgroundColor, r: parseFloat(cs.borderTopLeftRadius) || 0 };
      layer .__snaps .set(slot, s);
    }
    return s;
  };
  const sb = to ? snap("b", to) : null;
  if (sb && slotB.firstElementChild !== sb.node) slotB.replaceChildren(sb.node);
  if (!sb) slotB.replaceChildren();
  let rA = 0;
  let bgA = "rgba(0, 0, 0, 0)";
  if (built) {
    rA = (built.radius * A.w) / built.w;
    bgA = built.bg;
  } else if (from) {
    const sa = snap("a", from);
    if (slotA.firstElementChild !== sa.node) slotA.replaceChildren(sa.node);
    rA = (sa.r * A.w) / sa.w;
    bgA = sa.bg;
  } else slotA.replaceChildren();
  const rB = sb ? (sb.r * B.w) / sb.w : rA;
  const bgB = sb ? sb.bg : bgA;

  const words = f.fit === "contain";
  const inB = words ? smooth((q - 0.34) / 0.16) : smooth((q - 0.2) / 0.25);
  const outA = words ? smooth((q - 0.18) / 0.16) : smooth((q - 0.32) / 0.18);
  const place = (node , w , h , alpha , cover) => {
    const s = cover ? Math.max(cur.w / w, cur.h / h) : Math.min(cur.w / w, cur.h / h);
    node.style.transformOrigin = "0 0";
    node.style.transform = `translate(${(cur.w - w * s) / 2}px, ${(cur.h - h * s) / 2}px) scale(${s})`;
    node.style.opacity = String(Math.round(alpha * 1000) / 1000);
    node.style.visibility = alpha > 0.001 ? "visible" : "hidden";
  };
  slotA.style.cssText = slotB.style.cssText = "position:absolute;left:0;top:0;width:0;height:0;overflow:visible";
  if (built) {
    built.node.style.position = "absolute";
    place(built.node, built.w, built.h, raw < 0 ? 1 : 1 - outA, false);
  } else if (slotA.firstElementChild) {
    const sa = layer.__snaps.get("a") ;
    place(sa.node, sa.w, sa.h, 1 - outA, f.fit !== "contain");
  }
  if (sb) place(sb.node, sb.w, sb.h, raw < 0 ? 0 : inB, f.fit !== "contain");
  const bg = mixRgba(bgA, bgB, raw < 0 ? 0 : inB);
  const r = rA + (rB - rA) * p;
  const lift = raw < 0 ? (built && f.from && (f.from).shadow !== false ? 1 : 0) : Math.sin(Math.PI * Math.min(1, q * 1.4)) * (1 - p) + (built ? 1 - p : 0);
  const shadow = lift > 0.01 ? `;box-shadow:0 ${(2 + 22 * lift).toFixed(1)}px ${(6 + 50 * lift).toFixed(1)}px rgb(0 0 0 / ${(0.1 + 0.22 * Math.min(1, lift)).toFixed(3)})` : "";
  const pz = raw < 0 ? press : 1;
  box.style.cssText = `position:absolute;left:${cur.x}px;top:${cur.y}px;width:${cur.w}px;height:${cur.h}px;border-radius:${Math.min(r, cur.w / 2, cur.h / 2)}px;background:${bg};overflow:hidden;transform:scale(${pz});transform-origin:50% 50%${shadow}`;
  return { x: cur.x + cur.w / 2, y: cur.y + cur.h / 2, w: cur.w, h: cur.h, p: raw < 0 ? 0 : p, el: built?.el ?? null };
}

function piece(host , id , props) {
  return sequence(host, 0, [{ ...(props || {}), piece: id, at: 0 }]).el;
}

function frameText(target , host , o) {
  const W = num(o?.width, 1080);
  const H = num(o?.height, 1350);
  const want = num(o?.text, 0.048);
  const safe = Math.max(0, Math.min(num(o?.safe, 72), W / 2 - 1, H / 2 - 1));
  const pad = num(o?.pad, 8);

  const els = (isElement(target) ? [target] : [...(target)]).filter(Boolean);
  const doc = host.ownerDocument;
  const view = doc.defaultView;
  const hb = host.getBoundingClientRect();
  const hs = host.offsetWidth ? hb.width / host.offsetWidth : 1;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  const grow = (r) => {
    if (!r.width || !r.height) return;
    x0 = Math.min(x0, (r.left - hb.left) / hs);
    y0 = Math.min(y0, (r.top - hb.top) / hs);
    x1 = Math.max(x1, (r.right - hb.left) / hs);
    y1 = Math.max(y1, (r.bottom - hb.top) / hs);
  };

  const sizes = [];
  for (const el of els) {
    const walk = doc.createTreeWalker(el, 4 /* NodeFilter.SHOW_TEXT */);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const text = (n.nodeValue || "").trim();
      const p = n.parentElement;
      if (!text || !p) continue;
      const cs = view ? view.getComputedStyle(p) : null;
      if (cs && (cs.visibility === "hidden" || cs.display === "none" || cs.opacity === "0")) continue;
      const range = doc.createRange();
      range.selectNodeContents(n);
      const rects = range.getClientRects();
      if (!rects.length) continue;
      for (let i = 0; i < rects.length; i++) grow(rects[i] );
      const pr = p.getBoundingClientRect();
      const k = p.offsetWidth ? pr.width / p.offsetWidth : 1;
      sizes.push([(parseFloat(cs ? cs.fontSize : "12") || 12) * (k / hs), text.length]);
    }
    el.querySelectorAll("svg").forEach((s) => grow(s.getBoundingClientRect()));
  }
  if (!sizes.length) {
    for (const el of els) grow(el.getBoundingClientRect());
    sizes.push([12, 1]);
  }
  if (!Number.isFinite(x0)) return { x: 0, y: 0, zoom: 1, text: 0, fits: false };
  sizes.sort((a, b) => a[0] - b[0]);
  const total = sizes.reduce((s, x) => s + x[1], 0);
  let acc = 0;
  let font = sizes[sizes.length - 1] [0];
  for (const [px, n] of sizes) if ((acc += n) >= total / 2) {
    font = px;
    break;
  }
  const bw = x1 - x0 + 2 * pad, bh = y1 - y0 + 2 * pad;
  const zText = (want * H) / Math.max(1, font);
  const zFit = Math.min((W - 2 * safe) / Math.max(1, bw), (H - 2 * safe) / Math.max(1, bh));
  const zoom = Math.min(zText, zFit);
  return { x: (x0 + x1) / 2, y: (y0 + y1) / 2, zoom, text: Math.round(((font * zoom) / H) * 10000) / 10000, fits: zText <= zFit + 1e-9 };
}

function rectIn(el , host) {
  const a = el.getBoundingClientRect();
  const b = host.getBoundingClientRect();
  const s = host.offsetWidth ? b.width / host.offsetWidth : 1;
  return { x: (a.left - b.left) / s, y: (a.top - b.top) / s, w: a.width / s, h: a.height / s };
}

function cursor(host , o) {
  const doc = host.ownerDocument;
  let el = host.querySelector(":scope > .biliup-ui-cursor") ;
  if (!el) {
    el = doc.createElement("div");
    el.className = "biliup-ui-cursor";
    el.innerHTML = CURSOR_SVG;
    host.appendChild(el);
  }
  const size = num(o.size, 28);
  el.style.cssText = `position:absolute;left:0;top:0;width:${size}px;height:${size * 1.5}px;pointer-events:none;z-index:1000;transform-origin:${size * 0.18}px ${size * 0.1}px;transform:translate(${o.x - size * 0.18}px,${o.y - size * 0.1}px) scale(${num(o.scale, 1)})`;
  return el;
}

const SPINNER_SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-loader-circle spin"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path></svg>';
const CURSOR_SVG = '<svg viewBox="0 0 20 30" width="100%" height="100%" style="display:block;overflow:visible;filter:drop-shadow(0 1px 1.5px rgb(0 0 0 / 35%))"><path d="M3.6 3 L3.6 24.2 L8.6 19.4 L11.9 27 L15.1 25.6 L11.9 18.2 L18.6 18.2 Z" fill="#000" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>';

window.biliupUI = {
  parts: parts,
  size: size,
  slots: slots,
  mount: mount,
  find: find,
  findAll: findAll,
  piece: piece,
  sequence: sequence,
  fly: fly,
  rectIn: rectIn,
  frameText: frameText,
  cursor: cursor,
  timecode: timecode,
  css: css,
};
})();
