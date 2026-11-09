# Handoff : Kairo natif (Rust + Slint)

Branche `claude/rust-natif-2addd3`, code dans `native/`. Rien n'est publié, `main` n'a pas bougé.
Règles : réponses en français simple, zéro commentaire dans le code, commits avec `git -c user.name=Rub19 -c user.email=rub19.mailpro@gmail.com commit`, sans co-auteur IA, `git add` avec chemins précis.

## Fait

- Logique portée : LCU, live, OP.GG, VALORANT, TFT, builds, historique, mises à jour signées, `store.json` et reprise du `localStorage`.
- Toutes les pages en Slint : profils LoL, VALORANT et TFT, parties, builds et import, tier list, champions, partie en cours, overlays, réglages, choix d'avatar, sélecteur de jeu.
- Nouveautés : stats par mode, maîtrise des champions, export CSV, top traits et unités TFT, lancement avec Windows, réduction en zone de notification, F5.
- Logo définitif `kairo-1-instant.svg` : `native/assets/logo.svg`, PNG de 16 à 512, `icon.ico`. Il est branché sur l'exe, la fenêtre, l'en-tête, la zone de notification et l'installeur natif. Les raccourcis pointent vers l'icône de l'exe.
- Installeur natif de base dans `native/setup` (Rust + Slint, sans WebView) : `/S`, `/P /UPDATE /R`, `--uninstall`, `--purge`.

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

- Installeur : compilé, jamais lancé. Ne pas le lancer depuis Claude, il irait dans le conteneur MSIX. Le tester seulement en dossier de test.
- Zone de notification, boutons de fenêtre, redimensionnement, vraie souris, clavier (Tab, Ctrl K, Alt flèches), boutons latéraux.
- Transparence réelle de l'overlay par-dessus le jeu, et en vraie partie.
- Liste des parties avec le vrai client depuis le correctif, maîtrise des champions (forme de la réponse supposée), VALORANT réel.
- Import réel des runes, sorts et item set : jamais fait, aucun accord de Rub19.
- Copie d'image dans le presse-papiers : un seul essai réel, avant le mode test.
- Mémoire de la version Tauri : pas mesurée, donc la comparaison est incomplète.

## Reste à faire

1. Installeur custom de zéro demandé par Rub19 : partir de `native/setup` et prendre comme modèle `desktop/native` sur la branche `claude/ethone-rust-natif-dba200` du dépôt dashboard. Il doit avoir accueil ou mise à jour, progression, terminé, erreur avec Réessayer, désinstallation, options de raccourci et de lancement avec Windows, modes `/S` et `/P /UPDATE /R`, migration depuis `%LOCALAPPDATA%\Kairo` et `dev.ethone.ethra`. Ajouter l'option de lancement avec Windows (absente pour l'instant). Tester uniquement en dossier de test. Puis retirer l'ancien `setup/` Tauri.
2. Adapter `scripts/release.mjs` ou le remplacer par `native/release.ps1` avec signature minisign. Ne rien publier.
3. Passer tout le contenu de `docs/RUST_PARITY.md` en revue et cocher les cases : aucune n'est cochée. La note d'état en tête du fichier est périmée, la plupart des bugs listés sont corrigés.
4. Refaire les essais avec le vrai client, mesurer Tauri, envoyer à la session principale la liste des essais de Rub19 avec le chemin de l'installeur.
5. Finitions : pastille de groupe VALORANT en accent au lieu de sa couleur, motion à relire.

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
