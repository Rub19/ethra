# Riftwatch

Tracker League of Legends gratuit et open source, inspiré de Blitz / tracker.gg. App desktop Windows (Tauri).

Aucune clé API Riot nécessaire : comme Blitz, l'app parle au client League installé sur ton PC.

- **Client LoL (LCU)** : profils, historique, rangs, champ select, import runes / sorts / item sets.
- **Live Client Data API** (`127.0.0.1:2999`) : overlays en jeu.
- **Data Dragon** : images et données statiques.

## Lancer

```bash
npm install
npm run tauri dev
```

Build installeur : `npm run tauri build` (sortie dans `src-tauri/target/release/bundle/nsis`).

Overlays : mets le jeu en **mode fenêtré sans bordure**.

Riftwatch n'est pas approuvé par Riot Games et ne reflète pas les opinions de Riot Games.
