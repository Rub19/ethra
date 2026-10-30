import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { checkUpdate, openUrl, REPO } from "./util";

type State = "ready" | "downloading" | "installing" | "error";
const EVERY = 30 * 60 * 1000;

export default function UpdateCorner() {
  const [upd, setUpd] = useState<Update | null>(null);
  const [fallback, setFallback] = useState<{ version: string; url: string } | null>(null);
  const [state, setState] = useState<State>("ready");
  const [progress, setProgress] = useState(0);
  const [later, setLater] = useState("");

  useEffect(() => {
    const run = () =>
      check().then(
        (u) => u && setUpd(u),
        () => checkUpdate().then((x) => x && setFallback(x)),
      );
    run();
    const id = setInterval(run, EVERY);
    return () => clearInterval(id);
  }, []);

  const version = upd?.version ?? fallback?.version;
  if (!version || later === version) return null;

  const install = async () => {
    if (!upd) return openUrl(fallback?.url ?? `${REPO}/releases/latest`);
    setState("downloading");
    let total = 0;
    let got = 0;
    try {
      await upd.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        if (e.event === "Progress") {
          got += e.data.chunkLength;
          setProgress(total ? got / total : 0);
        }
        if (e.event === "Finished") setState("installing");
      });
    } catch {
      setState("error");
    }
  };
  const notes = upd?.body
    ?.split("\n")
    .map((l) => l.replace(/^[#\-\s]+/, "").replace(/[*_`]/g, "").trim())
    .find((l) => l && !/^nouveaut/i.test(l));

  return (
    <aside className={`updcorner ${state === "error" ? "err" : ""}`} role="status" aria-live="polite">
      <div className="updhead">
        <span className="updmark" aria-hidden="true" />
        <div>
          <b>{state === "error" ? "La mise à jour a échoué" : "Mise à jour disponible"}</b>
          <small>
            {state === "error"
              ? "Télécharge la nouvelle version depuis la page de Kairo."
              : state === "installing"
                ? "Installation, Kairo va redémarrer…"
                : `Kairo ${version} est prête. Tes réglages sont conservés.`}
          </small>
        </div>
      </div>
      {notes && state === "ready" && <p className="updnotes">{notes}</p>}
      {state === "downloading" || state === "installing" ? (
        <div className="updbar" aria-label={`Téléchargement ${Math.round(progress * 100)} %`}>
          <i style={{ width: `${Math.max(4, progress * 100)}%` }} />
          <small>{state === "installing" ? "Installation…" : `Téléchargement ${Math.round(progress * 100)} %`}</small>
        </div>
      ) : (
        <div className="updactions">
          <button className="primary" onClick={state === "error" ? () => openUrl(`${REPO}/releases/latest`) : install}>
            {state === "error" ? "Ouvrir la page" : upd ? "Redémarrer et mettre à jour" : "Télécharger"}
          </button>
          <button className="ghostbtn" onClick={() => setLater(version)}>Plus tard</button>
        </div>
      )}
    </aside>
  );
}
