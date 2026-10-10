---
title: 'Generateur de fond de carte PMTiles de recette'
type: 'feature'
ticket: '2'
created: '2026-10-09'
status: 'built'
baseline_revision: '541abc39722ac2608af83a015c9c8441cc2d2d95'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'entrée 1.3 doit afficher un fond de carte PMTiles, mais le projet ne télécharge aucune donnée (CLAUDE.md §2.4) et `/fixtures` est vide. Sans fichier de recette, ni la carte ni la garde hors ligne ne sont vérifiables.

**Approach:** écrire un générateur qui fabrique localement une archive PMTiles v3 valide, à tuiles PNG, couvrant la zone de Villeneuve-le-Roi aux zooms 10 à 14, et un vérificateur qui la relit avec la bibliothèque `pmtiles` officielle pour prouver sa conformité.

## Boundaries & Constraints

**Always:** le générateur ne fait aucune requête réseau et n'a aucune dépendance tierce — Node seul, avec son `zlib` intégré. L'archive produite respecte la spécification PMTiles v3 au point d'être relue par la bibliothèque `pmtiles` sans adaptation. Le fichier produit est versionné dans `/fixtures` (exception déjà posée dans `.gitignore`) et pèse moins de 200 Ko. Toute dépendance ajoutée voit sa licence écrite dans le message de commit (CLAUDE.md §8).

**Never:** ne pas télécharger de données, même une seule tuile. Ne pas produire de tuiles vectorielles MVT : hors périmètre, et le PNG suffit à prouver l'affichage. Ne pas toucher à `app/src/basemap.style.json` ni à `app/src/main.ts` — le branchement dans la carte est l'entrée 1.3. Ne pas ajouter de dépendance Rust : rien de ceci n'entre dans le moteur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Génération nominale | `npm run fixtures` sur un dépôt propre | `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, en-tête de 127 octets, magie `PMTiles`, version 3 | Aucune erreur attendue |
| Relecture par la bibliothèque | l'archive produite | `pmtiles` lit l'en-tête, les métadonnées et rend chaque tuile annoncée aux zooms 10 à 14 | Échec du script si une tuile annoncée est absente |
| Identifiant de tuile connu | z=12, x=3423, y=1763 | l'identifiant calculé vaut exactement 19078479, valeur donnée par la spécification | Échec si l'écart est non nul |
| Hors de l'emprise | une tuile en dehors de la boîte de Villeneuve-le-Roi | absente de l'archive, et la lecture renvoie « pas de tuile » sans planter | Retour vide, pas d'exception |
| Zoom hors bornes | z=9 ou z=15 | absent de l'archive, bornes `min_zoom` et `max_zoom` de l'en-tête respectées | Retour vide |
| Déterminisme | deux générations consécutives | deux fichiers identiques octet pour octet | Échec de la vérification si l'empreinte diffère |

</frozen-after-approval>

## Code Map

