# Parité Tauri → Rust natif (Slint)

Crate natif : `native/` (binaire `ethra.exe`, identifiant `dev.ethone.ethra`).
`[x]` porté et compilé. `[v]` porté et vu marcher dans le vrai exe. `[ ]` pas encore porté.

## État au 8 novembre 2026 (travail interrompu)

Tout compile, 44 tests passent, mais la parité n'est pas atteinte : les cases ci-dessous n'ont pas encore été cochées.

Bugs vus dans le vrai exe, à corriger d'abord :
- `Pic` sans largeur ni hauteur reste à 32 px : avatar du profil, splash des en-têtes et carte de partage.
- Liste des parties LoL vide sur le profil, alors que le panneau de gauche est rempli.
- Carte de partage non pleine largeur, bouton « Copier l'image » mal placé.
- Onglets CS, Or, XP trop larges.
- Bouton de fenêtre coupé à droite quand le compte est affiché.

Reste à faire : installeur natif jamais compilé, zone de notification et overlay jamais vus, VALORANT et TFT à relire, nouvelles fonctions, comparaison mémoire et taille en release.

## src-tauri/src/main.rs

- [ ] `lcu` : trouver `LeagueClientUx.exe`, port, token, requête HTTPS locale, oubli de la connexion en cas d'échec
- [ ] `live` : `https://127.0.0.1:2999/liveclientdata/*`
- [ ] `opgg` : appel JSON-RPC `mcp-api.op.gg/mcp`
- [ ] `valo` : lockfile, `ShooterGame.log`, version, entitlements, hôtes `local`, `pd`, `glz`, `shared`, `session`
- [ ] `game_icon` : icônes `.ico` de `ProgramData/Riot Games/Metadata`
- [ ] `open_url` : uniquement `https://github.com/Rub19/ethra`
- [ ] protocole `ethone://` : avatars de `ethone.dev/avatars/library`
- [ ] fenêtre principale 1360×860, min 1000×640, sans bordure, avec ombre
- [ ] fenêtre overlay transparente, toujours devant, hors barre des tâches, sans focus, plein écran, clics traversants
- [ ] fermer la fenêtre principale quitte tout

## src/lcu.ts

- [ ] `ddragon` : versions, champions, objets, runes, sorts, perks CommunityDragon
- [ ] `img` : champ, item, spell, icon, rune, emblem, role, ability
- [ ] `isCompleted`, `me`, `summoner`, `findPlayer`, `history`, `tftHistory`, `gameDetails` (avec cache), `ranked`, `queueNames` (avec cache)
- [ ] `rankScore`, `rankLabel`, `n`, `kda`, `cs`, `mmss`, `ago`, `settings` (valeurs par défaut)

## src/opgg.ts, src/opggParse.ts, src/opggParse.check.ts

- [ ] `call` (cache, JSON ou format OP.GG), `champId`, `meta`, `mainPosition`, `opggBuild`, `maxOf`, `opt`, `RANK_FILTERS`
- [ ] `parseOpgg`
- [ ] test de `opggParse.check.ts`

## src/history.ts, src/history.check.ts

- [ ] `isRemake`, `counted`, `filterGames`, `mergeGames`, `queuesIn`, `champsIn`, `formOf`, `parseTftGames`, `tftSummary`, `tftQueue`
- [ ] tests de `history.check.ts`

## src/buildData.ts

- [ ] `riotBuild`, `myBuild`, `bestBuild`, `pageName`
- [ ] `applyBuild` : page de runes, sorts, item set

## src/valoApi.ts

- [ ] `vdata`, `queueLabel`, `currentAct`, `vRank`, `seenPlayers`, `remember`, `vNames`, `vName` (par lots), `vHistory`, `vRRUpdates`
- [ ] `vMatch` (un appel à la fois, reprise sur 429), `vStats`, `vTeamWon`, `vScore`, `vParties`, `vPresence`, `mkLabel`, `vWeaponKills`, `vEconomy`

## src/tftApi.ts

