// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The templates that ship with Uncut (issue #111): Uncut's own, original briefs. Each is the text
// of a template file (templates.ts: front matter, Markdown, placeholders), kept here as strings so
// the server and the web bundle them with no files to package; Duplicate writes one out as the
// user's own .md file. General on purpose: they name no product and invent nothing; the facts come
// from the project through the placeholders.
import { parseTemplate, type Template } from "@/app/lib/editor/core/templates/templates";

const KEYNOTE = `---
name: Keynote one take
description: A product launch film rebuilt from the product's own UI: one unbroken take, a person asks and the AI does the editing, the camera pushes in so each moment fills the frame, on the beat.
---

<inputs>
Everything the film needs is in this project. Use only what is listed here; don't invent anything and don't stop to ask.
- Project: {{project}}
- The request typed on screen at the open, word for word (it is the user's own ask; edit it here before sending if you like):
  "Create a promo video for {{product|the product}} from this {{footage-kind}}"
  If it says just "footage", you may name the footage by what the clips show once you've looked at them (drone footage, a screen recording, product shots); change nothing else in it.
- Canvas: {{canvas}}. Length: {{length|the song's length}}.
- Song: {{music}}
- Beat grid: {{grid}}
- Footage (every file appears at least once):
{{footage}}
- Brand: {{kit}}
- The product's real screens. Open and study each one before drawing anything; the film is rebuilt from them:
{{screens}}
</inputs>

<look>
Rebuild the interface as flat vector graphics, measured from the screenshots: the same layout, proportions, labels and icons. Never approximate it from memory. UI labels are copied verbatim from the screenshots (if the model picker reads "Claude Default" with the Claude mark before it, it says exactly that); if a label doesn't fit, reframe or resize the element, never rename or truncate it.
- Colours by role. The accent marks only the one thing selected or active at a time:
{{palette}}
- Type: {{fonts}}. Interface labels in the body face, timecodes and numbers in the mono face if the kit has one, big statements in the heading face with tight tracking.
- Logos: {{logos}}. Placed as files, never traced or recoloured.
- The model picker under the composer, when the product has one, uses the actual model name and mark shown in the supplied product screenshots. If no mark is supplied, show the name alone; do not invent or substitute a provider logo.
- How this product's interface is built and what to show, from the kit. Follow its look exactly; the words, counts and results it quotes are examples, not a script (see Honesty below):
{{look}}
</look>

<direction>
A launch reveal told only with the product's own interface, in one unbroken take. Nothing cuts and nothing fades: every scene is made from a piece of the one before it. A button swells into a panel, a card becomes a row, a row stretches into a clip, a preview grows until it is the whole frame.
It must look like AI is editing. The cursor is the person, and it appears only where a person acts: typing the request into the composer and pressing send at the open, and pressing Export at the end. There it travels on an eased arc, settles, then types or clicks, and what it touched answers on that beat. After its last human act (the Export click) the cursor leaves the frame on its own motion, gliding off an edge or fading itself out (never a fade of the scene), and stays gone through the render, the logo and the close; it comes back only if the loop needs it on frame 0, at the same spot as on the first frame. Everything in between is the agent's work and happens with no cursor on screen: a step line of the agent ticks in, and on the next beat the edit it names appears by itself (a clip lands, a trim tightens, a look washes over the footage, a caption or a title writes itself in). Every change answers a step line; nothing moves without a visible cause.
The camera behaves like a screen-recording zoom: it pushes in until the moment that matters fills most of the frame, glides to the next one, and pulls back to the whole window at most once, for one bar, to show where things live. The cursor, while it is on screen, scales with it.
Readability floor, judged at phone size: any word the viewer has to read is at least 3 % of the frame's height on screen. The typed request is the biggest type in the film until the logo: the composer spans the frame's width and its text is at least 6 % of the frame's height. Whole windows at desktop scale with 1 % text are scenery, never the message; push in instead.
Every frame is full. The panel the camera is on always holds real content: no stretch of empty panel, no lone icon on a blank field, no dark frame waiting for the next thing.
Honesty: show only what the product has and what this run really did. Labels come from the screenshots. Numbers come from the inputs above (how many files, the song's length, the canvas) or from your own work in this turn (the beats of the storyboard you wrote). Labels, counts and results quoted in the kit's notes show the style only; the words and numbers on screen come from this run. Never show an outcome that hasn't happened: the critic's review comes after the film exists, so show it starting (its criteria rows appearing, bars beginning to fill), never a verdict, a final score or a made-up percentage. The export at the end may run to the product's own saved message, with no invented file size or time.
</direction>

<structure>
Plan in bars of the beat grid: count the bars below on the measured grid, start on a downbeat, change something on every beat and make one big change per bar.
1. Open, bars 1 to 2: the composer (or the product's first screen if it has none) fills the frame's width, already moving on frame 0. The cursor clicks into it and the request from the inputs is typed in big type, a word or two per beat, exactly as written: a user's plain ask, never edit instructions or a description of the film.
2. Send, bar 3: the cursor clicks send; the send control grows and carries the request into the agent's panel as its first message, landing large with the first step line already ticking under it: never one small bubble in a big empty panel, and the picture behind never drops to black.
3. Work, bars 4 to 5, no cursor from here: true step lines of this run tick in one per beat, with real counts (the files imported, the brief, the storyboard with its real number of beats). The cards the product has slide in under them. The camera stays pushed in so every line reads.
4. Build, up to the drop: one bar on the whole window, then a glide to the timeline: a step line names the cut, and the footage snaps onto the track one clip per beat with thumbnail strips, the song draws across with its waveform, the playhead starts to run. Then a dive into the preview that reaches full frame exactly on the drop.
5. Drop, 2 to 3 bars from {{drop|the drop found in the beat analysis}}: the real footage edge to edge, a new shot every one or two beats, each one arriving as a shape (inset, full bleed, pill, split), never as a cut. Only the best stretch of each clip.
6. Features, 2 to 3 bars: the footage shrinks back into the preview and one shape travels through 2 to 4 of the product's features, each a beat or two, each shown as the agent using it on this project: its step line ticks in, then the result fills the frame. Pick only what the product really has (its screens, or the kit's about and notes, name it) and what reads as motion at phone size and says the product's value at a glance. Good picks, when the product has them:
   - a look or effect visibly changing the footage (the plain shot, then the graded one)
   - captions appearing word by word on the video
   - a title or text animating in over the footage
   - clips trimming or snapping onto the timeline on the beat (if not already shown in the build)
   - the agent's review starting, its criteria rows appearing and their bars beginning to fill
   - one undo putting a whole turn back
   Steer away from dense panels and forms (settings, mixer faders, brand kit forms, file lists, capture setup) unless one is a single big, legible moment.
7. Close, the last 2 to 3 bars: the cursor comes back for the one last human act: it clicks Export, the product's export control answers, the progress runs and lands on its saved message. Then the last thing on screen folds into the logo while the tagline {{tagline|(none in the kit: leave it out)}} and {{url|(no URL in the kit: leave it out)}} rise out of a mask line, landing on the last downbeat. The logo never sits alone on an empty frame for more than a beat. Once Export is pressed the cursor glides off the frame (or fades itself out) and isn't seen through the render, the logo and the close. The last frame equals the first (the empty composer, and the cursor only if frame 0 has it, at the same spot), so the film loops.
</structure>

<never>
Crossfades, blur-ins, fades from or to black, brightness ramps that develop an image, 3D flips, particles, glows, lens flares, gradients on interface chrome, a theme the product doesn't use, panels, buttons or features missing from the screenshots, invented numbers, quotes or results, a hold over a second, near-empty frames, text too small for a phone, anything that looks like a stock template. And from the kit:
{{avoid}}
</never>

<build>
- One HTML motion graphic at the canvas size for the whole film. Every frame is computed from t alone inside an async seek(t): no CSS transitions or animations, no timers, nothing carried over from the previous frame.
- Motion settles on closed-form springs. A property that is moved several times is the sum of one spring per move, each starting at its own time, so every frame stands alone.
- Typing is computed from t as well (characters shown are a function of t), and the caret blinks on the grid.
- Clips landing on the timeline put their left and right edges on slightly different springs, so they stretch as they arrive.
- Footage: downscale each clip to the canvas size first, re-encode it with every frame a keyframe, load it from a blob URL and wait for the seeked event before drawing the frame.
- The song only, no sound effects unless asked, mastered around -14 LUFS.
- Leave the finished film on the timeline with the song under it, at the length above.
</build>

<gotchas>
- A shape that grows to fill the frame must overshoot the corners and take about a third of a second to get there; a fill that completes in one frame reads as a cut.
- Text that changes while its shape morphs needs a clip mask of its own, or it spills out.
- visibility: visible on a child beats a hidden parent; give children inherit.
- will-change on a layer the camera scales leaves its text soft; leave it off.
- A plain local file server often can't serve byte ranges, so a video loaded by URL won't seek; that's why the blob URL.
- While the cursor is on screen it stays inside the frame and above everything else; between send and Export, and after the Export click, it is gone, not parked by the button or in a corner.
- Compare the first and last frames, cursor included; any difference shows as a jump when the film loops.
- If you're not sure the product has something, leave it out.
</gotchas>

<start>
Don't ask questions; decide. First write the beat map: one line per bar with its time on the grid, what is on screen, what causes each change (the cursor, or the agent step line that names it) and where the camera is. Render four stills (the open with the request typed, the agent's work, the drop, the close) and check each against the readability floor and this brief. Then build the whole film, render a still per beat, scan for one-frame pops (a frame that changes far more than the frames around it) and fix them, and finish with the film and the song on the timeline.
</start>
`;

