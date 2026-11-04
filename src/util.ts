import { invoke } from "@tauri-apps/api/core";
import pkg from "../package.json";

export const REPO = "https://github.com/Rub19/ethra";

export const pressable = (run: () => void) => ({
  role: "button",
  tabIndex: 0,
  onClick: run,
  onKeyDown: (e: { key: string; preventDefault: () => void }) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    run();
  },
});

const FAVS = "rw-fav-champs";
export const favChamps = (): number[] => {
  try {
    return JSON.parse(localStorage.getItem(FAVS) || "[]");
  } catch {
    return [];
  }
};
export const toggleFavChamp = (id: number) => {
  const next = favChamps().includes(id) ? favChamps().filter((x) => x !== id) : [id, ...favChamps()];
  try {
    localStorage.setItem(FAVS, JSON.stringify(next));
  } catch {}
  return next;
};

export const sortProps = (active: boolean, desc: boolean, run: () => void) => ({
  tabIndex: 0,
  "aria-sort": active ? (desc ? "descending" : "ascending") : "none",
  onClick: run,
  onKeyDown: (e: { key: string; preventDefault: () => void }) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    run();
  },
}) as const;

export const toast = (msg: string) => window.dispatchEvent(new CustomEvent("rw-toast", { detail: msg }));

export const copyText = (text: string, label: string) =>
  navigator.clipboard.writeText(text).then(
    () => toast(`${label} copié`),
    () => toast("Copie impossible"),
  );

export const openUrl = (url: string) => invoke("open_url", { url }).catch(() => toast("Impossible d'ouvrir le lien"));

const parts = (v: string) => v.replace(/^v/, "").split(".").map((x) => parseInt(x, 10) || 0);
const newer = (a: string, b: string) => {
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
  return false;
};

export type Update = { version: string; url: string } | null;
let check: Promise<Update> | undefined;
export const checkUpdate = () =>
  (check ??= fetch("https://api.github.com/repos/Rub19/ethra/releases/latest")
    .then((r) => (r.ok ? r.json() : null))
    .then((j) => (j?.tag_name && newer(j.tag_name, pkg.version) ? { version: j.tag_name.replace(/^v/, ""), url: j.html_url } : null))
    .catch(() => null));

export const VERSION = pkg.version;
