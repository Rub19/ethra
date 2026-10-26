# Ethra

Tracker de jeux gratuit et open source, inspiré de Blitz / tracker.gg. App desktop Windows (Tauri). Premier jeu supporté : League of Legends, d'autres suivront.

## Télécharger

**[⬇ Ethra 0.1.0 pour Windows (installeur)](https://github.com/Rub19/ethra/releases/latest/download/Ethra_0.1.0_x64-setup.exe)**

Toutes les versions : [Releases](https://github.com/Rub19/ethra/releases). L'installeur n'est pas signé : Windows SmartScreen peut afficher « Windows a protégé votre ordinateur », clique sur « Informations complémentaires » puis « Exécuter quand même ».

## Fonctionnalités

- **Profil** : rangs, historique détaillé avec badges, page de chaque partie (notes, courbe CS / or / XP, carte de partage), champions joués.
- **Tier list** et **builds méta** (runes, objets, ordre des compétences, counters) via OP.GG, Faille et ARAM.
- **Partie en cours** : rang, winrate et mains de chaque joueur dès la sélection des champions.
- **Import auto** des runes, sorts et item set en sélection.
- **Overlays en jeu** : timers et alertes d'objectifs, ordre des compétences, benchmark de lane, valeur des objets, stats.
- **Avatar** au choix parmi la bibliothèque ETHONE ou ta propre photo.

## Comment ça marche

Aucune clé API Riot nécessaire : comme Blitz, l'app parle au client League installé sur ton PC.

- **Client LoL (LCU)** : profils, historique, rangs, champ select, import runes / sorts / item sets.
- **Live Client Data API** (`127.0.0.1:2999`) : overlays en jeu.
- **OP.GG** : tier list et builds méta.
- **Data Dragon / CommunityDragon** : images et données statiques.

Overlays : mets le jeu en **mode fenêtré sans bordure**.

## Développer

```bash
npm install
npm run tauri dev
```

Build installeur : `npm run tauri build` (sortie dans `src-tauri/target/release/bundle/nsis`).

Ethra n'est pas approuvé par Riot Games et ne reflète pas les opinions de Riot Games.
