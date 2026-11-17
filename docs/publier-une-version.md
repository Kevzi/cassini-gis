# Publier une version de Cassini (ticket 7.1)

Une étiquette `vX.Y.Z` fait tout : la recette construit le noyau Windows (gpu, Vulkan) sur
GitHub Actions, lance la garde hors ligne sur ce paquet, compile l'installeur en ligne, le
prouve contre un manifeste valide puis altéré, et publie la release avec le paquet du noyau (`cassini.exe` et ses DLL),
`cassini-installeur-en-ligne.exe` et `manifeste.json`. Rien n'est construit sur le poste du
propriétaire.

## Licence d'Inno Setup (outil de l'installeur)

L'installeur est compilé avec Inno Setup 6, celui déjà installé sur le poste GitHub (6.7.1 au premier essai ; le journal de la release affiche la version). Sa
licence est permissive : usage commercial permis, redistribution libre des installeurs produits,
et une copie modifiée d'Inno Setup doit être marquée comme telle. Cassini l'utilise sans le
modifier. Les installeurs produits embarquent un petit lanceur d'Inno Setup sous cette licence,
à lister dans les licences de la release (R7). **Adoption : à approuver par le propriétaire avant
la première étiquette** ; sinon, repli prévu à l'epic : un petit installeur maison hors du noyau.

## Noyau gpu et SDK Vulkan (ticket 7.6)

Le noyau livré est unique et gpu (Vulkan), livré avec les DLL de llama.cpp : la liste fermée est
dans `docs/paquet-du-noyau.md`. La recette installe le **SDK Vulkan 1.4.363.0** (adresse
officielle de LunarG et SHA-256 fixés dans `release.yml` ; la construction échoue si l'empreinte
diffère) en silencieux (`install --accept-licenses --default-answer --confirm-command`). Le SDK est
un outil de construction : il n'est jamais livré, et `vulkan-1.dll` vient du pilote de
l'utilisateur.

**Licence du SDK** (fichier `Licenses\LICENSE.txt` du SDK installé) : « The Vulkan SDK is comprised
of 100% open-source components. The majority of the materials are MIT or Apache 2.0 licenses. [...]
The SDK itself is not licensed. » Ce que Cassini livre venant du SDK passe par `ggml-vulkan.dll`
(shaders compilés par `glslc`, de licence permissive) ; à lister dans les licences de la release
(R7).

La compilation Vulkan se fait avec le générateur **Ninja** (`CMAKE_GENERATOR=Ninja`, comme sur le
poste du propriétaire) : avec le générateur Visual Studio, le sous-projet `vulkan-shaders-gen` de
llama.cpp échoue (`MSB8066`), à un ou plusieurs jobs, sur Visual Studio 2022 comme 2026.

### Essai du propriétaire du noyau gpu (étape hitl)

1. Lancer la recette à blanc (`gh workflow run release.yml`) et récupérer l'artefact de
   l'installeur complet ; l'installer.
2. Avec la carte : `set CASSINI_DEBUG_LLAMA=1`, lancer le noyau installé sur une phrase : le
   périphérique Vulkan est choisi et les couches sont posées sur la carte.
