# Handoff : Kairo natif (Rust + Slint)

Branche `claude/rust-natif-2addd3`, code dans `native/`. Rien n'est publié, `main` n'a pas bougé.
Règles : réponses en français simple, zéro commentaire dans le code, commits avec `git -c user.name=Rub19 -c user.email=rub19.mailpro@gmail.com commit`, sans co-auteur IA, `git add` avec chemins précis.

## Fait

- Logique portée : LCU, live, OP.GG, VALORANT, TFT, builds, historique, mises à jour signées, `store.json` et reprise du `localStorage`.
- Toutes les pages en Slint : profils LoL, VALORANT et TFT, parties, builds et import, tier list, champions, partie en cours, overlays, réglages, choix d'avatar, sélecteur de jeu.
- Nouveautés : stats par mode, maîtrise des champions, export CSV, top traits et unités TFT, lancement avec Windows, réduction en zone de notification, F5.
- Logo définitif `kairo-1-instant.svg` : `native/assets/logo.svg`, PNG de 16 à 512, `icon.ico`. Il est branché sur l'exe, la fenêtre, l'en-tête, la zone de notification et l'installeur natif. Les raccourcis pointent vers l'icône de l'exe.
- Installeur refait le 9 novembre sur le modèle d'Ethone et d'Argus (`native/setup`) : accueil ou mise à jour, progression, terminé, erreur avec Réessayer, désinstallation avec option « supprimer aussi mes réglages », raccourci bureau, lancement avec Windows, `/S`, `/P /UPDATE /R /ARGS`, `--uninstall`, `--purge`. Il remplace Kairo Tauri sur place (même dossier, même `ethra.exe`) et retire l'ancien Ethra. L'app passe en 0.4.0 pour que la mise à jour Tauri 0.3.4 la propose. `release.ps1` construit puis emballe avec `kairo-setup --pack`.
- Overlays en jeu refaits le 9 novembre : un seul panneau fin et translucide à droite (180 px), sections séparées par un filet, morts en portraits, écarts de lane sur une ligne, alertes et compétence en petites pastilles. Stats perso et valeur d'équipe désactivées par défaut.

## Vérifié

- 43 tests passent. Les 2 tests `--ignored` sont passés aussi, avant les derniers ajouts : vraie signature de mise à jour, et lecture d'une copie du vrai `localStorage`.
- Icône lue dans `target/debug/ethra.exe` : le pixel central est bien celui du nouveau logo.
- Vrai exe hors écran, en lecture seule, avec le vrai réseau : tier list, champion, overlays (page), réglages, decks TFT.
- Client League réel : profil (panneau et rangs) et page de partie, mais la liste des parties était encore vide à ce moment-là.
- Jeu d'essai (`native/tools/fixture.py`, pas de vraies données) : liste des parties, VALORANT (profil, match, manches, sélection des agents), TFT, sélection des champions.
- Overlay avec fausse partie : affichage correct, alpha transparent dans le snapshot. Style Windows `0x80c01b8` : clics traversants, sans focus, toujours devant, hors barre des tâches.
- Un clic simulé par événement Slint ouvre bien l'onglet Tier list.
- Tailles : `ethra.exe` 19,8 Mo contre 14,7 Mo pour Tauri (WebView2 non compris), installeur 16,6 Mo.
- Mémoire : pic de 192 Mo en release sur 14 pages.

## Pas vérifié

