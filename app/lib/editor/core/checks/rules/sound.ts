// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { DUCK } from "@/app/lib/editor/core/audio/effects";
import { duckEntry } from "@/app/lib/editor/core/audio/ducking";
import { type CheckRule, sounds, audible, isMusicTrack, soundOf, startOf, keyFade, mediaOf, isSpeechTrack, type Check, type CheckSeverity, secs } from "@/app/lib/editor/core/checks/context";
import { keyboardSound, SFX_PER_10S, SFX_REPEAT_MAX, SFX_UNDER_MUSIC_DB, SOUND_LIMITS, sfxItems, musicItems, visualEvents, busiest, type PlacedSfx, musicAt, SFX_FIX_UNDER_DB, trackGain, dbText, hitsOf, type VisualEvent, SLOW_BPM, FAST_CUT_S } from "@/app/lib/editor/core/checks/soundSheet";

/** Sound rules: silence, music fades and ducking, sound effects. */
export const SOUND_RULES: ReadonlyArray<CheckRule> = [
  {
    id: "silent",
    title: "Has sound",
    description: "A video gets an audio track: music at about −16 LUFS or the footage's own sound.",
    severity: "info",
    check: (ctx) => {
      if (ctx.end <= 0 || ctx.project.tracks.some((t) => sounds(ctx, t) && audible(ctx, t))) return [];
      return [{ key: "silent", message: "Nothing makes a sound, so the export carries a silent track.", hint: "Add a track from the Audio tab's library whose tempo fits the cutting, with fades." }];
    },
  },
  {
    id: "music-fades",
    title: "Music fades in and out",
    description: "Music that starts with the video or plays to its end fades in or out instead of starting or stopping cold.",
    severity: "warn",
    check: (ctx) =>
      ctx.project.tracks.flatMap((track) => {
        if (!isMusicTrack(ctx, track)) return [];
        return track.items.flatMap((item) => {
          if (item.type !== "audio" || !soundOf(ctx, item)) return [];
          const start = startOf(ctx, item);
          // A fade made with volume keys counts (the fix doesn't stack a fade on it).
          const needIn = !item.fadeIn && !keyFade(item, "in") && start === 0;
          const needOut = !item.fadeOut && !keyFade(item, "out") && start + item.duration >= ctx.end;
          if (!needIn && !needOut) return [];
          const fadeIn = needIn ? Math.min(ctx.fps, Math.floor(item.duration / 4)) : item.fadeIn;
          const fadeOut = needOut ? Math.min(2 * ctx.fps, Math.floor(item.duration / 4)) : item.fadeOut;
          const name = mediaOf(ctx, item)?.name ?? item.id;
          const what = needIn && needOut ? "starts and stops cold" : needIn ? "starts cold" : "stops cold";
          const set: Record<string, unknown> = {};
          if (needIn && fadeIn) set.fadeIn = fadeIn;
          if (needOut && fadeOut) set.fadeOut = fadeOut;
          const ok = Object.keys(set).length > 0 && !track.locked && !item.locked;
          return [
            {
              key: `music-fades:${item.id}`,
              message: `${name} ${what}; a short fade sounds finished.`,
              itemIds: [item.id],
              frame: needIn ? start : start + item.duration - 1,
              ...(ok ? { fix: { summary: needIn && needOut ? `Fade ${name} in and out` : needIn ? `Fade ${name} in` : `Fade ${name} out`, ops: [{ op: "update_item" as const, item: item.id, set }] } } : { hint: "Add a fade in the Inspector's Fades row." }),
            },
          ];
        });
      }),
  },
  {
    id: "music-ducking",
    title: "Music ducks under speech",
    description: "A music bed playing under a voice is ducked (audio.duck) so the voice stays clear.",
    severity: "info",
    check: (ctx) =>
      ctx.project.tracks.flatMap((track) => {
        if (!isMusicTrack(ctx, track) || duckEntry(track) || !sounds(ctx, track) || !audible(ctx, track)) return [];
        // The tracks it plays under: heard speech tracks (voice, narration, or main when its sound
        // is known to be speech) overlapping it in time. Effects and ambient sound don't count.
        const spans = track.items.map((i) => [startOf(ctx, i), startOf(ctx, i) + i.duration] as const);
        const keys = ctx.project.tracks.filter(
          (t) => t.id !== track.id && isSpeechTrack(ctx, t) && audible(ctx, t) && t.items.some((i) => soundOf(ctx, i) && spans.some(([s, e]) => startOf(ctx, i) < e && startOf(ctx, i) + i.duration > s)),
        );
        if (!keys.length) return [];
        const mixer = { ...(track.mixer ?? {}), effects: [...(track.mixer?.effects ?? []), { module: DUCK, params: { under: keys.map((k) => k.id), amountDb: -12, attackMs: 250, releaseMs: 600 } }] };
        const names = keys.map((k) => k.name);
        return [
          {
            key: `music-ducking:${track.id}`,
            message: `${track.name} plays under ${names.join(" and ")} at full level; ducking keeps the voice clear.`,
            itemIds: track.items.map((i) => i.id),
            frame: Math.min(...spans.map(([s]) => s)),
            ...(track.locked ? { hint: "Unlock the track, then add Duck in the Mixer." } : { fix: { summary: `Duck ${track.name} under ${names.join(" and ")}`, ops: [{ op: "update_track" as const, track: track.id, set: { mixer } }] } }),
          },
        ];
      }),
  },
  {
    id: "sound",
    title: "Sound design",
    description: `Sound effects are few and earn their place: at most ${SFX_PER_10S} in any 10 s, each on something that changes on screen, the same file at most ${SFX_REPEAT_MAX - 1} times, one big hit per 15 s, ${SFX_UNDER_MUSIC_DB} to 12 dB under the music, no clicks on text, no meme sounds unless asked, and music whose tempo fits the cutting.`,
    severity: "warn",
    check: (ctx) => {
      const out: Array<Omit<Check, "id" | "severity"> & { severity?: CheckSeverity }> = [];
      // D217: the video's sound profile sets the numbers; off: no sound-effect limits.
      if (ctx.sound === "off") return out;
      const lim = SOUND_LIMITS[ctx.sound];
      const fps = ctx.fps;
      const placed = sfxItems(ctx);
      // The music-only profile: the user asked for no effects, so one is a finding of its own and
      // the other effect rules have nothing to judge.
      const sfx = ctx.sound === "music" ? [] : placed;
      if (ctx.sound === "music" && placed.length) {
        out.push({
          key: `sound:music-only:${placed[0]!.start}`,
          message: `The user asked for the music only, but ${placed.length} sound effect${placed.length === 1 ? " is" : "s are"} on the timeline (${placed.slice(0, 4).map((s) => `${s.label} at ${secs(s.start, fps)}`).join(", ")}${placed.length > 4 ? ", …" : ""}).`,
          itemIds: placed.map((s) => s.item.id),
          frame: placed[0]!.start,
          hint: "Delete them; let the cuts and moves land on the music's beats instead.",
        });
      }
      const music = musicItems(ctx);
      const events = visualEvents(ctx);
      const length = Math.max(1, ctx.end);
      // Density: the busiest 10 s (a shorter video pro rata, at least 2).
      const window = Math.min(10 * fps, length);
      const limit = Math.max(2, Math.floor((lim.per10s * window) / (10 * fps)));
      const dense = busiest(sfx.map((s) => s.start), 10 * fps);
      if (dense.count > limit) {
        const inWindow = sfx.filter((s) => s.start >= dense.from && s.start <= dense.to);
        out.push({
          key: `sound:density:${dense.from}`,
          message: `${dense.count} sound effects start between ${secs(dense.from, fps)} and ${secs(dense.to, fps)}; at most ${limit} in any ${secs(window, fps)}, so each one means something.`,
          itemIds: inWindow.map((s) => s.item.id),
          frame: dense.from,
          range: { start: dense.from, end: dense.to + 1 },
          hint: "Keep the effects on the moments that matter (the reveal, a real click, the end card) and delete the rest; a cut on the beat needs no whoosh.",
        });
      }
      // The same file over and over.
      const bySound = new Map<string, Array<PlacedSfx>>();
      for (const s of sfx) bySound.set(s.sound, [...(bySound.get(s.sound) ?? []), s]);
      for (const [sound, uses] of bySound) {
        if (uses.length < lim.repeatMax) continue;
        out.push({
          key: `sound:repeat:${sound}`,
          message: `${uses[0]!.label} plays ${uses.length} times (${uses.map((s) => secs(s.start, fps)).join(", ")}); the same sound again and again reads as a template.`,
          itemIds: uses.map((s) => s.item.id),
          frame: uses[0]!.start,
          hint: `Keep it on at most ${lim.repeatMax - 1} real events, or use different sounds for different kinds of event.`,
        });
      }
      // Big hits.
      const hits = sfx.filter((s) => s.kind === "hit");
      const heavy = busiest(hits.map((s) => s.start), 15 * fps);
      if (heavy.count > lim.hitsPer15s) {
        const these = hits.filter((s) => s.start >= heavy.from && s.start <= heavy.to);
        out.push({
          key: `sound:hits:${heavy.from}`,
          message: `${these.length} big hits (${these.map((s) => `${s.label} at ${secs(s.start, fps)}`).join(", ")}) within 15 s; a boom lands once, on the moment that matters most.`,
          itemIds: these.map((s) => s.item.id),
          frame: heavy.from,
          hint: "Keep one hit for the reveal or the logo; the others go, or become something softer.",
        });
      }
      for (const s of sfx) {
        // Level against the music it plays over.
        const under = musicAt(music, s.start);
        if (under && s.gain - under.gain > -lim.underDb) {
          const target = Math.round((under.gain - (lim.underDb > 0 ? SFX_FIX_UNDER_DB : 0) - trackGain(s.track)) * 10) / 10;
          const ok = !s.track.locked && !s.item.locked && target >= -60;
          // Only the pack's level is known (peak-normalised): another file's may already be quiet,
          // so it's a soft "level" finding to listen to or measure, never a hard one.
          out.push({
            key: `sound:${s.bundled ? "loud" : "level"}:${s.item.id}`,
            ...(s.bundled ? {} : { severity: "info" as const }),
            message: s.bundled
              ? lim.underDb > 0
                ? `${s.label} at ${secs(s.start, fps)} is at ${dbText(s.gain)} over music at ${dbText(under.gain)}; effects sit ${lim.underDb} to 12 dB under the music (the pack's effects are peak-normalised, so at the same gain they jump out).`
                : `${s.label} at ${secs(s.start, fps)} is at ${dbText(s.gain)} over music at ${dbText(under.gain)}, more than ${-lim.underDb} dB over it even for a loud edit.`
              : `${s.label} at ${secs(s.start, fps)} is at ${dbText(s.gain)} over music at ${dbText(under.gain)}; its own loudness is unknown, so check it sits ${lim.underDb > 0 ? `${lim.underDb} to 12 dB under` : `no more than ${-lim.underDb} dB over`} the music (media_analyze loudness, or listen).`,
            itemIds: [s.item.id],
            frame: s.start,
            ...(ok ? { fix: { summary: lim.underDb > 0 ? `Put ${s.label} ${SFX_FIX_UNDER_DB} dB under the music` : `Put ${s.label} at the music's level`, ops: [{ op: "update_item" as const, item: s.item.id, set: { gainDb: target } }] } } : { hint: "Turn it down in the Inspector." }),
          });
        }
        // What it lands on. D297: under the ui profile a press inside a graphic (a motion film's one
        // item) lands on a change the timeline can't see: the critic judges it on the frames.
        const on = hitsOf(ctx, s, events);
        const inGraphic = ctx.sound === "ui" && ctx.project.tracks.some((t) => !t.hidden && t.items.some((i) => i.type === "graphic" && (ctx.starts.get(i.id) ?? 0) <= s.start && s.start < (ctx.starts.get(i.id) ?? 0) + i.duration));
        if (!on.length && !inGraphic) {
          const near = events.reduce<VisualEvent | undefined>((best, e) => (!best || Math.abs(e.frame - s.start) < Math.abs(best.frame - s.start) ? e : best), undefined);
          out.push({
            key: `sound:sync:${s.item.id}`,
            message: `${s.label} at ${secs(s.start, fps)} lands on nothing that changes on screen${near ? ` (the nearest is ${near.what} at ${secs(near.frame, fps)})` : ""}.`,
            itemIds: [s.item.id],
            frame: s.start,
            hint: s.kind === "whoosh" ? "Start a whoosh 1 to 2 frames before the cut or move it carries, or delete it." : s.kind === "riser" ? "End a riser on the reveal it builds to, or delete it." : "Move it onto the frame its event happens (1 to 2 frames early), or delete it.",
          });
        } else if (on.length && lim.taste && s.kind === "ui" && !keyboardSound(s) && on.every((e) => e.text)) {
          out.push({
            key: `sound:click-text:${s.item.id}`,
            message: `${s.label} at ${secs(s.start, fps)} clicks on a text (${on[0]!.what}); a click belongs on a click the viewer sees, and a text arriving or typing isn't one.`,
            itemIds: [s.item.id],
            frame: s.start,
            hint: "Delete it, or put it where a cursor or a finger presses something.",
          });
        }
        if (lim.taste && s.kind === "meme") {
          out.push({ key: `sound:meme:${s.item.id}`, severity: "info", message: `${s.label} at ${secs(s.start, fps)} is a meme sound; use one only when the user asked for that joke.`, itemIds: [s.item.id], frame: s.start, hint: "Delete it unless the brief asks for meme humour." });
        }
      }
      // A slow track under fast cuts.
      const slow = music.find((m) => typeof m.bpm === "number" && m.bpm < SLOW_BPM && m.item.duration >= Math.min(length, 8 * fps));
      if (slow) {
        const cuts = events.filter((e) => e.frame >= slow.start && e.frame < slow.start + slow.item.duration && !e.what.endsWith("leaving") && !e.what.endsWith("landing") && e.until === undefined);
        const avg = slow.item.duration / fps / Math.max(1, cuts.length + 1);
        if (cuts.length >= 4 && avg < FAST_CUT_S) {
          out.push({
            key: `sound:pace:${slow.item.id}`,
            severity: "info",
            message: `${slow.media.name} is ${slow.bpm} BPM under something new every ${avg.toFixed(1)} s; a slow loop under fast cutting feels sluggish.`,
            itemIds: [slow.item.id],
            frame: slow.start,
            hint: "Pick music whose tempo fits the cutting (110 to 130 BPM for a punchy promo) and cut on its beats (beat_grid), or slow the cutting down.",
          });
        }
      }
      return out;
    },
  },
];