- `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0, licence relevée le 2026-10-09) et deux scripts, `fixtures` et `verify:fixtures`. L'entrée 1.3 promouvra `pmtiles` en dépendance de production quand l'application l'importera.
- `app/scripts/generate-basemap.mjs` -- **à créer.** Le générateur. Node pur, zéro dépendance tierce, `zlib` intégré pour le gzip du répertoire et `zlib.deflateSync` pour les PNG.
- `app/scripts/verify-basemap.mjs` -- **à créer.** Le vérificateur, seul endroit qui importe `pmtiles`.
- `app/scripts/check-offline.mjs` -- modèle à suivre pour le style : Node ESM, échec par `process.exit(1)` avec un message explicite. Ne pas le modifier.
- `app/src-tauri/icons/` -- les icônes du dépôt ont été produites par un encodeur PNG minimal (IHDR, IDAT `zlib.deflate`, IEND, filtre 0, RGBA 8 bits) ; réutiliser cette approche pour les tuiles.
- `fixtures/README.md` -- remplir le tableau des jeux de test avec le fichier produit, sa nature, son origine « généré » et son utilisateur, l'entrée 1.3.
- `app/src/main.ts` et `app/src/basemap.style.json` -- **ne pas toucher**, c'est l'entrée 1.3.

## Tasks & Acceptance

**Execution:**
- [ ] `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0) et les scripts `fixtures` et `verify:fixtures`, puis chaîner `verify:fixtures` dans `check` -- en premier, parce que le vérificateur de la tâche suivante importe ce paquet et que la recette doit protéger le fixture
- [ ] `app/scripts/generate-basemap.mjs` -- écrire l'encodeur PMTiles v3 : varint little-endian, identifiant de tuile par courbe de Hilbert, répertoire racine, métadonnées JSON, section de tuiles ; puis l'encodeur PNG des tuiles et l'assemblage du fichier -- c'est le livrable de l'entrée
- [ ] `app/scripts/verify-basemap.mjs` -- relire l'archive avec `pmtiles` et couvrir les six scénarios de la matrice d'entrées-sorties : en-tête, bornes de zoom, présence de chaque tuile annoncée, identifiant de référence de la spécification, tuile hors emprise, et déterminisme par double génération -- la preuve que l'archive est conforme, pas seulement qu'elle existe, et chaque commande vient avec son test (CLAUDE.md §8)
- [ ] `fixtures/README.md` -- documenter le fichier produit, l'emprise, les zooms couverts et la commande qui le régénère -- sans ça le fixture est un binaire orphelin dans le dépôt

**Acceptance Criteria:**
- Given un dépôt fraîchement cloné et `npm install` fait, when on lance `npm run fixtures`, then `fixtures/basemap-villeneuve-le-roi.pmtiles` est créé et pèse moins de 200 Ko.
- Given l'archive générée, when on lance `npm run check`, then la vérification passe au vert et affiche le nombre de tuiles et les zooms couverts.
- Given l'archive générée, when on l'ouvre avec la bibliothèque `pmtiles`, then son en-tête annonce le type de tuile PNG, la compression interne gzip, les tuiles non compressées, et les zooms 10 à 14.
- Given le générateur, when on l'exécute hors réseau, then il réussit — il ne fait aucune requête.
- Given une tuile corrompue ou tronquée introduite à la main dans l'archive, when on lance la vérification, then elle échoue avec un code de sortie non nul.

## Implementation Notes

**Conversion de Hilbert validée avant tout le reste**, comme le plan l'exigeait : la valeur de référence de la spécification (`z12/3423/1763 → 19078479`) est retrouvée, ainsi que les cinq premières valeurs de la table du §4.1. Le `rot` de l'algorithme classique prend `n`, pas `s` — se tromper là donne un ordre faux qui passe au zoom 1 et casse aux zooms supérieurs.

**Tuiles en gris + alpha (type de couleur PNG 4) au lieu de RVBA.** Mesuré pendant l'implémentation : le damier en RVBA pesait 1 351 octets par tuile contre 685 en gris + alpha, pour un rendu identique ici. Avec une cellule de 64 px, l'archive finale fait 26 Ko pour 35 tuiles, loin sous le plafond de 200 Ko du plan.

**L'emprise ne produit que 35 tuiles**, pas la centaine estimée au plan : la boîte de Villeneuve-le-Roi est petite (z10:1, z11:1, z12:4, z13:9, z14:20). Un seul répertoire racine suffit donc largement, et le garde-fou des 16 384 premiers octets n'est jamais approché.

**Le contrôle d'intégrité PNG a dû être refait.** La première version ne vérifiait que la signature et la présence d'`IEND` : l'inversion d'un octet au milieu d'un IDAT passait au vert, et seul le contrôle de déterminisme l'attrapait. `pngValide` parcourt maintenant les morceaux, recalcule chaque CRC avec `zlib.crc32` et décompresse l'IDAT ; la corruption est désormais nommée à la tuile près (`z14/8299/5646 : CRC du morceau IDAT invalide`). Deux contrôles indépendants la détectent.

