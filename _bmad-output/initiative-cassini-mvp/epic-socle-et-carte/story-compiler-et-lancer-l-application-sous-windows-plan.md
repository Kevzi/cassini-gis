---
title: "Compiler et lancer l'application sous Windows"
type: 'chore'
ticket: '4'
created: '2026-10-10'
status: 'built'
baseline_revision: '9a0b2324fd598d53ff0d16c3c7be9567cbf71a73'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** rien n'a jamais été compilé pour Windows, et l'investigation montre que ça **échouerait dès `cargo build`** : `tauri-build` exige `app/src-tauri/icons/icon.ico` pour fabriquer la ressource Windows de l'exécutable, et ce fichier n'existe pas. Deux autres pièges attendent le propriétaire sur le même dépôt partagé entre WSL et Windows : `app/node_modules` ne contient que des binaires Linux, et `target/` porte 644 Mo d'objets Linux que `cargo` ne sépare pas par système hôte.

**Approach:** lever ici tout ce qui peut l'être — produire `icon.ico` et le déclarer — puis livrer au propriétaire une marche à suivre écrite, étagée du moins coûteux au plus coûteux, qui vérifie chaque prérequis avant de l'installer et distingue ce qui prouve quoi. Le propriétaire exécute, observe, et rapporte ; aucune étape ne suppose qu'il sache lire du code.

## Decisions

- **L'installeur (MSI, NSIS) est hors périmètre de cette entrée.** L'entrée demande une application qui s'ouvre, pas un paquet distribuable, et la construction d'un installeur télécharge WiX et NSIS depuis GitHub puis exige la fonctionnalité Windows VBSCRIPT. On s'arrête à l'exécutable, construit avec `--no-bundle`. Le chemin des ressources reste tout de même éprouvé : `tauri-build` recopie `bundle.resources` à côté du binaire, donc l'archive du fond de carte est servie comme dans une application installée.
- **Deux preuves distinctes, dans cet ordre.** `tauri dev` d'abord, parce qu'il donne le retour le plus rapide, mais il sert l'interface depuis un serveur local en HTTP : débrancher le câble ne la casse pas, donc ce n'est **pas** une preuve du hors ligne. L'exécutable construit embarque l'interface et constitue la vraie preuve. Les deux sont demandés.
- **Rien n'est supprimé par Cassini.** `node_modules` et `target/` doivent disparaître du poste Windows, mais c'est le propriétaire qui les supprime, en sachant que ces dossiers sont régénérables et ignorés par git.

</frozen-after-approval>

## Implementation Notes

**Pourquoi la route `oneshot`.** Le changement de code se réduit à un fichier binaire produit par un outil et à une ligne de configuration. Le livrable réel est une marche à suivre en français, calibrée pour quelqu'un qui ne code pas, et qui dépend entièrement de la recherche documentaire tenue dans cette session. La confier à un agent sans ce contexte la dégraderait.

## Review Triage Log

