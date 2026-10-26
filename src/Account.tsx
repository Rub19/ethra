import { useEffect, useMemo, useRef, useState } from "react";
import catalog from "./avatars.json";
import { DD, img, Summoner } from "./lcu";
import type { Go } from "./main";

const BASE = "http://ethone.localhost/";
type Collection = { id: string; label: string; items: [string, string, string][] };
const COLLECTIONS = catalog as Collection[];
const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

export function useAvatar() {
  const read = () => {
    try {
      return localStorage.getItem("rw-avatar") || "";
    } catch {
      return "";
    }
  };
  const [url, setUrl] = useState(read);
  useEffect(() => {
    const sync = () => setUrl(read());
    window.addEventListener("rw-avatar", sync);
    window.addEventListener("storage", sync);
    return () => (window.removeEventListener("rw-avatar", sync), window.removeEventListener("storage", sync));
  }, []);
  const save = (next: string) => {
    try {
      if (next) localStorage.setItem("rw-avatar", next);
      else localStorage.removeItem("rw-avatar");
    } catch {}
    window.dispatchEvent(new Event("rw-avatar"));
  };
  return [url, save] as const;
}

function resize(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => {
      const size = 256, c = document.createElement("canvas");
      c.width = c.height = size;
      const s = Math.min(image.width, image.height);
      c.getContext("2d")!.drawImage(image, (image.width - s) / 2, (image.height - s) / 2, s, s, 0, 0, size, size);
      resolve(c.toDataURL("image/webp", 0.85));
    };
    image.onerror = reject;
    image.src = URL.createObjectURL(file);
  });
}

function AvatarPicker({ current, fallback, onClose, onPick }: { current: string; fallback: string; onClose: () => void; onPick: (url: string) => void }) {
  const [platform, setPlatform] = useState(COLLECTIONS[0].id);
  const [q, setQ] = useState("");
  const [choice, setChoice] = useState(current);
  const file = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const sections = useMemo(() => {
    const words = fold(q).split(/\s+/).filter(Boolean);
    const col = COLLECTIONS.find((c) => c.id === platform)!;
    const hit = ([name, series]: [string, string, string]) => words.every((w) => fold(`${name} ${series}`).includes(w));
    const groups = new Map<string, [string, string, string][]>();
    for (const a of col.items) if (hit(a)) groups.set(a[1], [...(groups.get(a[1]) ?? []), a]);
    return [...groups.entries()];
  }, [platform, q]);

  return (
    <div className="modalbg" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal picker" role="dialog" aria-modal="true" aria-label="Choisir un avatar">
        <div className="phd">
          <img className="pv" src={choice || fallback} />
          <div className="grow">
            <h2>Choisir un avatar</h2>
            <small>Bibliothèque ETHONE · {COLLECTIONS.reduce((n, c) => n + c.items.length, 0)} avatars</small>
          </div>
          <button className="ghostbtn" onClick={() => setChoice("")}>Icône LoL</button>
          <button className="primary" onClick={() => (onPick(choice), onClose())}>Utiliser</button>
          <button className="xbtn" aria-label="Fermer" onClick={onClose}>✕</button>
        </div>
        <div className="seg wide">
          {COLLECTIONS.map((c) => <button key={c.id} className={platform === c.id ? "on" : ""} onClick={() => setPlatform(c.id)}>{c.label}</button>)}
        </div>
        <div className="prow2">
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Rechercher une série ou un personnage" autoFocus />
          <input ref={file} type="file" accept="image/png,image/jpeg,image/webp,image/gif" hidden onChange={(e) => e.target.files?.[0] && resize(e.target.files[0]).then(setChoice)} />
          <button className="ghostbtn" onClick={() => file.current?.click()}>Ma photo</button>
        </div>
        <div className="pscroll">
          {sections.length === 0 && <p className="muted center">Aucun résultat.</p>}
          {sections.map(([series, items]) => (
            <section key={series}>
              <h3>{series} <span className="muted">{items.length}</span></h3>
              <div className="arow">
                {items.map(([name, , path]) => {
                  const url = BASE + path;
                  return (
                    <button key={path} className={`av ${choice === url ? "on" : ""}`} onClick={() => setChoice(url)} title={name}>
                      <img src={url} loading="lazy" />
                      <span>{name}</span>
                    </button>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
        <small className="muted foot">Personnages et logos © leurs ayants droit respectifs</small>
      </div>
    </div>
  );
}

export default function AccountMenu({ self, d, go }: { self: Summoner; d: DD; go: Go }) {
  const [open, setOpen] = useState(false);
  const [picker, setPicker] = useState(false);
  const [avatar, setAvatar] = useAvatar();
  const ref = useRef<HTMLDivElement>(null);
  const icon = img.icon(d, self.profileIconId);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);

  const item = (label: string, run: () => void) => <button onClick={() => (setOpen(false), run())}>{label}</button>;

  return (
    <div className="account" ref={ref}>
      <button className="mechip" onClick={() => setOpen(!open)} aria-expanded={open}>
        <img src={avatar || icon} />
        {self.gameName}
        <span>#{self.tagLine}</span>
        <i className="chev">▾</i>
      </button>
      {open && (
        <div className="menu">
          <div className="mhd">
            <button className="avedit" onClick={() => (setOpen(false), setPicker(true))} title="Changer d'avatar">
              <img src={avatar || icon} />
              <span>Modifier</span>
            </button>
            <div>
              <b>{self.gameName}</b>
              <small>#{self.tagLine} · niveau {self.summonerLevel}</small>
            </div>
          </div>
          {item("Mon profil", () => go({ page: "profil" }))}
          {item("Changer d'avatar", () => setPicker(true))}
          <hr />
          {item("Overlays", () => go({ page: "overlays" }))}
          {item("Réglages", () => go({ page: "reglages" }))}
          <hr />
          <small className="muted ver">Ethra · gratuit et open source</small>
        </div>
      )}
      {picker && <AvatarPicker current={avatar} fallback={icon} onClose={() => setPicker(false)} onPick={setAvatar} />}
    </div>
  );
}