**Dépendance transitive déclarée (CLAUDE.md §8) :** `pmtiles` 4.5.0 est en BSD-3-Clause et entraîne `fflate` 0.8.3, en **MIT**, relevé sur le paquet installé. Aucune autre dépendance n'entre.

**Le calcul d'identifiant est recoupé avec la bibliothèque**, pas seulement avec la valeur de référence : `zxyToTileId` de `pmtiles` est comparé au nôtre sur les 35 tuiles. Juge indépendant plutôt qu'auto-attestation.

**Déterminisme confirmé par construction :** `zlib.gzipSync` de Node écrit un MTIME nul, donc la sortie est stable d'une exécution à l'autre. Vérifié par le contrôle de double génération.

**Vérifié hors périmètre :** le diff ne touche ni `app/src/main.ts`, ni `app/src/basemap.style.json`, ni `core/`, ni `app/src-tauri/`, conformément au « Never » du plan. Le branchement dans la carte reste l'entrée 1.3.

**23 contrôles** dans le vérificateur, les six lignes de la matrice couvertes, toutes passées.

**Après revue (`bmad-review`, 3 lentilles, 21 constats) :** dix correctifs appliqués, deux constats reportés, cinq rejetés avec motif, un hors périmètre. Le vérificateur passe de 23 à 30 contrôles. Détail et preuves dans le journal de triage ci-dessous. Le correctif le plus utile est C1 : en supprimant mon propre décompresseur, le vérificateur exerce désormais le chemin par défaut de la bibliothèque — exactement celui que MapLibre utilisera à l'entrée 1.3.

## Plan Change Log

## Review Triage Log