const ONE_SHAPE = `---
name: One shape, never cut
description: A one-take film where a single shape carries every scene, morphing from one state to the next on the beat, with no cuts.
---

<inputs>
Use only what this project has; don't ask for more and don't invent anything.
- Project: {{project}}
- Brand: {{kit}}
- Footage and images:
{{footage}}
- Song: {{music}}
- Canvas: {{canvas}}. Length: {{length|the song's length}}.
</inputs>

<direction>
One hero shape, a rounded rectangle or a pill, is on screen from the first frame to the last and never leaves it. Every scene happens inside it or is made from it: it holds a clip, stretches into a title bar, splits into a row of cards, floods the frame and contracts again. There are no cuts, wipes or fades; every change is the shape changing.
When the film is about footage, the shape carries the clips and is large: at rest it fills most of the frame, and it goes full bleed on the biggest moments. Footage stays sharp, never zoomed until it softens.
Words are few and big: a headline of one to three words at a time, rising out of a mask line, readable on a phone.
Never: hard cuts, crossfades, blur-ins, slices or tiles, bouncy easing, particles, glows, frames that are nearly empty, holds over a second except the end.
</direction>

<structure>
{{tempo}}, {{beats|a change on nearly every beat}}.
- Open: the shape is already moving on frame 0, holding the strongest image or a big headline.
- Middle: a new state every bar, each made out of the last. Vary the form (inset, full bleed, pill, split) so no two bars look the same.
- Drop ({{drop|the song's biggest moment}}): the shape floods the frame with the best shot.
- End: the shape settles into the logo or a last word from the kit, held about a second, and springs back to the first frame's shape so it loops.
</structure>

<build>
- One motion graphic for the whole film; every frame is a pure function of time.
- Morphs take about 0.4 s, moves about 0.8 s, settling softly. One main move at a time; the next can start as it settles.
- Read the song's beat grid and drop first, start on a downbeat, and put transitions on the kick.
- Check one still per bar, then scan for one-frame pops and near-empty frames.
</build>

<gotchas>
- A flood must overscale past the corners and take about a third of a second, or the corners pop and half the frame changes at once.
- A clip inside a shape is drawn at its shown size or larger; never upscale a small file into a full-bleed frame.
- The last frame must equal the first, or the loop stutters.
</gotchas>

<start>
Plan a state per bar on the song's real beats, check four stills (the open, the drop, the busiest moment, the end), then build the film and check it before you finish.
</start>
`;

