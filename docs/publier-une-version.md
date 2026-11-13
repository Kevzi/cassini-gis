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

## Porte 2 : l'installeur complet hors ligne (ticket 7.2)

Sur la même étiquette (ou un lancement manuel du workflow, sans publication), la recette
construit aussi `cassini-installeur-complet.exe`, un seul fichier qui contient `cassini.exe`, le
modèle et l'archive de l'extension QGIS. Elle échoue avant toute publication si l'archive de
l'extension pèse 25 Mo ou plus ou contient un binaire, si le modèle est introuvable ou si son
SHA-256 diffère de celui de `installer/modele.json`, ou si le fichier dépasse 4 Go (limite d'Inno
Setup pour un seul fichier : 4,2 Go ; repli prévu : zip et `install.cmd`). Le modèle est
téléchargé depuis Hugging Face (`KevziFar/cassini-composition-4b-GGUF`), jamais construit ici.

Le fichier dépasse les 2 Gio d'une release GitHub : il est livré comme **artefact du workflow**.
Récupération : onglet Actions du dépôt, l'exécution de la release, section « Artifacts »,
`cassini-installeur-complet-<étiquette>` (conservé 14 jours ; il contient aussi
`cassini_qgis-<version>.zip`). La recette prouve déjà l'installation silencieuse sur le poste de
GitHub avec une règle de pare-feu qui interdit au fichier toute sortie réseau, mais seule la VM
ci-dessous constitue la vraie preuve « machine débranchée ».

Les deux installeurs partagent le même `AppId` (voulu) : un seul Cassini par utilisateur, chaque
porte met à jour l'autre, et le désinstalleur ne retire que ce que le dernier installeur a posé.

### Essai du propriétaire en VM (étape hitl)

1. VM Windows propre, **réseau coupé** (adaptateur désactivé), puis y copier le fichier.
2. Lancer `cassini-installeur-complet.exe` : `%LOCALAPPDATA%\Cassini\bin` reçoit `cassini.exe`,
   `installation.json` et `models\` (le modèle et sa déclaration) ; `%LOCALAPPDATA%\Cassini\extension`
   reçoit l'archive de l'extension.
3. Dans une console : `%LOCALAPPDATA%\Cassini\bin\cassini.exe run system.about` répond, et
   `cassini.exe run model.list` liste `cassini-composition-4b-Q4_K_M.gguf`.
4. Dans QGIS : Extensions, Installer depuis un ZIP, choisir l'archive du dossier `extension`.
5. Rejouer la démo : le panneau trouve le noyau et le modèle sans réseau.
6. Désinstaller (Paramètres, Applications) : le noyau et le modèle disparaissent.

Hors périmètre ici (7.4) : déclaration `*.modele.json` avec empreintes, hébergement de ce fichier
sur Hugging Face, noyau gpu (7.6).
