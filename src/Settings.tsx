import { useEffect, useState } from "react";
import { settings, Settings as S } from "./lcu";
import { RANK_FILTERS } from "./opgg";
import { seenPlayers } from "./valoApi";
import { checkUpdate, openUrl, REPO, toast, Update, VERSION } from "./util";

type Bool = { [K in keyof S]: S[K] extends boolean ? K : never }[keyof S];

const GENERAL: [Bool, string, string][] = [
  ["autoOpenLive", "Ouvrir la partie en cours automatiquement", "Kairo passe sur la page de la partie dès la sélection des champions ou des agents."],
  ["autoAccept", "Accepter la partie automatiquement", "Clique sur « Accepter » à ta place quand une partie League est trouvée."],
];
const LOL: [Bool, string, string][] = [
  ["autoRunes", "Importer les runes", "Crée une page « Kairo : Champion Rôle » dès que tu verrouilles ton champion."],
  ["autoSpells", "Importer les sorts d'invocateur", "Applique les sorts les plus joués avec ce champion."],
  ["autoItems", "Créer l'item set", "Ajoute ton build dans la boutique en jeu."],
];
const SHORTCUTS: [string, string][] = [
  ["Ctrl K", "Rechercher un joueur ou un champion"],
  ["Ctrl 1, 2, 3", "Passer sur League, VALORANT ou TFT"],
  ["Alt ← et Alt →", "Page précédente et suivante"],
  ["Boutons latéraux de la souris", "Page précédente et suivante"],
];

export default function Settings(_: { only?: string }) {
  const [cfg, setCfg] = useState<S>(settings);
  const [upd, setUpd] = useState<Update | undefined>(undefined);
  const [seen, setSeen] = useState(() => seenPlayers().length);

  useEffect(() => {
    checkUpdate().then(setUpd);
  }, []);

  const save = (patch: Partial<S>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
  };
  const toggle = ([k, l, help]: [Bool, string, string]) => (
    <label key={k} className="toggle">
      <div><b>{l}</b><small>{help}</small></div>
      <input type="checkbox" checked={!!cfg[k]} onChange={(e) => save({ [k]: e.target.checked })} />
    </label>
  );
  const clear = (keys: string[], msg: string) => {
    try {
      keys.forEach((k) => localStorage.removeItem(k));
    } catch {}
    toast(msg);
  };

  return (
    <div className="settings">
      <div className="pagehead">
        <h1>Réglages</h1>
        <p className="muted">Kairo est gratuit. Il lit tes clients Riot sur ce PC, la méta sur OP.GG et les images sur Data Dragon, CommunityDragon et valorant-api.com.</p>
      </div>
      <div className="setgrid">
        <div className="setcol">
          <section className="card">
            <h3>Général</h3>
            {GENERAL.map(toggle)}
          </section>
          <section className="card">
            <h3>League of Legends</h3>
            {LOL.map(toggle)}
            <label className="toggle col">
              <div><b>Source des builds</b><small>« Auto » prend la méta OP.GG, sinon tes parties, sinon la recommandation Riot.</small></div>
              <select value={cfg.source} onChange={(e) => save({ source: e.target.value })}>
                <option value="auto">Auto</option>
                <option value="opgg">Toujours la méta OP.GG</option>
                <option value="toi">Toujours tes parties</option>
                <option value="riot">Toujours la recommandation Riot</option>
              </select>
            </label>
            <label className="toggle col">
              <div><b>Rang des builds</b><small>Les builds OP.GG viennent des parties de ce niveau, aussi pour l'import automatique.</small></div>
              <select value={cfg.rankFilter} onChange={(e) => save({ rankFilter: e.target.value })}>
                {RANK_FILTERS.map(([k, l]) => <option key={k} value={k}>{l}</option>)}
              </select>
            </label>
          </section>
        </div>
        <div className="setcol">
          <section className="card">
            <h3>VALORANT</h3>
            <div className="setrow">
              <div><b>Joueurs croisés</b><small>{seen} joueurs mémorisés pour la recherche. Ils restent sur ce PC.</small></div>
              <button className="ghostbtn" disabled={!seen} onClick={() => (clear(["rw-vseen"], "Joueurs croisés oubliés"), setSeen(0))}>Oublier</button>
            </div>
          </section>
          <section className="card">
            <h3>Raccourcis</h3>
            <dl className="keys">
              {SHORTCUTS.map(([k, l]) => (
                <div key={k}><dt><kbd>{k}</kbd></dt><dd>{l}</dd></div>
              ))}
            </dl>
          </section>
          <section className="card">
            <h3>Données</h3>
            <div className="setrow">
              <div><b>Cache TFT</b><small>Noms et icônes du set en cours, rechargés une fois par jour.</small></div>
              <button className="ghostbtn" onClick={() => clear(["rw-tft"], "Cache TFT vidé, il sera rechargé")}>Vider</button>
            </div>
          </section>
          <section className="card about">
            <h3>À propos</h3>
            <div className="aboutrow">
              <span>Version</span>
              <b>
                {VERSION}{" "}
                {upd === undefined ? <small className="muted">vérification…</small> : upd ? <small className="good">{upd.version} disponible</small> : <small className="muted">à jour</small>}
              </b>
            </div>
            {upd && <button className="primary" onClick={() => openUrl(upd.url)}>Télécharger la version {upd.version}</button>}
            <div className="aboutrow">
              <span>Code source</span>
              <button className="linkbtn" onClick={() => openUrl(REPO)}>github.com/Rub19/ethra</button>
            </div>
            <small className="muted">Kairo n'est pas approuvé par Riot Games.</small>
          </section>
        </div>
      </div>
    </div>
  );
}