**Passe 1 — lentille `quick`, sous-agent en contexte vierge.** 7 constats, tous réels, tous corrigés sur place : la route `oneshot` les traite comme des correctifs, aucun n'exigeait de replanifier. Verdicts : 2 hauts, 4 moyens, 1 bas.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| R1 — le chemin de l'exécutable donné au propriétaire n'existe pas : Tauri ne renomme le binaire que si `mainBinaryName` est défini, sinon il garde le nom du paquet cargo, soit `cassini-app.exe` | haut | corrigé | **Vérifié dans le schéma livré avec le client** (`app/node_modules/@tauri-apps/cli/config.schema.json`) : `mainBinaryName` — « By default, Tauri uses the output binary from `cargo` » ; `productName` ne nomme que les paquets, les chemins d'installation et les métadonnées. La configuration ne portait pas `mainBinaryName`, `Cargo.toml` déclare `name = "cassini-app"`, sans `[[bin]]` ni `default-run`. C'était l'étape qui porte toute la preuve hors ligne : le propriétaire se serait arrêté là. Correctif : `mainBinaryName: "Cassini"` ajouté, **et** la marche à suivre accepte les deux noms, parce que je n'ai pas pu confirmer que le renommage s'applique quand on saute l'installeur |
| R2 — le prérequis WebView2 était conditionné à tort : la page affirmait que Windows 10 à partir de 1803 l'embarque | haut | corrigé | Relevé juste : 1803 est la version minimale de Windows que Tauri prend en charge, pas un seuil de préinstallation de WebView2. Seul Windows 11 l'embarque toujours. Comme cette entrée écarte l'installeur, rien ne l'aurait réparé : la fenêtre se serait ouverte blanche après vingt minutes de compilation. Correctif : vérification explicite par la clé de registre, aux deux emplacements, dans l'étape 1 |
| R3 — le prérequis le plus lourd, le compilateur C++, n'était jamais détecté, alors que l'étape 2 dit « n'installe que ce que l'étape 1 a signalé absent » | moyen | corrigé | Relevé juste, et c'est une contradiction avec l'approche annoncée par le plan. L'étape 1 ne sondait que `rustc`, `cargo`, `node`, `npm`. Correctif : détection par `vswhere.exe` sur le composant `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` |
| R4 — l'étape 6 demandait deux choses incompatibles : que la carte se laisse déplacer **et** que la ligne d'état dise « Carte prête » | moyen | corrigé | Vérifié dans `app/src/main.ts` : `carte.on("moveend", …)` remplace la ligne d'état par les coordonnées au premier déplacement. L'étape 5 posait l'ordre correctement, l'étape 6 non. Le propriétaire aurait rapporté un défaut qui n'en est pas un. Correctif : ordre explicité aux deux étapes, et dans ce qu'il doit rapporter |
| R5 — la collision WSL / Windows était présentée comme un coût unique | moyen | corrigé | Relevé juste et bien vu : les deux systèmes visent le même dossier `target`, donc chacun de mes lancements de tests depuis WSL réimpose au propriétaire une longue recompilation. Et symétriquement, son `npm install` Windows casse `npm run build` et `npm run tauri` de mon côté. Correctif : la page le dit, et propose de séparer les dossiers si ça devient pénible |
| R6 — le chemin absolu personnel du propriétaire, nom d'utilisateur compris, était écrit partout dans une page d'un dépôt public | moyen | corrigé | Relevé juste. Le dépôt est public sous Apache-2.0 et `docs/README.md` annonce la page de façon générique. Correctif : une variable `$projet` posée une fois, aucun chemin personnel restant — vérifié par recherche, zéro occurrence |
| R7 — le remède d'icône documenté écrasait des fichiers versionnés et ne reproduisait pas le fichier livré | bas | corrigé | Relevé juste : `tauri icon` régénère tout le jeu dans `icons/`, écrasant les PNG versionnés. Le `icon.ico` livré a été produit dans un dossier temporaire, seul le `.ico` étant recopié. Correctif : le remède dit de ne pas lancer la commande sur le dossier et décrit ce qui a réellement été fait. (Le relecteur ajoutait que la source 512 px donnerait un 256 px suréchantillonné : c'est faux, 512 vers 256 est une réduction. Le reste du constat tient) |

**Ce que la revue a évité.** Deux constats hauts, tous deux dans la documentation et non dans le code : le propriétaire aurait cherché un exécutable au mauvais nom à l'étape même qui conclut l'entrée, et une machine Windows 10 sans WebView2 lui aurait donné une fenêtre blanche après vingt minutes, avec pour seule aide une ligne de tableau. Sur une entrée dont le livrable est une marche à suivre, c'est la marche à suivre qu'il faut relire — pas le diff.

## Verification

**Commands:**
- `cd app && npm run check` -- expected: inchangé, tout au vert ; cette entrée ne touche ni le style, ni le code de l'interface
- `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check` -- expected: inchangé, tout au vert
- `python3 -c "import struct; d=open('app/src-tauri/icons/icon.ico','rb').read(); print(struct.unpack('<HHH', d[:6]))"` -- expected: `(0, 1, n)` — un ICO valide annonçant `n` images
- `grep -n 'icon.ico' app/src-tauri/tauri.conf.json` -- expected: le fichier est déclaré dans `bundle.icon`

**Manual checks (if no CLI):**
- La compilation et le lancement sous Windows ne peuvent pas être vérifiés depuis cet environnement : c'est tout l'objet de l'entrée. `docs/compiler-sous-windows.md` porte la marche à suivre et, pour chaque étape, ce que le propriétaire doit voir et ce qu'il doit rapporter en cas d'écart.