- [ ] `tftData` (cache d'un jour), `tftDecks`

## src/util.ts

- [ ] favoris, toast, `copyText`, `openUrl`, `checkUpdate`, `newer`, version

## src/main.tsx

- [ ] routeur : historique, retour et suivant, boutons latéraux de la souris, Alt+← et Alt+→
- [ ] recherche LoL : Ctrl+K, joueur `Nom#TAG`, champions, favoris, joueurs récents, flèches, Entrée, Échap
- [ ] choix du jeu mémorisé, dernière page par jeu, Ctrl+1/2/3
- [ ] barre de titre : logo, retour et suivant, onglets avec trait animé, état du client, boutons de fenêtre
- [ ] suivi LCU toutes les 2 s : phase, ouverture auto de la partie, acceptation auto, import auto à la sélection
- [ ] suivi VALORANT toutes les 3 s : session, présence, ouverture auto
- [ ] notices « client fermé » avec tier list, agents ou decks en attendant
- [ ] bandeau « partie en cours », toast, navigation clavier sans animation

## src/GamePicker.tsx

- [ ] menu au survol et au clic, Échap, clic ailleurs
- [ ] logos : `.ico` Riot sinon tracé SVG, illustrations, mention légale

## src/Account.tsx

- [ ] menu du compte, `useAvatar`
- [ ] choix d'avatar : collections, recherche, sections, photo perso recadrée à 256 px, retour à l'icône LoL

## src/Profile.tsx

- [ ] `keepGames` (150 parties), `lpHistory` (200 points), `badges`
- [ ] en-tête avec splash, copie du Riot ID, stats rapides, sous-onglets Aperçu et Champions
- [ ] filtres par mode, champion et résultat, « Voir plus de parties »
- [ ] session du jour avec copie du résumé, rangs, courbe LP au survol, forme récente, top champions
- [ ] ligne de partie : icônes, KDA, KP, nombres, objets, badges, équipes cliquables
- [ ] `Scoreboard`, tableau des champions, états vide, erreur et réessayer

## src/Match.tsx

- [ ] `laneOpponent`, `score`, `grade`
- [ ] en-tête, notes, résumé, courbe CS / or / XP au survol, objectifs, tableau des scores
- [ ] carte de partage et copie de l'image

## src/Builds.tsx, src/Jungle.tsx

- [ ] sources OP.GG, tes parties, reco Riot avec repli
- [ ] Faille et ARAM, rôles, filtre de rang, mode compact
- [ ] runes, sorts, options d'objets, ordre des compétences avec variantes, matchups
- [ ] interrupteurs d'import auto, bouton « Importer dans le client »
- [ ] `route`, carte et étapes de la jungle, équipe et départ

## src/Champions.tsx, src/TierList.tsx

- [ ] champions : recherche, classes, joués ou tous, tri, grille, tableau
- [ ] tier list : recherche, favoris, rôles, tri, évolution, étoile

## src/Live.tsx

- [ ] attente, équipes en sélection et en jeu, cartes joueur avec tags OTP et séries, part AD/AP, build compact

## src/Overlays.tsx, src/overlay.tsx, src/Widget.tsx

- [ ] page Overlays : 9 cartes avec aperçu, interrupteur général
- [ ] timers Dragon, Nashor, inhibiteurs, alertes à 30 s
- [ ] prochaine compétence, ennemis morts, prochain objet, benchmark de lane, valeur des objets, stats, jungle path
- [ ] réglages appliqués sans relancer

## src/Settings.tsx

- [ ] toutes les options, source et rang des builds, joueurs croisés, raccourcis
- [ ] données : cache TFT, parties, LP, recherches, avec confirmation
- [ ] À propos : version, mise à jour, code source, attribution Slint

## src/Tft.tsx, src/TftProfile.tsx

- [ ] decks méta : stats, unités, objets, traits, difficulté, reroll
- [ ] profil : rang, résumé, lignes de partie, « Voir plus », erreur

## src/Updater.tsx

- [ ] vérification toutes les 30 min, repli sur GitHub, téléchargement avec progression, signature minisign, installation
- [ ] « Plus tard », erreur et page de téléchargement

## src/Valo.tsx

- [ ] profil : en-tête, onglets Parties, Agents, Cartes, Armes, filtres de file, session du jour, rangs, chargement par lots
- [ ] ligne de match : MVP, multi-kills, RR, promotion, réessayer
- [ ] recherche des joueurs croisés
- [ ] tier list des agents : rôles, tri, tier calculé
- [ ] page de match : manches, économie, détail d'une manche, tableau des scores
- [ ] partie en cours : sélection et en jeu, groupes, forme, rang moyen, joueurs masqués

## src/Skeleton.tsx

- [ ] squelettes de chargement

## localStorage → fichier local

Toutes les clés `rw-*` : `rw-settings`, `rw-recent`, `rw-game`, `rw-route-*`, `rw-games-*`, `rw-lp-*`, `rw-fav-champs`, `rw-vseen`, `rw-avatar`, `rw-tft`.

- [ ] `store.json` dans `%LOCALAPPDATA%\dev.ethone.ethra`
- [ ] reprise au premier lancement depuis `EBWebView\Default\Local Storage\leveldb`
