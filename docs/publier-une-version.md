# Publier une version de Cassini (ticket 7.1)

Une étiquette `vX.Y.Z` fait tout : la recette construit le noyau Windows (processeur) sur
GitHub Actions, lance la garde hors ligne sur ce binaire, compile l'installeur en ligne, le
prouve contre un manifeste valide puis altéré, et publie la release avec `cassini.exe`,
`cassini-installeur-en-ligne.exe` et `manifeste.json`. Rien n'est construit sur le poste du
propriétaire.

## Licence d'Inno Setup (outil de l'installeur)

L'installeur est compilé avec Inno Setup 6, celui déjà installé sur le poste GitHub (6.7.1 au premier essai ; le journal de la release affiche la version). Sa
licence est permissive : usage commercial permis, redistribution libre des installeurs produits,
et une copie modifiée d'Inno Setup doit être marquée comme telle. Cassini l'utilise sans le
modifier. Les installeurs produits embarquent un petit lanceur d'Inno Setup sous cette licence,
à lister dans les licences de la release (R7). **Adoption : à approuver par le propriétaire avant
la première étiquette** ; sinon, repli prévu à l'epic : un petit installeur maison hors du noyau.

## Étapes du propriétaire pour `v0.1.0`

1. **Audit de l'historique** : relire tout l'historique git par une personne (secrets, données
   personnelles, rien de militaire, voir `CLAUDE.md` §2.9) avant d'ouvrir le dépôt.
2. **Passage en public** du dépôt GitHub `Kevzi/cassini-gis`, puis activer GitHub Pages
   (Réglages, Pages, source « GitHub Actions ») ; `site.yml` publie `site/` sur
   `https://kevzi.github.io/cassini-gis`.
3. Vérifier que `version` dans `Cargo.toml` vaut `0.1.0`, que la recette est verte sur `main`.
4. **Étiquette** : `git tag v0.1.0` puis `git push origin v0.1.0`. Si l'étiquette diffère de la
   version de `Cargo.toml`, la release échoue et rien n'est publié.
5. Sur un poste Windows, lancer l'installeur publié : `cassini.exe` est posé dans
   `%LOCALAPPDATA%\Cassini\bin`, avec `installation.json` à côté ; ouvrir la page à son adresse
   fixe et vérifier la page de l'avertissement de Windows.

## Tester l'installeur à la main

```powershell
python -I installer/manifeste.py --etiquette v0.1.0 --base-url http://127.0.0.1:8000 --sortie web\manifeste.json C:\ct\release\cassini.exe
copy C:\ct\release\cassini.exe web\ ; python -m http.server 8000 --directory web
.\cassini-installeur-en-ligne.exe /MANIFESTE=http://127.0.0.1:8000/manifeste.json
```

## Ce que la vérification d'empreinte garantit, et ce qu'elle ne garantit pas

L'installeur compare le noyau téléchargé au SHA-256 du `manifeste.json` de la même release. Cela
protège d'un téléchargement corrompu ou d'un fichier remplacé seul ; cela ne protège pas d'une
release entièrement compromise, où manifeste et noyau changeraient ensemble. Le paramètre
`/MANIFESTE=` sert aux essais locaux ; l'installeur n'en a pas besoin à l'usage normal. Les
empreintes des installeurs eux-mêmes seront publiées sur la page d'accueil (7.12).