const BEAT_CUT = `---
name: Beat-cut reel
description: A fast reel of the best moments of your footage, cut on the song's beats, with the biggest shot on the drop.
---

<inputs>
Use only what this project has; don't ask for more and don't invent anything.
- Project: {{project}}
- Footage:
{{footage}}
- Song: {{music}}
- Brand: {{kit}}
- Canvas: {{canvas}}. Length: {{length|the song's length}}.
</inputs>

<direction>
A reel that feels edited to the music: every cut lands on a beat, the energy follows the song. Pick the strongest moments of the footage: steady, sharp, well exposed, something moving in the frame. Leave out shaky, blurred or frozen stretches and gaps in the recording.
Fill the canvas: reframe each shot for this aspect ratio rather than letterboxing it, keeping the subject where the eye already is from the shot before.
Text is optional and sparse: at most a short title at the start and the brand at the end, using the kit's fonts and logo files.
Never: speed ramps or effects on every cut, the same shot twice, transitions that hide weak footage, invented captions or claims.
</direction>

<structure>
{{tempo}}, {{beats|cut on the beats}}.
- Open: the most striking shot on frame 0, already in motion. No fade from black.
- Build: shots of one or two beats, wider first, getting tighter and faster towards the drop.
- Drop ({{drop|the song's biggest moment}}): the best shot of all lands exactly here.
- After the drop: the fastest cutting, then slow down for the last bar.
- End: the last shot or the logo held on the final downbeat, the music ending with it, not cut off.
</structure>

<build>
- Analyse the footage for clean stretches and the song for its beat grid and drop before cutting.
- Place every cut on a beat frame; big changes on downbeats.
- Trim the song so it ends on a phrase, and fade only its last beat if it has to stop early.
- Keep the music around -14 LUFS; mute the clips' own sound unless it adds something.
- Watch it through once at full speed and fix any cut that lands off the beat.
</build>

<gotchas>
- A clip trimmed past a drop in its recording jumps; check the cut points.
- Two similar shots in a row read as a jump cut; separate them or pick one.
</gotchas>

<start>
Pick the shots and lay out the beat map first, then cut the reel, check it against the beats, and leave it on the timeline with the song.
</start>
`;