3. Sans la carte : `set GGML_VK_VISIBLE_DEVICES=aucune` puis la même phrase : le processeur est
   utilisé.

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
python -I installer/assembler_noyau.py --source C:\ct\release --sortie dist\noyau
python -I installer/manifeste.py --etiquette v0.1.0 --base-url http://127.0.0.1:8000 --sortie web\manifeste.json (Get-ChildItem dist\noyau -File).FullName
copy dist\noyau\* web\ ; python -m http.server 8000 --directory web
.\cassini-installeur-en-ligne.exe /MANIFESTE=http://127.0.0.1:8000/manifeste.json
```

## Ce que la vérification d'empreinte garantit, et ce qu'elle ne garantit pas

L'installeur compare le noyau téléchargé au SHA-256 du `manifeste.json` de la même release. Cela
protège d'un téléchargement corrompu ou d'un fichier remplacé seul ; cela ne protège pas d'une
release entièrement compromise, où manifeste et noyau changeraient ensemble. Le paramètre
`/MANIFESTE=` sert aux essais locaux ; l'installeur n'en a pas besoin à l'usage normal. Les
empreintes des installeurs eux-mêmes sont publiées sur la page d'accueil (ticket 12).

## Page d'accueil (ticket 12)

`site/index.html` et `site/faq.html` sont générés, jamais écrits à la main, depuis le manifeste de
la release et le fichier de l'installeur en ligne (son empreinte est calculée). Les liens pointent vers
la release de ce manifeste, pas vers `latest` : **régénérer la page à chaque release**, sans quoi
elle montre l'ancienne version :

```powershell
python -I installer/page_accueil.py --manifeste manifeste.json --installeur-en-ligne cassini-installeur-en-ligne.exe --sortie site
```

La vidéo `site/cassini-presentation.mp4` doit peser moins de 25 Mo (le générateur refuse sinon) ;
sans elle, la page est générée sans balise vidéo. Réencodage, hors du poste du dépôt (ffmpeg
requis) : par exemple `ffmpeg -i cassini-presentation.mp4 -c:v libx264 -crf 30 -preset slow -c:a aac -b:a 96k -movflags +faststart site/cassini-presentation.mp4`,
en baissant le CRF jusqu'à passer sous 25 Mo. Relire la page, puis pousser `site/` : le workflow
`site.yml` la publie.

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
   `cassini.exe run model.list` liste `cassini-composition-4b-0.2.0-Q4_K_M.gguf`.
4. Dans QGIS : Extensions, Installer depuis un ZIP, choisir l'archive du dossier `extension`.
5. Rejouer la démo : le panneau trouve le noyau et le modèle sans réseau.
6. Désinstaller (Paramètres, Applications) : le noyau et le modèle disparaissent.

## Porte 1 complète : le modèle facultatif (ticket 7.4)

L'installeur en ligne propose désormais le modèle : une case « Télécharger et installer le modèle »,
**cochée par défaut** ; en silencieux, `/MODELE=0` (sans) ou `/MODELE=1` (avec, défaut). Le modèle
est téléchargé depuis Hugging Face à l'adresse du manifeste, son SHA-256 est vérifié, il est posé
dans `%LOCALAPPDATA%\Cassini\bin\models` et sa déclaration `cassini-composition-4b.modele.json`
(`model`, `prompt`, `model_sha256`, la même forme que la porte 2) est écrite à côté, si bien que le
noyau le vérifie au premier chargement. Un modèle absent, injoignable ou dont un octet a changé
refuse **toute** l'installation (noyau compris, rien n'est posé) ; sans modèle (case décochée ou
ancien manifeste sans section `modele`), le noyau s'installe seul. Le dossier `models` n'est jamais
supprimé : ni à la mise à jour (un ancien modèle reste, et sa déclaration aussi : celle du nouveau
modèle prend alors son nom), ni à la désinstallation. Le modèle (environ 2,8 Go) est tenu deux fois
sur le disque pendant l'installation (téléchargement temporaire, puis copie à destination) : prévoir
environ 6 Go d'espace libre.

Le manifeste porte deux sections de plus après `fichiers` : `modele` (valeurs de
`installer/modele.json`, seule source) et `porte2` (nom versionné, adresse Hugging Face, SHA-256 et
taille de l'installeur complet). La recette de release prouve, sur le poste de GitHub et avec un
petit modèle de preuve servi en local : avec modèle, sans modèle (`/MODELE=0`), octet changé.

### Dépôt de la porte 2 sur Hugging Face : jeton `HF_TOKEN`

À la publication d'une étiquette, la recette dépose l'installeur complet et un fichier
`cassini-installeur-complet-<version>.empreintes.json` dans le dépôt Hugging Face du modèle, **si
le secret `HF_TOKEN` du dépôt GitHub existe** (jeton Hugging Face avec droit d'écriture sur
`KevziFar/cassini-composition-4b-GGUF`). Sans jeton, la recette affiche un avis et continue : le
manifeste annonce alors une adresse qui ne répond pas encore, à déposer à la main.

### Essai du propriétaire en VM (porte 1 complète)

1. VM Windows propre, réseau actif. Lancer `cassini-installeur-en-ligne.exe` en laissant la case
   cochée : `bin\models` reçoit le modèle et `cassini-composition-4b.modele.json`.
2. `cassini.exe run model.list` liste le modèle sans signaler `models.tampered`, puis rejouer la démo.
3. Relancer l'installeur avec `/MODELE=0` dans un autre dossier : le noyau seul, `models` vide.
4. Servir un modèle dont un octet est changé (`/MANIFESTE=` local) : l'installation est refusée et
   rien n'est posé.
5. Désinstaller : le noyau disparaît, le dossier `models` reste.

## Mettre à jour Cassini : le même programme que l'installeur en ligne

L'installeur en ligne sert aussi de mise à jour. Il copie une version de lui-même dans
`%LOCALAPPDATA%\Cassini\mettre-a-jour-cassini.exe` et crée, dans le menu Démarrer, le raccourci
**« Mettre à jour Cassini »**, qui le lance avec le paramètre `/MISEAJOUR`. Rien ne se met à jour
tout seul : aucune vérification automatique, tout se lance à la main.

Avec `/MISEAJOUR`, le programme lit le manifeste, compare sa version à celle de
`bin\installation.json` (trois nombres séparés par des points ; une version illisible vaut « mise à
jour proposée »), demande confirmation, vérifie le SHA-256 de chaque fichier, puis pose le noyau et,
si le modèle du manifeste n'est pas déjà dans `bin\models`, ce nouveau modèle **à côté** des anciens.
`installation.json` garde ses deux clés ; `installe_le` devient la date de la dernière mise à jour.

- Installé à jour (version du manifeste inférieure ou égale, modèle du manifeste déjà là) : message
  « Cassini est à jour », rien n'est téléchargé ni réécrit, code 0.
- Cassini non installé : message « Cassini n'est pas installé », rien n'est posé, code non nul en
  silencieux.
- Noyau en cours d'usage (QGIS ouvert) : message qui dit de fermer QGIS, rien n'est posé.
- Silencieux : `cassini-installeur-en-ligne.exe /MISEAJOUR /VERYSILENT` (pas de dialogue ;
  `/MANIFESTE=` et `/MODELE=0|1` fonctionnent comme à l'installation ; en mise à jour, le modèle n'est
  proposé par défaut que si un `.gguf` est déjà installé).

**Le nom installé du modèle porte sa version** (`installer/modele.json`, champ `nom_installe`, par
exemple `cassini-composition-4b-0.2.0-Q4_K_M.gguf`) : un nouveau modèle ne remplace jamais l'ancien
et aucun fichier de `models` n'est supprimé ni écrasé. Un nouveau modèle publié change donc le
nom dans `installer/modele.json` (et dans `cassini-hors-ligne.iss`, `NomModele`, et `release.yml`).
La déclaration de la porte 2 garde le nom fixe `cassini-composition-4b.modele.json` ; si elle nomme
déjà un autre modèle, la déclaration du nouveau prend le nom du modèle.

La recette de release prouve, avec un petit modèle servi en local : installation d'une version A,
`/MISEAJOUR` contre un manifeste B (version plus haute, modèle versionné) puis contre le même
manifeste (rien de réécrit), noyau verrouillé, Cassini non installé.

### Essai du propriétaire en VM (mise à jour)

1. Installer une version (porte 1), puis lancer « Mettre à jour Cassini » depuis le menu Démarrer
   contre un manifeste plus récent (`/MANIFESTE=` local) : la confirmation s'affiche en français.
2. Relancer le panneau dans QGIS : les deux modèles sont proposés, l'ancien reste choisissable.
3. Relancer « Mettre à jour Cassini » : « Cassini est à jour ».
4. Ouvrir QGIS (noyau chargé) et relancer la mise à jour : le message demande de fermer QGIS.

## Linux (7.7) : noyau processeur, deux portes en tar et script POSIX

Sous Linux, les deux portes sont une archive tar et **un seul script POSIX**, `installer/cassini.sh`,
qui installe sans droits root dans `~/.local/share/cassini` (le noyau dans `bin/cassini`, où
l'extension le cherche, avec `installation.json` et `models/`). Dépendances : `sh`, `tar`,
`sha256sum`, et `curl` ou `wget` pour la porte 1 seulement.

La recette a un job `linux` (Ubuntu 22.04, glibc 2.35) qui précède le job Windows. Il construit le
noyau **processeur** (`cargo build --release -p cassini`, sans `gpu`, sans `target-cpu=native` : pas de
Vulkan sous Linux, une base x86-64 générique, plus lente que le noyau Windows, à mesurer sur la VM),
lance la garde hors ligne (table des symboles importés de l'ELF), puis `installer/assembler_linux.py`
fabrique, par liste fermée :

- `cassini-noyau-linux-<version>.tar.gz` : le seul fichier `cassini` (porte 1, release GitHub) ;
- `cassini-complet-linux-<version>.tar` : `noyau/cassini`, `modele/<nom>.gguf`, `cassini.sh`,
  `VERSION` et `empreintes.txt` (porte 2, plus de 2 Gio : dépôt Hugging Face du modèle, comme la
  porte 2 Windows, pas la release GitHub).

L'assemblage échoue si le binaire n'est pas un ELF nommé `cassini` ou si `objdump -T` y lit une
version de glibc au-delà de 2.35. `installer/preuve_linux.py` prouve ensuite `cassini.sh` sur ces
archives, avec un serveur local et un petit modèle de preuve : installation valide (`run system.about`,
`model.list` sans `models.tampered`), octet changé dans le noyau puis dans le modèle (refus, rien
posé), mise à jour A vers B (ancien modèle gardé), « à jour » (rien téléchargé ni réécrit),
mise à jour sans installation (refus), et la porte 2 avec de faux `curl` et `wget` qui laissent une
trace (aucun appel réseau). Le manifeste gagne une section `linux` (`noyau` et `porte2` : nom,
adresse, SHA-256, et taille pour la porte 2), **après** `porte2` : l'installeur Windows ne la lit pas.
La release joint le tar du noyau et `cassini.sh`.

### Installer et mettre à jour

```sh
# Porte 1 (réseau) : télécharger cassini.sh depuis la release, puis
sh cassini.sh                      # noyau et modèle (--sans-modele pour le noyau seul)
sh cassini.sh --manifeste CHEMIN_OU_URL   # un autre manifeste (essai)

