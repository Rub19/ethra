# Parité Tauri → Rust natif (Slint)

Crate natif : `native/` (binaire `ethra.exe`, identifiant `dev.ethone.ethra`).
`[x]` porté et compilé. `[v]` porté et vu marcher dans le vrai exe. `[ ]` pas encore porté.

## État au 9 novembre 2026

Revue faite ligne par ligne contre le code (`native/src`, `native/ui`) : chaque fonction ou chaîne attendue a été retrouvée. `[v]` reprend ce qui a été vu marcher dans le vrai exe, hors écran (voir `HANDOFF-RUST.md`). Rien n'a été essayé à la souris ni par-dessus une vraie partie.

Bugs du 8 novembre :
- `Pic` à 32 px et bouton de fenêtre coupé : corrigés (« composants à la bonne taille, en-tête qui tient »).
- Liste des parties LoL vide : corrigée, vue avec le jeu d'essai ; pas encore revue avec le vrai client.
- Onglets CS, Or, XP : bons, recapturés le 9 novembre.
- Carte de partage : pleine largeur, mais le bouton « Copier l'image » était parti à gauche par-dessus les runes ; corrigé le 9 novembre et recapturé. La copie elle-même n'a pas été refaite.

## src-tauri/src/main.rs

- [v] `lcu` : trouver `LeagueClientUx.exe`, port, token, requête HTTPS locale, oubli de la connexion en cas d'échec
- [v] `live` : `https://127.0.0.1:2999/liveclientdata/*`
- [v] `opgg` : appel JSON-RPC `mcp-api.op.gg/mcp`
- [x] `valo` : lockfile, `ShooterGame.log`, version, entitlements, hôtes `local`, `pd`, `glz`, `shared`, `session`
- [x] `game_icon` : icônes `.ico` de `ProgramData/Riot Games/Metadata`
- [x] `open_url` : uniquement `https://github.com/Rub19/ethra`
- [x] protocole `ethone://` : avatars de `ethone.dev/avatars/library`
- [x] fenêtre principale 1360×860, min 1000×640, sans bordure, avec ombre
- [v] fenêtre overlay transparente, toujours devant, hors barre des tâches, sans focus, plein écran, clics traversants
- [x] fermer la fenêtre principale quitte tout

## src/lcu.ts

- [v] `ddragon` : versions, champions, objets, runes, sorts, perks CommunityDragon
- [v] `img` : champ, item, spell, icon, rune, emblem, role, ability
- [x] `isCompleted`, `me`, `summoner`, `findPlayer`, `history`, `tftHistory`, `gameDetails` (avec cache), `ranked`, `queueNames` (avec cache)
- [x] `rankScore`, `rankLabel`, `n`, `kda`, `cs`, `mmss`, `ago`, `settings` (valeurs par défaut)

## src/opgg.ts, src/opggParse.ts, src/opggParse.check.ts

- [v] `call` (cache, JSON ou format OP.GG), `champId`, `meta`, `mainPosition`, `opggBuild`, `maxOf`, `opt`, `RANK_FILTERS`
- [v] `parseOpgg`
- [v] test de `opggParse.check.ts`

## src/history.ts, src/history.check.ts

- [v] `isRemake`, `counted`, `filterGames`, `mergeGames`, `queuesIn`, `champsIn`, `formOf`, `parseTftGames`, `tftSummary`, `tftQueue`
- [v] tests de `history.check.ts`

## src/buildData.ts

- [x] `riotBuild`, `myBuild`, `bestBuild`, `pageName`
- [x] `applyBuild` : page de runes, sorts, item set
 (porté, pas essayé en vrai)

## src/valoApi.ts

- [v] `vdata`, `queueLabel`, `currentAct`, `vRank`, `seenPlayers`, `remember`, `vNames`, `vName` (par lots), `vHistory`, `vRRUpdates`
- [v] `vMatch` (un appel à la fois, reprise sur 429), `vStats`, `vTeamWon`, `vScore`, `vParties`, `vPresence`, `mkLabel`, `vWeaponKills`, `vEconomy`

## src/tftApi.ts