- Installeur : essayé seulement en dossier de test (`KAIRO_SETUP_TEST=<dossier>`, plus `KAIRO_SETUP_SNAP` ou `KAIRO_SETUP_OFFSCREEN`) sur une fausse installation Tauri 0.3.4 et Ethra : passage au natif, mise à jour passive, silencieuse, désinstallation avec et sans réglages. Jamais lancé pour de vrai. Dernier : `native\release\Kairo_0.4.0_x64-setup.exe`, 16,8 Mo, SHA-256 `34C25305BC327892CC786B787ADB3A2EB8BEC83F945A0CB81B31504636F99565` (recompilé le 9 novembre avec la carte de partage et les pastilles de groupe, non signé).
- Nouveaux overlays : vus sur captures hors écran avec une fausse partie, pas par-dessus le vrai jeu.
- Zone de notification, boutons de fenêtre, redimensionnement, vraie souris, clavier (Tab, Ctrl K, Alt flèches), boutons latéraux.
- Transparence réelle de l'overlay par-dessus le jeu, et en vraie partie.
- Liste des parties avec le vrai client depuis le correctif, maîtrise des champions (forme de la réponse supposée), VALORANT réel.
- Import réel des runes, sorts et item set : jamais fait, aucun accord de Rub19.
- Copie d'image dans le presse-papiers : un seul essai réel, avant le mode test.
- Mémoire de la version Tauri : pas mesurée, donc la comparaison est incomplète.

## Reste à faire

1. Essais de Rub19 : lancer `native\release\Kairo_0.4.0_x64-setup.exe` sur sa vraie installation, puis une partie avec les overlays. Après son OK seulement : retirer l'ancien `setup/` Tauri.
2. Fait : `native/release.ps1 -Notes "…"` signe l'installeur avec `~/.tauri/kairo-updater.key` (ou `KAIRO_SIGN_KEY`) par le `tauri signer` du dépôt principal, puis écrit `release/latest.json` vers la release GitHub `v<version>`, comme `scripts/release.mjs`. Essayé avec une clé jetable, jamais avec la vraie clé. Sans clé, l'installeur est produit non signé. Publier reste la décision de Rub19.
3. Fait le 9 novembre : `docs/RUST_PARITY.md` revu ligne par ligne contre le code, 43 lignes vues marcher, 31 portées (dont 5 jamais essayées en vrai), aucune manquante. Recapturé après la partie : onglets CS, Or, XP bons ; bouton « Copier l'image » de la carte de partage corrigé (il était à gauche par-dessus les runes).
4. Refaire les essais avec le vrai client, mesurer Tauri, envoyer à la session principale la liste des essais de Rub19 avec le chemin de l'installeur.
5. Finitions : pastilles de groupe dans l'en-tête des équipes VALORANT passées à la couleur du groupe (comme les lignes et comme Tauri), vu sur capture avec le jeu d'essai (« Duo » en jaune) ; motion à relire.

## Pièges

- Un composant Slint sans `inherits Rectangle` et sans taille préférée fait 0 px : il ne s'affiche pas et ne reçoit pas les clics.
- Un nom de propriété peut masquer une fonction ou une valeur : `max`, `center`, `close`.
- Barlow n'a pas ▾ ▴ ▲ ▼ ★ ✕ : les dessiner avec `Path`.
- Pour une largeur dépendante de la fenêtre, utiliser `Theme.win-w`, sinon boucle de liaison.
- Les heredocs Bash abîment les backslashes : écrire les scripts avec Write.
- `Model` et `Image` ne sont pas `Send` : construire les lignes sur le fil UI avec `on_ui`.
- Tester toujours avec `KAIRO_READONLY=1` et `KAIRO_DATA_DIR` : sinon l'import auto pourrait écrire dans le client.
- Pendant un jeu : compiler en priorité basse, aucune fenêtre visible (`KAIRO_OFFSCREEN=1`).
- `git push` a échoué une fois avec `commit_refs` : réessayer.

## Commandes

Depuis `native/` :

```bash
cmd //c ".\\low.cmd build"
```

```bash
cmd //c ".\\low.cmd test"
```

```bash
powershell -File snap.ps1 -Script "start;game:lol;go:tierlist" -Wait 7000
```

```bash
python tools/fixture.py "$TEMP/fake1.json" None MENUS
```

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File release.ps1
```

Les images de `snap.ps1` vont dans `%TEMP%\kairo_snap`. Le jeu d'essai s'active avec `KAIRO_FAKE=<fichier>`, le faux live avec `KAIRO_FAKE_LIVE=<fichier>`. L'installeur sort dans `native/release/Kairo_0.3.4_x64-setup.exe`.