# Porte 2 (hors ligne) : déplier cassini-complet-linux-<version>.tar, puis, dans le dossier
sh cassini.sh                      # trouve noyau/ et modele/ à côté de lui, aucun appel réseau

# Mise à jour, à la main
~/.local/share/cassini/mettre-a-jour-cassini --mise-a-jour    # copie du script posée à l'installation
```

Mêmes règles que sous Windows (7.5) : chaque fichier est vérifié par `sha256sum` avant la pose et
rien n'est posé si l'un échoue (le message nomme le fichier) ; le noyau est posé par `mv` d'un fichier
temporaire du même dossier (QGIS peut rester ouvert) ; la version se compare à trois nombres ;
« à jour » ne télécharge ni ne réécrit rien (code 0) ; aucun fichier de `models` n'est supprimé ni
écrasé, un nouveau modèle se pose à côté sous son nom versionné avec sa déclaration `*.modele.json` ;
sans installation, `--mise-a-jour` refuse (code non nul). La confirmation n'est demandée que sur un
terminal ; `--oui` la saute.

### Essai du propriétaire sur Ubuntu 22.04 propre (hitl)

1. VM Ubuntu 22.04 propre, réseau actif : `sh cassini.sh` ; `~/.local/share/cassini/bin/cassini run
   system.about` répond et `run model.list` liste le modèle sans `models.tampered`. Mesurer la vitesse
   du noyau processeur.
2. Autre VM, réseau coupé : déplier le tar complet, `sh cassini.sh` ; même résultat.
3. Installer QGIS, l'extension depuis son ZIP, rejouer la démo : le panneau trouve le noyau.
4. Lancer `mettre-a-jour-cassini --mise-a-jour` contre une version plus récente : l'ancien modèle reste ;
   le relancer : « Cassini est à jour ».

## Licences livrées (R7)

Chaque paquet du noyau, Windows et Linux, contient deux textes, joints aussi à la release :

- `LICENCES-TIERCES.txt` : les crates du noyau (`core`, `ai`, `cli`, sans `cassini-app`, comme
  `deny.toml`) avec leur version et leur licence SPDX, triées, lues dans `cargo metadata` ; puis
  une liste fermée de composants hors Cargo : llama.cpp (MIT), le modèle Qwen3.5-4B modifié
  (Apache-2.0), Inno Setup (licence permissive, lanceur embarqué dans les installeurs Windows) et
  le SDK Vulkan (outil de construction, jamais livré). Les avis ColorBrewer et WMM2025 restent dans
  `NOTICE`.
- `FRONTIERE-GPL.txt` : copie de `docs/frontiere-gpl.md`, qui explique pourquoi l'extension (GPL) et
  le noyau (Apache-2.0) restent deux programmes séparés.

`python -I installer/licences.py --sortie dist/licences` les écrit (stdlib Python et `cargo metadata`
hors ligne, sortie identique d'une exécution à l'autre). Il échoue, en nommant la crate, si une crate
livrée n'a pas de licence déclarée ou n'a que des licences GPL, LGPL ou AGPL à offrir. Pour ajouter
ou retirer un composant hors Cargo (nouveau modèle, nouvel outil), modifier `HORS_CARGO` dans
`installer/licences.py`. La recette génère les deux fichiers avant l'assemblage : `assembler_noyau.py`
et `assembler_linux.py` (option `--textes`) échouent s'ils manquent. Sous Windows, ils sont dans
`bin` à côté de `cassini.exe` (le manifeste les liste, l'installeur en ligne les télécharge) ; sous
Linux, `cassini.sh` les pose dans `~/.local/share/cassini/bin`.
