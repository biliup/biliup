import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { BrandState, BrandKit } from "@/app/lib/editor/contracts";
import { X, FolderOpen, Plus, Type, Sparkles } from "lucide-react";
import { useState } from "react";
import { actions, getState, brandFileUrl } from "@/app/lib/editor/state/store";
import { type View, type KitDraft, HEX6, type LogoSlot, lines, LOGO_SLOTS, LOGO_BACKDROP, COLOR_ROLES, FONT_ROLES } from "@/app/ui/editor/panels/brand/kit";
import { EditorFrame, ScopeChoice, Swatches } from "@/app/ui/editor/panels/brand/parts";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { openMenu } from "@/app/ui/editor/timeline/ContextMenu";
import { releaseBrandFiles } from '@/app/lib/editor/browser-files';
import {useEditorAction} from '../../controls/useEditorAction';
import {isTextComposition} from '@/app/lib/editor/keyboard';

export function KitEditor({ brand, view, hasProject, online, onDraft, onClose }: { brand: BrandState; view: Extract<View, { kind: "kit" }>; hasProject: boolean; online: boolean; onDraft: (f: (d: KitDraft) => KitDraft) => void; onClose: () => void }) {
  useEditorLanguage();
  const d = view.draft;
  const {busy,error,editable,isCurrent,run}=useEditorAction();
  const [note, setNote] = useState<string | undefined>();
  // The kit as saved (the style guide is the agent's: always the saved one).
  const saved = brand.kits.find((k) => k.id === view.id && k.scope === view.scope);
  const files = brand.files[view.id] ?? {};
  const [source, setSource] = useState("");
  // D198: logos and font files from a file, a folder or an address: logos fill the empty slots, fonts become rows (or a family's file).
  const importFiles = async (from: string | undefined,current:()=>boolean) => {
    const what = from?.trim();
    if (!what) return;
    if(!current()){releaseBrandFiles(what);return;}
    const got=await actions.brandImport(view.id, view.scope, what);
    if(!current())return;
    if (!got) return;
    setSource("");
    onDraft((cur) => {
      const logos = { ...cur.logos };
      for (const f of got) if (f.kind === "logo" && f.slot && !logos[f.slot]) logos[f.slot] = f.name;
      const fonts = [...cur.fonts];
      for (const f of got.filter((x) => x.kind === "font")) {
        if(fonts.some(font=>font.file===f.name))continue;
        const family = f.family ?? f.name;
        const same = fonts.findIndex((x) => x.family.trim().toLowerCase() === family.toLowerCase() && !x.file);
        if (same >= 0) fonts[same] = { ...fonts[same]!, file: f.name };
        else fonts.push({ family, role: fonts.length ? "body" : "headline", file: f.name });
      }
      return { ...cur, logos, fonts };
    });
  };
  const importFrom=(from:string)=>run(current=>importFiles(from,current));
  const chooseImport=(directory=false)=>run(async(current)=>importFiles(await actions.pickBrandSource(directory),current));
  const fontFile = (i: number) => run(async(current)=>{
    const from = await actions.pickBrandSource();
    if (!from) return;
    if(!current()){releaseBrandFiles(from);return;}
    const got = await actions.brandImport(view.id, view.scope, from);
    if(!current())return;
    const font = got?.find((f) => f.kind === "font");
    if (font) onDraft((cur) => ({ ...cur, fonts: cur.fonts.map((x, j) => (j === i ? { ...x, file: font.name, family: x.family.trim() || (font.family ?? x.family) } : x)) }));
  });
  const badHex = d.colors.findIndex((c) => !HEX6.test(c.hex.trim()));
  const set = (patch: Partial<KitDraft>) => onDraft((cur) => ({ ...cur, ...patch }));

  const pickInto = (multi: boolean) => run(async(current)=>{
    const paths = await actions.pickBrandFiles();
    const names: Array<string> = [];
    try{
      for (const path of multi ? paths : paths.slice(0, 1)) {
        if(!current())return;
        const name = await actions.brandAddFile(view.id, view.scope, path);
        if (name) names.push(name);
      }
      return names;
    }finally{paths.forEach(releaseBrandFiles)}
  });

  const save = () => run(async(owned)=>{
    if (badHex >= 0 || !d.name.trim()) return;
    const current = getState().brand;
    const style = current?.kits.find((k) => k.id === view.id && k.scope === view.scope)?.style;
    let logos: Partial<Record<LogoSlot, string>> = { ...d.logos };
    let examples = [...d.examples];
    let fontFiles = new Map<string, string>();
    const moving = d.scope !== view.scope;
    if (moving) {
      // The files go with the kit: copied into the new place (maybe under another name); an unpinned project copy then goes.
      const oldFiles = current?.files[view.id] ?? {};
      const renamed = new Map<string, string>();
      for (const name of new Set([...Object.values(logos), ...examples, ...d.fonts.map((f) => f.file)].filter((n): n is string => !!n))) {
        if(!owned())return false;
        const path = oldFiles[name];
        if (!path) continue;
        const copied = await actions.brandAddFile(view.id, d.scope, path);
        if (!copied) return false;
        renamed.set(name, copied);
      }
      const to = (n: string | undefined) => (n === undefined ? undefined : (renamed.get(n) ?? n));
      logos = Object.fromEntries(Object.entries(logos).map(([k, v]) => [k, to(v)]).filter(([, v]) => v !== undefined));
      examples = examples.map((n) => to(n)!);
      fontFiles = new Map(d.fonts.flatMap((f) => (f.file ? [[f.file, to(f.file)!] as const] : [])));
    }
    const kit: BrandKit = {
      id: view.id,
      name: d.name.trim(),
      scope: d.scope,
      logos,
      colors: d.colors.map((c) => ({ hex: c.hex.trim().toLowerCase(), role: c.role, ...(c.name ? { name: c.name } : {}) })),
      fonts: d.fonts.filter((f) => f.family.trim()).map((f) => ({ family: f.family.trim(), role: f.role, ...(f.weight ? { weight: f.weight } : {}), ...(f.file ? { file: fontFiles.get(f.file) ?? f.file } : {}) })),
      ...(d.about.trim() ? { about: d.about.trim() } : {}),
      tone: d.tone,
      taglines: lines(d.taglines),
      urls: lines(d.urls),
      dos: lines(d.dos),
      donts: lines(d.donts),
      examples,
      ...(style ? { style } : {}),
    };
    if(!owned())return false;
    let ok = await actions.brandSaveKit(kit);
    // D198: pinning to this project copies the kit (the library keeps it for other projects); unpinning moves it back.
    if (ok && owned() && moving && view.scope === "project") ok = await actions.brandDeleteKit(view.id, view.scope);
    return ok;
  },{onSuccess:ok=>{if(ok)onClose()}});

  const remove = () => run(()=>actions.brandDeleteKit(view.id,view.scope),{onSuccess:ok=>{if(ok)onClose()}});

  const fromReference = () => run(async(current)=>{
    setNote(undefined);
    if (getState().agent?.running) return setNote("The agent is busy");
    const path = await actions.pickBrandReference();
    if (!path) return;
    if(!current()){releaseBrandFiles(path);return;}
    if (getState().agent?.running){releaseBrandFiles(path);return setNote("The agent is busy");}
    const file=await actions.brandAddFile(view.id,view.scope,path);
    if(!file||!current())return;
    if (getState().agent?.running) return setNote("The agent is busy");
    const asset=getState().brand?.files[view.id]?.[file];
    if(!asset)return;
    await actions.agentSend(`Make a style guide for the kit ${JSON.stringify(saved?.name ?? (d.name.trim() || view.id))} (${view.id}). Read its existing identity using brand_read, then inspect this registered reference using reference_frames: ${JSON.stringify(asset)}. Save the observed style with brand_write. Preserve the existing brand colours and fonts unless I approve their replacement.`, [], [], { access: "auto" });
    if(current())actions.openPanel("agent");
  });

  const style = saved?.style;
  return (
    <EditorFrame
      title={d.name.trim() || editorText("Kit")}
      testId="brand-kit-editor"
      onClose={onClose}
      onSave={() => void save()}
      onDelete={saved ? remove : undefined}
      deleteScope={view.scope}
      saveDisabled={!online || badHex >= 0 || !d.name.trim()}
      busy={busy} readOnly={!editable}
      problem={error||(badHex >= 0 ? "Colours are #rrggbb" : undefined)}
    >
      <div className="lib-section">
        <span className="lib-label">{editorText("Name")}</span>
        <input className="field input" data-testid="brand-kit-name" maxLength={80} value={d.name} onChange={(e) => set({ name: e.target.value })} />
        <span className="lib-label">{editorText("Where")}</span>
        <ScopeChoice value={d.scope} hasProject={hasProject} testId="brand-kit-scope" onChange={(scope) => set({ scope })} />
      </div>

      <div className="lib-section">
        <span className="lib-label">{editorText("Logos")}</span>
        {LOGO_SLOTS.map(({ id, label }) => {
          const name = d.logos[id];
          const src = name && files[name] ? brandFileUrl(view.id, name, files[name]) : undefined;
          return (
          <div key={id} className="brand-line" data-testid={`brand-logo-${id}`}>
            <span className="brand-line-k">{label}</span>
            <span className="brand-logo-thumb" style={{ background: LOGO_BACKDROP[id] }} data-testid={`brand-logo-${id}-thumb`}>
              {src ? <img src={src} alt={name} draggable={false} /> : null}
            </span>
            <span className={`brand-file ellipsis ${name ? "" : "muted"}`} title={name ? files[name] : undefined}>
              {name ?? editorText("None")}
            </span>
            <button
              className="btn ghost brand-small-btn"
              data-testid={`brand-logo-${id}-pick`}
              disabled={!online}
              onClick={() =>
                void pickInto(false).then((names) => {
                  const name=names?.[0];
                  if (name&&isCurrent()) onDraft((cur) => ({ ...cur, logos: { ...cur.logos, [id]: name } }));
                })
              }
            >
              {editorText("Choose…")}</button>
            {d.logos[id] && (
              <Tooltip label={editorText("Remove")}>
                <button
                  className="icon-btn"
                 
                  data-testid={`brand-logo-${id}-clear`}
                  onClick={() => {
                    const { [id]: _gone, ...rest } = d.logos;
                    set({ logos: rest });
                  }}
                >
                  <X />
                </button>
              </Tooltip>
            )}
          </div>
          );
        })}
        <div className="brand-line brand-import" data-testid="brand-import">
          <input
            className="field input"
            data-testid="brand-import-source"
            placeholder={editorText("HTTPS file link")}
            spellCheck={false}
            value={source}
            onChange={(e) => setSource(e.target.value)}
            onKeyDown={(e) => {if(e.key==='Enter'&&!isTextComposition(e.nativeEvent)){e.preventDefault();void importFrom(source)}}}
          />
          <Tooltip label={editorText("Import files or folder")}>
            <button className="icon-btn" aria-label={editorText("Import files or folder")} aria-haspopup="menu" data-testid="brand-import-pick" disabled={!online || busy} onClick={e=>{
              const rect=e.currentTarget.getBoundingClientRect();
              openMenu(rect.left,rect.bottom,[
                {id:'brand-files',label:editorText('Choose files'),run:()=>void chooseImport()},
                {id:'brand-folder',label:editorText('Choose folder'),run:()=>void chooseImport(true)},
              ],editorText('Import'));
            }}>
              <FolderOpen />
            </button>
          </Tooltip>
          <button className="btn ghost brand-small-btn" data-testid="brand-import-go" disabled={!online || busy || !source.trim()} onClick={() => void importFrom(source)}>
            {editorText("Import")}</button>
        </div>
      </div>

      <div className="lib-section">
        <div className="brand-head">
          <span className="lib-label">{editorText("Colours")}</span>
          <Tooltip label={editorText("Add colour")}>
            <button className="icon-btn" data-testid="brand-color-add" onClick={() => set({ colors: [...d.colors, { hex: "#ffffff", role: d.colors.length ? "accent" : "background" }] })}>
              <Plus />
            </button>
          </Tooltip>
        </div>
        {d.colors.map((c, i) => {
          const ok = HEX6.test(c.hex.trim());
          const patch = (p: Partial<(typeof d.colors)[number]>) => set({ colors: d.colors.map((x, j) => (j === i ? { ...x, ...p } : x)) });
          return (
            <div key={i} className="brand-line" data-testid={`brand-color-${i}`}>
              <span className="brand-swatch big" style={{ background: ok ? c.hex.trim() : "transparent" }} />
              <input className={`field input mono brand-hex ${ok ? "" : "invalid"}`} data-testid={`brand-color-hex-${i}`} value={c.hex} maxLength={7} spellCheck={false} onChange={(e) => patch({ hex: e.target.value })} />
              <select className="field input" data-testid={`brand-color-role-${i}`} value={c.role} onChange={(e) => patch({ role: e.target.value })}>
                {!(COLOR_ROLES as ReadonlyArray<string>).includes(c.role) && <option value={c.role}>{c.role}</option>}
                {COLOR_ROLES.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
              <Tooltip label={editorText("Remove")}>
                <button className="icon-btn" data-testid={`brand-color-remove-${i}`} onClick={() => set({ colors: d.colors.filter((_, j) => j !== i) })}>
                  <X />
                </button>
              </Tooltip>
            </div>
          );
        })}
      </div>

      <div className="lib-section">
        <div className="brand-head">
          <span className="lib-label">{editorText("Fonts")}</span>
          <Tooltip label={editorText("Add font")}>
            <button className="icon-btn" data-testid="brand-font-add" onClick={() => set({ fonts: [...d.fonts, { family: "", role: d.fonts.length ? "body" : "headline" }] })}>
              <Plus />
            </button>
          </Tooltip>
        </div>
        {d.fonts.map((f, i) => {
          const patch = (p: Partial<(typeof d.fonts)[number]>) => set({ fonts: d.fonts.map((x, j) => (j === i ? { ...x, ...p } : x)) });
          return (
            <div key={i} className="brand-line" data-testid={`brand-font-${i}`}>
              <input className="field input" data-testid={`brand-font-family-${i}`} placeholder={editorText("Family")} value={f.family} maxLength={80} onChange={(e) => patch({ family: e.target.value })} />
              <select className="field input brand-font-role" data-testid={`brand-font-role-${i}`} value={f.role} onChange={(e) => patch({ role: e.target.value })}>
                {!(FONT_ROLES as ReadonlyArray<string>).includes(f.role) && <option value={f.role}>{f.role}</option>}
                {FONT_ROLES.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
              <input
                className="field input mono brand-weight"
                data-testid={`brand-font-weight-${i}`}
                type="number"
                min={100}
                max={900}
                step={100}
                placeholder="400"
                value={f.weight ?? ""}
                onChange={(e) => {
                  const n = Number(e.target.value);
                  patch({ weight: e.target.value && n >= 100 && n <= 900 ? Math.round(n) : undefined });
                }}
              />
              <Tooltip label={f.file ? editorText("{0}: loaded in graphics, texts and the export", { 0: f.file }) : editorText("Add the font's file")}>
                <button className={`icon-btn ${f.file ? "on" : ""}`} data-testid={`brand-font-file-${i}`} disabled={!online} onClick={() => void fontFile(i)}>
                  <Type />
                </button>
              </Tooltip>
              <Tooltip label={editorText("Remove")}>
                <button className="icon-btn" data-testid={`brand-font-remove-${i}`} onClick={() => set({ fonts: d.fonts.filter((_, j) => j !== i) })}>
                  <X />
                </button>
              </Tooltip>
            </div>
          );
        })}
      </div>

      <div className="lib-section">
        <span className="lib-label">{editorText("What it is")}</span>
        <textarea className="gen-prompt brand-text" data-testid="brand-kit-about" rows={3} placeholder={editorText("What the product is and does, in a paragraph")} value={d.about} onChange={(e) => set({ about: e.target.value })} />
        <span className="lib-label">{editorText("Tone of voice")}</span>
        <textarea className="gen-prompt brand-text" data-testid="brand-kit-tone" rows={3} value={d.tone} onChange={(e) => set({ tone: e.target.value })} />
        <span className="lib-label">{editorText("Taglines, one per line")}</span>
        <textarea className="gen-prompt brand-text" data-testid="brand-kit-taglines" rows={2} value={d.taglines} onChange={(e) => set({ taglines: e.target.value })} />
        <span className="lib-label">{editorText("URLs, one per line")}</span>
        <textarea className="gen-prompt brand-text mono" data-testid="brand-kit-urls" rows={2} spellCheck={false} value={d.urls} onChange={(e) => set({ urls: e.target.value })} />
        <span className="lib-label">{editorText("Do's, one per line")}</span>
        <textarea className="gen-prompt brand-text" data-testid="brand-kit-dos" rows={3} value={d.dos} onChange={(e) => set({ dos: e.target.value })} />
        <span className="lib-label">{editorText("Don'ts, one per line")}</span>
        <textarea className="gen-prompt brand-text" data-testid="brand-kit-donts" rows={3} value={d.donts} onChange={(e) => set({ donts: e.target.value })} />
      </div>

      <div className="lib-section">
        <div className="brand-head">
          <span className="lib-label">{editorText("Examples")}</span>
          <Tooltip label={editorText("Add examples")}>
            <button
              className="icon-btn"
             
              data-testid="brand-example-add"
              disabled={!online}
              onClick={() => void pickInto(true).then((names) => {if(names?.length&&isCurrent())onDraft((cur) => ({ ...cur, examples: [...cur.examples, ...names.filter((n) => !cur.examples.includes(n))] }))})}
            >
              <Plus />
            </button>
          </Tooltip>
        </div>
        {d.examples.length === 0 && <div className="muted small">{editorText("None")}</div>}
        {d.examples.map((name, i) => (
          <div key={name} className="brand-line" data-testid={`brand-example-${i}`}>
            <span className="brand-file ellipsis" title={files[name]}>
              {name}
            </span>
            <Tooltip label={editorText("Remove")}>
              <button className="icon-btn" data-testid={`brand-example-remove-${i}`} onClick={() => set({ examples: d.examples.filter((x) => x !== name) })}>
                <X />
              </button>
            </Tooltip>
          </div>
        ))}
      </div>

      <div className="lib-section" data-testid="brand-style">
        <span className="lib-label">{editorText("Style guide")}</span>
        {style ? (
          <div className="brand-style">
            {style.palette.length > 0 && <Swatches hexes={style.palette.map((p) => p.hex)} />}
            <StyleRow label={editorText("Type")} value={style.type} />
            <StyleRow label={editorText("Shot lengths")} value={style.shotLengths} />
            <StyleRow label={editorText("Transitions")} value={style.transitions} />
            <StyleRow label={editorText("Camera")} value={style.camera} />
            <StyleRow label={editorText("Grain")} value={style.grain} />
            <StyleRow label={editorText("Notes")} value={style.notes ?? ""} />
            <StyleRow label={editorText("From")} value={style.source ?? ""} />
          </div>
        ) : (
          <div className="muted small">{editorText("None yet")}</div>
        )}
        <button className="btn brand-wide-btn" data-testid="brand-style-from-reference" disabled={!online || !saved} onClick={() => void fromReference()}>
          <Sparkles />
          {editorText("Make a style from a reference…")}</button>
        {note && (
          <div className="muted small" data-testid="brand-style-note">
            {editorText(note)}
          </div>
        )}
      </div>
    </EditorFrame>
  );
}

function StyleRow({ label, value }: { label: string; value: string }) {
  useEditorLanguage();
  if (!value.trim()) return null;
  return (
    <div className="brand-style-row">
      <span className="brand-line-k">{label}</span>
      <span className="selectable">{value}</span>
    </div>
  );
}