const TALKING_HEAD = `---
name: Talking head with subtitles
description: A tight cut of someone talking to camera, with the filler taken out and burned-in subtitles from the transcript.
---

<inputs>
Use only what this project has; don't ask for more and don't invent anything.
- Project: {{project}}
- Media:
{{media}}
- Brand: {{kit}}
- Canvas: {{canvas}}. Length: {{length|as long as the strongest take needs, under a minute for a social cut}}.
</inputs>

<direction>
A clean, tight cut of the speaker that keeps their meaning and their voice. Cut from the transcript: take out fillers, false starts, repeats and long pauses, keep whole sentences, and keep a pause when it lands a point. Never change what was said or the order that changes its meaning.
Open on the strongest line that stands on its own; it can move to the start if it makes sense there.
Hide jump cuts with a gentle punch-in on emphasis, alternating framings, or real b-roll from the media over the voice. Keep the face in the same place across cuts.
Subtitles on every line, from the transcript with names and product words spelled right: short groups of a few words, at most two lines, placed clear of the platform's buttons and the bottom of the frame, in the kit's font. One highlighted word per line at most.
Never: words the speaker didn't say, captions that bounce on every word, music that competes with the voice, effects over speech.
</direction>

<structure>
- Hook: the strongest line in the first two seconds, subtitled from frame 0.
- Body: one idea per beat of the talk, a few seconds each, in the speaker's order.
- Close: the call to action as said, then the logo and URL from the kit if there is one.
</structure>

<build>
- Transcribe first, then cut by the transcript.
- Make the subtitles from the same transcript after cutting, and read them through once against the audio.
- Voice clear and levelled around -14 LUFS; any music ducked well under it, or none.
- Check the hook, every punch-in and the subtitles at phone size.
</build>

<gotchas>
- A cut in the middle of a breath sounds wrong; cut in the gaps.
- Subtitles that sit too low get covered by the platform's interface.
</gotchas>

<start>
Transcribe, propose the cut as a list of the lines you keep, then make it, add the subtitles, and check the result before finishing.
</start>
`;

/** Biliup's built-in templates, in the order the menu lists them. */
export const BUILTIN_TEMPLATES: ReadonlyArray<Template> = [
  ["keynote-one-take", KEYNOTE],
  ["one-shape-never-cut", ONE_SHAPE],
  ["beat-cut-reel", BEAT_CUT],
  ["talking-head-subtitles", TALKING_HEAD],
].map(([id, text]) => ({ ...parseTemplate(text!, id!), builtin: true as const }));

export const BUILTIN_TEMPLATE_IDS: ReadonlySet<string> = new Set(BUILTIN_TEMPLATES.map((t) => t.id));