- [v] `tftData` (cache d'un jour), `tftDecks`

## src/util.ts

- [x] favoris, toast, `copyText`, `openUrl`, `checkUpdate`, `newer`, version

## src/main.tsx

- [x] routeur : historique, retour et suivant, boutons latéraux de la souris, Alt+← et Alt+→
- [x] recherche LoL : Ctrl+K, joueur `Nom#TAG`, champions, favoris, joueurs récents, flèches, Entrée, Échap
- [v] choix du jeu mémorisé, dernière page par jeu, Ctrl+1/2/3
- [v] barre de titre : logo, retour et suivant, onglets avec trait animé, état du client, boutons de fenêtre
- [x] suivi LCU toutes les 2 s : phase, ouverture auto de la partie, acceptation auto, import auto à la sélection
- [x] suivi VALORANT toutes les 3 s : session, présence, ouverture auto
- [x] notices « client fermé » avec tier list, agents ou decks en attendant
- [x] bandeau « partie en cours », toast, navigation clavier sans animation

## src/GamePicker.tsx

- [v] menu au survol et au clic, Échap, clic ailleurs
- [v] logos : `.ico` Riot sinon tracé SVG, illustrations, mention légale

## src/Account.tsx

- [x] menu du compte, `useAvatar`
- [v] choix d'avatar : collections, recherche, sections, photo perso recadrée à 256 px, retour à l'icône LoL

## src/Profile.tsx

- [x] `keepGames` (150 parties), `lpHistory` (200 points), `badges`
- [v] en-tête avec splash, copie du Riot ID, stats rapides, sous-onglets Aperçu et Champions
- [v] filtres par mode, champion et résultat, « Voir plus de parties »
- [x] session du jour avec copie du résumé, rangs, courbe LP au survol, forme récente, top champions
- [v] ligne de partie : icônes, KDA, KP, nombres, objets, badges, équipes cliquables
- [x] `Scoreboard`, tableau des champions, états vide, erreur et réessayer

## src/Match.tsx

- [v] `laneOpponent`, `score`, `grade`
- [v] en-tête, notes, résumé, courbe CS / or / XP au survol, objectifs, tableau des scores
- [v] carte de partage ; [x] copie de l'image (portée, pas refaite depuis le mode test)

## src/Builds.tsx, src/Jungle.tsx

- [v] sources OP.GG, tes parties, reco Riot avec repli
- [v] Faille et ARAM, rôles, filtre de rang, mode compact
- [v] runes, sorts, options d'objets, ordre des compétences avec variantes, matchups
- [x] interrupteurs d'import auto, bouton « Importer dans le client »
 (porté, pas essayé en vrai)
- [x] `route`, carte et étapes de la jungle, équipe et départ

## src/Champions.tsx, src/TierList.tsx

- [v] champions : recherche, classes, joués ou tous, tri, grille, tableau
- [v] tier list : recherche, favoris, rôles, tri, évolution, étoile

## src/Live.tsx

- [v] attente, équipes en sélection et en jeu, cartes joueur avec tags OTP et séries, part AD/AP, build compact

## src/Overlays.tsx, src/overlay.tsx, src/Widget.tsx

- [v] page Overlays : 9 cartes avec aperçu, interrupteur général
- [v] timers Dragon, Nashor, inhibiteurs, alertes à 30 s
- [v] prochaine compétence, ennemis morts, prochain objet, benchmark de lane, valeur des objets, stats, jungle path
- [x] réglages appliqués sans relancer

## src/Settings.tsx

- [v] toutes les options, source et rang des builds, joueurs croisés, raccourcis
- [x] données : cache TFT, parties, LP, recherches, avec confirmation
- [x] À propos : version, mise à jour, code source, attribution Slint

## src/Tft.tsx, src/TftProfile.tsx

- [v] decks méta : stats, unités, objets, traits, difficulté, reroll
- [v] profil : rang, résumé, lignes de partie, « Voir plus », erreur

## src/Updater.tsx

- [x] vérification toutes les 30 min, repli sur GitHub, téléchargement avec progression, signature minisign, installation
 (porté, pas essayé en vrai)
- [x] « Plus tard », erreur et page de téléchargement
 (porté, pas essayé en vrai)

## src/Valo.tsx

- [v] profil : en-tête, onglets Parties, Agents, Cartes, Armes, filtres de file, session du jour, rangs, chargement par lots
- [v] ligne de match : MVP, multi-kills, RR, promotion, réessayer
- [x] recherche des joueurs croisés
- [v] tier list des agents : rôles, tri, tier calculé
- [v] page de match : manches, économie, détail d'une manche, tableau des scores
- [v] partie en cours : sélection et en jeu, groupes, forme, rang moyen, joueurs masqués

## src/Skeleton.tsx

- [x] squelettes de chargement

## localStorage → fichier local

Toutes les clés `rw-*` : `rw-settings`, `rw-recent`, `rw-game`, `rw-route-*`, `rw-games-*`, `rw-lp-*`, `rw-fav-champs`, `rw-vseen`, `rw-avatar`, `rw-tft`.

- [v] `store.json` dans `%LOCALAPPDATA%\dev.ethone.ethra`
- [v] reprise au premier lancement depuis `EBWebView\Default\Local Storage\leveldb`

## Ajouts du natif (absents de Tauri)

- [v] stats par mode, maîtrise des champions, export CSV, top traits et unités TFT
- [x] lancement avec Windows, réduction en zone de notification, F5
- [v] overlays en jeu refaits : un panneau fin à droite, pastilles d'alerte et de compétence
- [x] installeur natif (`native/setup`) : passage depuis Tauri et Ethra, mise à jour, désinstallation, essayé seulement en dossier de test

## 20 fonctionnalités en plus (demandées le 10 novembre 2026)

`[ ]` à faire, `[x]` fait et vérifié (tests ou parcours e2e), avec la preuve dans « Fait ensuite » ou ici.

- [~] 1. Champions à éviter : pires taux de victoire récents (au moins 3 parties, moins de 45 %). La série en cours existait déjà ; ligne de la carte « Tendances », testée mais absente du jeu d'essai (aucun champion sous 45 %)
- [~] 2. Conseil de pause après 3 défaites d'affilée, sous la session du jour (test streak_counts_the_latest_run), réglable dans Réglages (« Conseil de pause ») ; pas vu à l'écran : le jeu d'essai finit sur des victoires
- [x] 3. Objectif de session : lien sous la session du jour qui passe de 2 à 3 puis 5 victoires, « Objectif : 1/3 victoires », « Objectif atteint » (test session_goals_cycle_and_report), joué dans le parcours e2e à 68 étapes ; objectif en LP non fait
- [x] 4. Taux de victoire par moment de la journée (nuit, matin, après-midi, soir, 3 parties au moins) : carte « Tendances » du profil (test insights_come_from_kept_games), vue dans le parcours e2e à 56 étapes
- [x] 5. Taux de victoire par jour de la semaine : carte « Tendances » du profil (test insights_come_from_kept_games), vue dans le parcours e2e à 56 étapes
- [x] 6. Temps de jeu du jour et des 7 derniers jours : carte « Tendances » du profil (test insights_come_from_kept_games), vue dans le parcours e2e à 56 étapes
- [x] 7. Plus longues séries de victoires et de défaites (les coéquipiers ne sont pas dans l'historique du client, seulement dans le détail de chaque partie) : carte « Tendances » du profil (test insights_come_from_kept_games), vue dans le parcours e2e à 56 étapes
- [x] 8. Durée moyenne des victoires et des défaites (même raison que 7) : carte « Tendances » du profil (test insights_come_from_kept_games), vue dans le parcours e2e à 56 étapes
- [x] 9. KDA des dernières parties : existait déjà (« KDA sur les 5 dernières, sur les 5 d'avant » dans la carte des dernières parties)
- [x] 10. Filtre de période sur l'historique (toute période, aujourd'hui, 7 jours, 30 jours ; test the_period_filter_keeps_recent_games), vu dans le parcours e2e à 75 étapes. Le filtre par champion existait déjà ; par joueur, impossible : l'historique du client ne contient que ton joueur
- [x] 11. Note personnelle sur une partie, champ sous le résumé de la partie, gardée sur ce PC (rw-note-<partie>), saisie dans le parcours e2e et relue dans store.json
- [x] 12. Joueurs épinglés : « Épingler ce joueur » sur le profil d'un autre joueur, en tête de la recherche (10 au plus, test pins_toggle_and_stay_short), vu dans le parcours e2e à 89 étapes ; sans leur rang (il faudrait interroger chaque profil à l'ouverture de la recherche)
- [x] 13. Comparer deux profils : carte « Toi face à … » sur le profil d'un autre joueur (parties, taux de victoire, KDA, CS par minute, champion favori ; tes parties gardées sur ce PC ; test two_profiles_compare_line_by_line), vue dans le parcours e2e à 90 étapes
- [x] 14. VALORANT : RR gagnés ou perdus par carte, colonne « RR » de l'onglet Cartes (le taux de victoire par carte existait déjà) ; même code que l'onglet Rôles vu dans le parcours e2e à 60 étapes
- [x] 15. VALORANT : RR par agent, colonne « RR » de l'onglet Agents (la précision par arme n'est pas dans les données de Riot, seulement la précision globale, déjà affichée)
- [x] 16. VALORANT : onglet « Rôles » du profil (parties, taux de victoire, K/D, ACS, ADR, HS, manches, RR par rôle), vu dans le parcours e2e à 60 étapes
- [~] 17. VALORANT : « Exporter mes parties VALORANT » dans Réglages, CSV des parties chargées (test valorant_csv_escapes_and_has_a_header) ; pas cliqué dans le parcours (fenêtre d'enregistrement de Windows)
- [x] 18. TFT : place moyenne par trait et par unité, existait déjà (« Tes traits les plus joués », « place moy. »)
- [~] 19. Raccourci global Ctrl+Maj+O pour montrer ou cacher les overlays (RegisterHotKey, message rappelant le raccourci, rappel sur la page Overlays) ; pas essayé : il faudrait appuyer sur les touches du PC
- [~] 20. Mini fenêtre de session toujours devant : lien « Mini fenêtre » de la session du jour, fenêtre Slint always-on-top (victoires et défaites, KDA, LP, objectif) mise à jour à chaque rendu du profil ; ouverte et fermée dans le parcours e2e à 95 étapes (hors écran), pas vue sur un vrai écran
- [~] 21. Récap de fin de partie (demandé par Rub19) : à la fin d'une partie League, ARAM compris, Kairo passe sur League et ouvre le récap (réglage « Ouvrir le récap à la fin de la partie », actif par défaut, ignoré en TFT). Grosse stat = celle où tu es le mieux classé des 10 (dégâts, soins, contrôle, dégâts réduits ou subis, kills, or, vision hors ARAM) ; en ARAM : dégâts subis, contrôle des foules, soins, score « Résistance » au lieu de « Farming ». Test `highlight_picks_the_stat_where_i_rank_best`, scénario e2e/recap.txt (10 étapes, phase EndOfGame, partie ARAM d'essai), parcours principal 95 étapes ; pas encore vu après une vraie partie