**Passe 1 — `bmad-review`, lentilles `adversarial`, `edge-case-hunter`, `verification-gap`, exécutées en séquence par le lead faute de sous-agents (l'utilisateur les a refusés).** 21 constats déposés. Verdicts : 0 haut, 6 moyens, 13 bas, 1 indécidable, 0 faux. Recoupements entre lentilles conservés et signalés plutôt que fusionnés (A5↔V2, A7↔V1, A3↔C2).

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| C1 — le plan promettait une archive « relue sans adaptation », mais le vérificateur fournissait son propre décompresseur ; le chemin par défaut de la bibliothèque, celui que MapLibre empruntera à l'entrée 1.3, n'était jamais exercé | moyen | corrigé | Testé sans décompresseur : en-tête, métadonnées et tuile lus correctement. L'adaptateur est supprimé, la promesse devient vraie et le chemin de 1.3 est validé d'avance |
| V1 / A7 — aucun test ne prouvait que le vérificateur sache échouer ; la preuve par corruption n'existait que dans une session | moyen | corrigé | Recherche dans tout le dépôt : aucun fichier `*.test.*` ni `*.spec.*`, `tests/` ne contient qu'un README. Quatre contrôles ajoutés qui mettent le validateur en échec exprès, en mémoire |
| V2 / A5 — le centre et le zoom d'ouverture écrits dans l'en-tête n'étaient contrôlés par rien ; une inversion longitude/latitude au site d'appel passait inaperçue | moyen | corrigé | `grep` de `centerLon`, `centerLat`, `centerZoom` dans le vérificateur : 0 occurrence. Octets relus à la main : valeurs correctes, donc trou de vérification et non bug. Deux contrôles ajoutés |
| A1 — `tuilePng` ignorait le zoom : une tuile du zoom 10 et une du zoom 14 étaient identiques octet pour octet, alors que le commentaire du code affirmait rendre le zoom visible | moyen | corrigé | Mesuré par empreinte : 2 images distinctes pour 35 tuiles. Le fond dépend maintenant du zoom, 8 images distinctes (les zooms 10 et 11 n'ont qu'une tuile, donc une seule phase) |
| A4 — le contrôle intitulé « l'emprise annoncée est celle du repère de la carte » ne lisait jamais `basemap.style.json` ; il comparait l'en-tête à la constante qui l'avait écrit | moyen | corrigé | Le contrôle lit désormais le style et compare la boîte du polygone du repère ; l'ancien contrôle est conservé sous un intitulé honnête |
| A9 — `SourceTampon.getBytes` tronquait silencieusement une lecture dépassant la fin du tampon | bas | corrigé | Une archive tronquée cassait plus loin, de façon inexplicable. Lève maintenant une erreur nommant le décalage et les tailles |
| A3 / C2 — `fixtures/README.md` annonçait 22 contrôles dans le commit même qui en comptait 23 | bas | corrigé | Porté à 30, et la liste des contrôles complétée |
| A13 — la limite de précision du zoom 26 était gardée mais non documentée | bas | corrigé | Commentaire ajouté expliquant pourquoi, et que le MVP s'arrête au zoom 14 |
| E3 — une tuile manquante était rapportée « N sur M » sans la nommer, contrairement au contrôle PNG | bas | corrigé | La première tuile manquante est maintenant nommée |
| E1 — les autres membres de l'énumération de compression (inconnue, brotli, zstd) levaient une erreur opaque | bas | sans objet | Le code concerné a disparu avec le correctif C1 |
| A2 — 35 blobs stockés pour 8 images distinctes ; le format permet la déduplication | bas | reporté | 25,7 Ko ne gêne personne et la déduplication ajoute de la complexité à un encodeur écrit à la main. Voir `deferred-work.md` |
| A10 — les métadonnées JSON ne portent ni `bounds` ni `center` | indécidable | reporté | Impossible de trancher sans lire le code du protocole pmtiles de MapLibre. L'entrée 1.3 le tranchera de fait. Voir `deferred-work.md` |
| V3 — la commande de déterminisme listée au plan n'avait jamais été exécutée | bas | corrigé | Exécutée : deux générations, empreintes identiques. Le contrôle en mémoire reste dans le vérificateur |
| A6 — `encoderRepertoire` et `encoderEntete` sont exportés sans être exercés directement | bas | rejeté | Leur couverture transitive par `construireArchive` est réelle : les 30 contrôles échouent si l'un des deux se casse |
| A8 — une exécution réussie n'affiche qu'un décompte, pas la liste des contrôles | bas | rejeté | Le correctif ajoute une branche et du bruit en sortie pour un défaut que personne ne rencontre en usage courant |
| A11 — `leafDirectoryOffset` pointe sur les tuiles avec une longueur nulle | bas | rejeté | La spécification n'exige que la longueur nulle ; la bibliothèque lit l'archive sans broncher. Aucun préjudice nommable |
| A12 — `writeFileSync` et `mkdirSync` ne sont pas gardés | bas | rejeté | Script de développement : une trace de pile sur disque plein est un diagnostic acceptable, et le garde ajoute de la complexité |
| E2 — rien n'assure que l'archive ne contient aucune tuile en trop, au-delà de l'égalité des décomptes | bas | rejeté | Un surplus et un manque devraient s'annuler exactement pour passer, alors que chaque tuile attendue est par ailleurs servie et validée |
| E4 — `ecrireVarint` refuse les négatifs mais pas les non-entiers | — | hors périmètre | Non atteignable depuis les sites d'appel actuels, tous entiers par construction. Écarté par la règle de portée de la lentille |

**Résultat : 30 contrôles au vert après correction, contre 23 avant la revue.** Aucun constat haut. La revue a produit un correctif qui vaut à lui seul son coût : C1 valide d'avance le chemin de décompression que l'entrée 1.3 va emprunter.

**Limite assumée :** ces lentilles ont été exécutées par l'auteur du code. Elles apportent une diversité de méthode, pas d'indépendance de contexte. Le prompt autonome `…-plan-quick.md` reste disponible pour une revue réellement indépendante en session vierge.


## Design Notes

**Le piège de la spécification.** Le pseudocode de l'annexe A.1 de la spécification PMTiles v3 **omet le nombre d'entrées**, alors que le §4.2 l'exige comme première partie du répertoire et que la procédure de décodage du §4.3 le lit en premier. Le répertoire s'encode donc dans cet ordre : nombre d'entrées, puis les identifiants delta-encodés, puis les longueurs de série, puis les longueurs, puis les décalages — chaque valeur en varint little-endian.

**Identifiant de tuile.** `tile_id = (4^z - 1) / 3 + hilbert(2^z, x, y)`. Le premier terme est le cumul des tuiles des zooms inférieurs ; le second est la position sur la courbe de Hilbert d'ordre `z`. L'ordre attendu au zoom 1 est `(0,0)=1, (0,1)=2, (1,1)=3, (1,0)=4`, ce qui correspond à l'algorithme `xy2d` classique à rotations. La valeur de référence de la spécification, `z=12, x=3423, y=1763 → 19078479`, sert de test et doit être vérifiée **avant** d'écrire la moindre tuile.

**En-tête.** 127 octets : magie `PMTiles` sur 7 octets, version `3`, puis huit entiers non signés 64 bits little-endian (décalage et longueur du répertoire racine, des métadonnées, des répertoires feuilles, des tuiles), trois compteurs 64 bits, puis `clustered = 1`, compression interne `2` (gzip), compression des tuiles `1` (aucune, le PNG est déjà compressé), type de tuile `2` (PNG), zoom minimum et maximum, positions minimale, maximale et centrale sur 8 octets chacune — longitude puis latitude, chacune multipliée par 10 000 000 et écrite en entier signé 32 bits little-endian — et le zoom central.

**Décalages contigus.** Les tuiles étant écrites dans l'ordre des identifiants, `clustered` vaut 1 et chaque décalage sauf le premier s'encode en `0` quand il suit exactement la tuile précédente. Un seul répertoire racine suffit : l'emprise retenue produit bien moins d'entrées que la limite de 16 257 octets compressés.

**Emprise et zooms.** La boîte `[2.37, 48.71]`–`[2.46, 48.76]` est celle du repère déjà présent dans `app/src/basemap.style.json`, et la carte s'ouvre sur `[2.415, 48.735]` au zoom 12 d'après `app/src/main.ts`. Les zooms 10 à 14 encadrent donc l'ouverture par défaut. Au-delà de l'emprise, l'archive est vide, ce qui est assumé et documenté : c'est un fixture, pas un fond de production.

**Contenu des tuiles.** Un damier avec bordure et diagonale suffit à voir que le déplacement et le zoom fonctionnent. Pas de texte : rendre des chiffres demanderait une police embarquée, hors périmètre.

**Pourquoi Node et pas Rust.** Node est déjà requis pour construire l'interface, son `zlib` est intégré, et c'est le même interpréteur que la bibliothèque `pmtiles` qui sert de juge. Un générateur Rust imposerait des dépendances de compression au moteur, dont CLAUDE.md §3 exige qu'il reste sans dépendance.

## Verification

**Commands:**
- `cd app && npm install` -- expected: `pmtiles` installé, aucune autre dépendance ajoutée
- `cd app && npm run fixtures` -- expected: `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, taille affichée, moins de 200 Ko
- `cd app && npm run check` -- expected: typage strict, garde hors ligne et vérification du fixture tous au vert
- `cd app && npm run fixtures && sha256sum ../fixtures/*.pmtiles && npm run fixtures && sha256sum ../fixtures/*.pmtiles` -- expected: deux empreintes identiques (déterminisme)
- `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check` -- expected: le moteur reste intact et vert
