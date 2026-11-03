---
title: "L'export PDF prend une échelle demandée"
type: 'feature'
ticket: '38'
created: '2026-11-03'
status: 'built'
baseline_revision: 'ad273b627b292855df8372bc729effabcf119d63'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 2
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `layout.export_pdf` ne prend qu'un titre : « une carte au 1:25 000 » ne peut pas être tenue, le modèle refuse ou exporte à une autre échelle.

**Approach:** Un paramètre facultatif `scale` (dénominateur entier). L'extension pose cette échelle à la carte de la mise en page, centrée sur le centre de l'emprise que montre la carte de QGIS, et le résultat rend `scale`. Sans `scale`, cadrage et résultat inchangés. Une famille du générateur apprend « 1:25 000 », « 1/25000 », « au 25 000e », « 1:25k », sans ré-entraîner ici.

## Boundaries & Constraints

**Always:** Décision (propriétaire, 2026-11-03) : `scale` est un entier strictement positif ; bornes fixées ici à 100 ≤ `scale` ≤ 100 000 000, refusées par le schéma au-delà (`registry.invalid_params`) et par le geste (`layout.bad_params`). Le résultat ne porte la clé `scale` que si elle a été demandée : un journal ancien rejoue `identical: true` (comparaison stricte, `core/src/journal/summary.rs` `same_result`). Le centre est celui de la carte de QGIS ; sans carte (QGIS sans interface, sondes), celui de l'emprise des couches visibles. L'échelle relue sur la carte de mise en page doit valoir `scale` (arrondi entier), sinon `layout.export_failed` sans fichier.

Décision (propriétaire, 2026-11-03, après relecture) : **le centre est journalisé.** Avec `scale`, le résultat rend aussi `center`, le centre utilisé (un `Point`, dans le système du projet) ; au rejeu, le noyau repasse ce centre au geste, qui l'emploie au lieu de la carte de QGIS : le PDF rejoué montre le même endroit. Comme `leave_out`, ce paramètre n'est rempli que par le noyau au rejeu, jamais par l'appelant ni le modèle.

Décision (propriétaire, 2026-11-03, après relecture) : **un cadre vide est refusé.** Avec `scale`, si aucune couche visible ne touche l'emprise de la carte de mise en page, aucun fichier n'est laissé et l'erreur est `layout.empty`, avec un message qui dit de déplacer la vue sur les données.

**Never:** Toucher le garde-fou (`core/src/point.rs`), le jeu tenu à l'écart, la grammaire, `scoring.py`. Ré-entraîner (passe groupée après 37 et 38). Changer le format de page ou le cadrage sans `scale`. Ajouter une table de chaînes (aucune n'existe pour ce geste écrit à la main).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Échelle demandée | `{"title":"Carte","scale":25000}` | carte de mise en page à 1:25 000 centrée sur la carte de QGIS ; résultat `scale: 25000` et `center` | — |
| Sans échelle | `{"title":"Carte"}` | cadrage sur l'emprise des couches ; résultat sans clé `scale` ni `center` | — |
| Hors bornes / mal typé | `scale` 0, -5, 99, 100000001, 2.5, "25000", true | refus avant QGIS | `registry.invalid_params` (noyau), `layout.bad_params` (geste) |
| Pas de carte QGIS | geste sans interface, `scale` 25000 | centre = centre de l'emprise des couches visibles | — |
| Échelle non tenue | relue ≠ `scale` | aucun fichier | `layout.export_failed` |
| Journal ancien | ligne d'export sans `scale` | rejeu `identical: true` | — |
| Rejeu après déplacement de la vue | journal à `scale` 25000 et `center` C ; la carte de QGIS montre ailleurs | le geste reçoit `center` C et cadre sur C ; rejeu `identical: true` | — |
| Centre fourni par l'appelant | `{"title":"Carte","scale":25000,"center":{…}}` hors rejeu | le noyau l'ôte, comme `leave_out` ; un plan qui le nomme est rejeté | erreur de plan (« décidé par le noyau ») |
| Cadre vide | vue de QGIS sur une zone sans donnée, `scale` 25000 | aucun fichier | `layout.empty` |
| Garde-fou | « Exporte la carte au 1:25 000 », plan `scale: 25000` | plan inchangé (`scale` n'est pas un `Point`) | — |

</frozen-after-approval>

## Code Map

Point de départ (boucle 2) : `git stash apply` de `stash@{0}` (« 5.38 passe 2 (KEEP) »), qui contient déjà tout ce qui suit sauf le correctif du journal (ligne `core/src/journal/mod.rs` ci-dessous) ; tests verts avant d'ajouter. `stash@{1}` (passe 1) est déjà inclus.

- `core/src/registry.rs` l.54-58 (`leave_out_param`), l.127-131 -- ajouter, sur le même patron, `replay_param: Option<(String, String)>` (paramètre, champ du résultat journalisé), `#[serde(skip)]`, et le constructeur `.replay_param("center", "center")`.
- `core/src/journal/mod.rs` l.274-278 (session : ôter le paramètre de l'appelant) et l.815-819 (`Replay::run` : insérer `leave_out`) -- faire de même pour `replay_param` : ôté à l'appel ; **à l'écriture de la ligne (`call_logged`, l.~312), la valeur du champ est gardée sous une clé réservée de la ligne, `"replay": {"<paramètre>": <valeur>}`, même quand le résultat est résumé** (un résultat de plus de 200 octets, `journal/summary.rs` `SUMMARY_LIMIT`, n'est gardé que par taille et empreinte : le centre n'y serait plus) ; au rejeu, le paramètre est copié depuis `line["replay"][paramètre]`, sinon absent. Test : un export dont le résultat dépasse 200 octets (une dizaine de sources) rejoue avec le centre du journal et `identical: true`. `docs/canal-local.md` : décrire la clé `replay` de la ligne si les clés de ligne y sont décrites.
- `ai/src/plan.rs` l.233-238 (`catalog_entry` cache le paramètre) et l.489 (plan qui le nomme rejeté) -- y ajouter `replay_param`. Test l.949 : `center` absent du catalogue.
- `core/src/commands/layout_export_pdf.rs` -- `center: Option<Point>` en paramètre (`#[serde(default)]`) et en résultat (`skip_serializing_if`), `crate::point::Point` ; `.replay_param("center", "center")` ; descriptions ; test du contrat.
- `core/src/journal/tests.rs` -- la doublure `ProjectQgis` rend `center` (celui reçu, sinon un centre propre à sa génération) ; test : export à `scale` journalisé, rejeu avec une doublure dont la « vue » est ailleurs → le geste reçoit le `center` du journal, `identical: true`. Corriger la recopie de `scale: null` (passe 1, #6).
- `qgis-plugin/cassini_qgis/layout_export.py` -- `_read` : `center` reçu (dict x, y, epsg ; validé) prime sur `canvas_center` ; centre rendu en `Point` (`epsg` tiré de l'authid `EPSG:n` du projet ; un projet sans code EPSG refuse l'export à l'échelle, `layout.bad_params`) ; un `center` d'un autre EPSG est reprojeté par le fond. Le fond rend aussi l'emprise de la carte de mise en page (`carte.extent()`) : aucune emprise de couche visible ne la touche → `_discard` + `layout.empty`. `items_placed` posé après les contrôles (passe 1, #7). Extraire la reprojection d'un point en fonction du fond sans `iface` (`reproject_point(x, y, from_authid, to_authid)`), employée par `canvas_center` et pour `center` (passe 1, #3).
- `qgis-plugin/tests/test_layout_export.py`, `probe_layout_export.py` -- cas de la matrice ; la sonde ajoute un export à `center` donné en EPSG:4326 sur le projet en 2154 (reprojection réelle) et un cadre vide refusé.
- `qgis-plugin/cassini_qgis/summary.py` -- résultat absent : l'échelle lue dans les paramètres (passe 1, #4).
- `training/src/families.rs` `scale_text` -- ajouter « 1:25000 », « 1 : 25 000 », « au 25 000ème » (passe 1, #5) ; le test des formes écrites suit.

## Tasks & Acceptance

**Execution:**
- [x] Reprendre la passe 2 (`git stash apply stash@{0}`), tests verts avant d'ajouter
- [x] `core/src/journal/mod.rs`, `core/src/journal/tests.rs` -- clé réservée `replay` de la ligne ; test du résultat résumé (> 200 octets) -- le centre survit au résumé
- [x] Reprendre la passe 1 (`git stash apply`), tests verts avant d'ajouter
- [x] `core/src/registry.rs`, `core/src/journal/mod.rs`, `ai/src/plan.rs` -- `replay_param` : ôté à l'appel, rempli au rejeu depuis le résultat, caché au modèle, refusé dans un plan ; tests -- le mécanisme
- [x] `core/src/commands/layout_export_pdf.rs`, `core/src/journal/tests.rs` -- `center` ; tests de contrat et de rejeu (vue déplacée) -- le centre journalisé
- [x] `qgis-plugin/cassini_qgis/layout_export.py` + tests + sonde -- `center` reçu, `Point` rendu, cadre vide refusé, reprojection testée sous QGIS -- le geste
- [x] `summary.py`, `families.rs` + tests -- correctifs #4 et #5 de la passe 1
- [x] `docs/canal-local.md` (si le paramètre rempli au rejeu y est décrit), `qgis-plugin/README.md` -- docs

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent les tests de `cassini-core` et `cassini-ai`, then un export à `scale` journalise `center` (clé `replay` de la ligne, même résultat résumé), son rejeu repasse ce `center` au geste et rend `identical: true`, un `center` fourni par l'appelant est ôté, et le catalogue du modèle ne montre pas `center`.
- Given l'extension sans QGIS (doublure), when tournent ses tests, then un `center` reçu prime sur la carte de QGIS, et un cadre sans couche donne `layout.empty` sans fichier.
- Given la sonde sous QGIS, when elle tourne, then un `center` en EPSG:4326 est reprojeté dans le système du projet, l'échelle relue vaut 25000, et un cadre vide est refusé.

## Implementation Notes

- Bornes de `scale` dans le noyau : la validation passe par la désérialisation du type Rust
  (`registry.rs`, `validate`), pas par le schéma ; `#[schemars(range)]` seul ne refusait pas 0,
  99 ni 100000001. Ajout d'un `deserialize_with = "bounded_scale"` (patron `positive` de
  `analysis_buffer.rs`), constantes `MIN_SCALE`/`MAX_SCALE`.
- `scale: null` passe le schéma du noyau (`Option`) ; le geste le traite comme absent (aucune
  clé `scale` au résultat) plutôt que de le refuser.
- Résumé : « au 1:25 000 » (fr, espace simple comme le reste de `texts.py`), « at 1:25,000 » (en).
- Vérifié : `cargo test --workspace` vert (avec `LIBCLANG_PATH` pour `llama-cpp-sys-2`),
  `cargo fmt --check` et `cargo clippy --workspace --all-targets -D warnings` propres,
  `unittest discover qgis-plugin/tests` vert (357), `check_extension.py` propre, sonde
  `probe_layout_export.py` sous QGIS 3.44.15 : `RÉSULTAT : OK`, export relu à 25000.
- Boucle 2 (centre journalisé, cadre vide) : `replay_param` sur le patron de `leave_out`
  (ôté à l'appel, recopié au rejeu depuis `result[champ]`, caché au catalogue, refusé dans un
  plan). Le catalogue ôte aussi les `$defs` que seul un paramètre caché employait (le `Point`
  de `center` n'entre pas dans le long prompt). Un plan du modèle qui nomme `center` est rejeté
  par `judge`, puis le garde-fou (inchangé) le transforme en sa question, le `Point` n'étant pas
  écrit dans la demande : jamais rendu. Côté geste : `reproject_point` sur le fond (sans
  `iface`), `center` reçu ignoré sans `scale`, centre reçu non reprojetable → `layout.bad_params`,
  cadre vide vérifié après l'export sur `carte.extent()` (fichier effacé), variable de la barre
  d'échelle renommée (`scalebar`, elle masquait `scale`). `docs/canal-local.md` ne décrit pas
  `leave_out` : non modifié.
- Vérifié (boucle 2) : `cargo test --workspace` vert (core 282, ai 80), fmt et clippy propres,
  `unittest discover qgis-plugin/tests` vert (367), `check_extension.py` propre, sonde sous
  QGIS 3.44.15 `RÉSULTAT : OK` (centre EPSG:4326 ramené en 2154 à < 1 cm, échelle relue 25000,
  cadre à 500 km → `layout.empty`).
- Boucle 2, correctif du journal : `call_logged` écrit `"replay": {"center": …}` dans la ligne
  dès que le résultat porte le champ de `replay_param`, avant tout résumé ; `Replay::run` lit
  `line["replay"][paramètre]` (plus `result`). Test `the_logged_center_survives_a_summarized_result`
  (dix sources, résultat résumé sans `center`, rejeu avec le centre du journal, `identical: true`) ;
  `docs/canal-local.md` décrit la clé `replay` (il décrit déjà `layer_id`). Vérifié : `cargo test
  --workspace` vert (core 283, ai 80), fmt et clippy propres, `unittest discover` vert (367).
- Non fait : recette QGIS avec le modèle (la carte de QGIS réelle, `canvas_center`, n'est
  exercée que par la doublure ; la sonde tourne sans interface).

- Recette du 2026-11-03 (noyau `--features gpu` et extension installés ; sonde `probe_layout_export.py` sous QGIS 3.44.15 : `RÉSULTAT : OK`, centre arrondi). Sonde enchaînée sur une copie du projet réel : le modèle ajusté actuel refuse les trois phrases à l'échelle (attendu avant le ré-entraînement groupé) et exporte la phrase sans échelle ; avec Qwen3.5-9B, « … en PDF au 1/25000 avec le titre … » et « Garde les zones U du PLU et sors-moi la carte en PDF au 25 000e » donnent des plans à `scale: 25000`, exécutés, centre journalisé sous `replay`, rejeu `{"steps": 3, "identical": true}`. Deux défauts antérieurs relevés, reportés (deferred-work) : rejeu d'une étape déléguée en erreur, titre vide admis par le schéma. Centrage sur la vue réelle de QGIS : recette du propriétaire (README étape 4).

## Plan Change Log

- **Boucle 1 (2026-11-03), intent_gap.** Déclencheurs : constats #1 (centre non journalisé, rejeu infidèle sous `identical: true`) et #2 (cadre vide exporté comme un succès). Amendé : deux décisions du propriétaire dans l'intention, quatre lignes de matrice, Code Map et tâches (`replay_param`, `center`, `layout.empty`), correctifs #3 à #7 intégrés. État mauvais évité : un PDF rejoué qui montre un autre endroit en se disant identique ; un PDF vide qui annonce des couches. **KEEP** (passe 1, `stash@{0}`) : `scale` borné par `bounded_scale` et `MIN_SCALE`/`MAX_SCALE` (le schéma seul ne refuse rien) ; clé `scale` au résultat seulement si demandée ; `_scale_param` (null = absent) ; repli sur le centre des couches sans `iface` ; relecture de l'échelle → `export_failed` ; résumé « au 1:25 000 » ; famille `export_pdf_scale` (poids 3) et ses tests ; test du garde-fou ; étapes de README et de sonde.

- **Boucle 2 (2026-11-03), bad_plan.** Déclencheur : constat de l'orchestrateur à la relecture de la passe 2 — le journal ne garde un résultat de plus de 200 octets que par taille et empreinte (`summary.rs`), donc un vrai export (plusieurs sources + centre) perd son `center` et le rejeu recentre puis diverge ; les doublures, à résultats courts, ne le voyaient pas. Amendé : Code Map (`journal/mod.rs` : clé réservée `replay` de la ligne, lue au rejeu), tâches et critère. État mauvais évité : un rejeu qui échoue sur tout export réel à l'échelle. **KEEP** (passe 2, `stash@{0}`) : tout le reste de la passe 2 — `replay_param` (registre, catalogue, `judge`, `$defs` cachés), `center` en paramètre et résultat, geste (`reproject_point`, centre reçu prioritaire, projet sans EPSG refusé, cadre vide → `layout.empty`, `items_placed` après contrôles, `scalebar`), résumé, formes du générateur, sonde à 5 étapes ; et le KEEP de la boucle 1.

## Review Triage Log

Passe 1 (2026-11-03, lentilles thorough) : high 1, medium 3, low 10, false 1.

| # | Constat (lentille) | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Le centre pris sur la carte de QGIS n'est ni dans les paramètres ni dans le résultat : un rejeu après déplacement de la vue cadre ailleurs et dit `identical: true` (blind, edge, intent) | high | intent_gap | `_read` relit `canvas_center` à chaque exécution ; `Replay::run` ne passe que les paramètres du journal ; l'intention ne dit pas ce que fait le rejeu → question au propriétaire |
| 2 | Vue de QGIS sur une zone sans donnée : PDF vide rendu comme un succès, `layer_count` et sources annoncés (blind, edge) | medium | intent_gap | aucune vérification que le cadre croise une couche ; refuser ou accepter est un choix d'intention → question au propriétaire |
| 3 | `canvas_center` réel (reprojection) jamais exécuté, ni par les tests ni par la sonde sans interface (verification-gap) | medium | patch (après la boucle) | pré-vérifié ; extraire la reprojection pure et la sonder sous QGIS |
| 4 | Résultat absent : le résumé n'annonce pas l'échelle des paramètres (blind) | low | patch (après la boucle) | `summary._export_pdf` sort avant ; le docstring du module promet « phrase des seuls paramètres » |
| 5 | Formes « 1:25000 », « 1 : 25 000 », « 25 000ème » jamais tirées (blind) | low | patch (après la boucle) | `scale_text` n'a que 5 formes ; ajout direct de bras |
| 6 | La doublure du journal recopie `scale: null` (edge) | low | patch (après la boucle) | le geste réel l'omet ; filtrer `null` |
| 7 | `items_placed` posé avant l'échec de l'échelle (edge) | low | patch (après la boucle) | réordonner |
| 8 | SCR du projet vide : centre non reprojeté (edge) | low | reject | rare, la garde ajouterait une branche |
| 9 | Échelle énorme hors domaine du SCR → `export_failed` (edge) | low | reject | rare ; échec propre sans fichier |
| 10 | Relecture de l'échelle juste après `setScale` peu probante (blind) | low | reject | aucun mauvais résultat ; la sonde vérifie au moins le fond réel |
| 11 | Bornes recopiées en Python sans test croisé (blind) | low | reject | correctif = nouveau test transversal, dérive improbable |
| 12 | `scale: null` non testé côté noyau, journalisé tel quel (blind) | low | reject | rejoue à l'identique : le geste le traite comme absent |
| 13 | Garde-fou non testé avec point + échelle (blind) | false | reject | le garde-fou ne cherche que les `Point` dans le multiensemble ; un nombre de plus ne peut pas faire poser la question |
| 14 | Pas d'exemples négatifs (« barre d'échelle », « à 25 000 m ») (blind) | low | reject | la famille `export_pdf` (poids 6) enseigne déjà l'export sans `scale` |
| 15 | Espace insécable, ordre du résumé, description sans le repli, catalogue sans test `scale` (blind, gap) | low | reject | cosmétique ou sans effet observable |

Passe 2 (2026-11-03) : relecture de l'orchestrateur avant les lentilles ; high 1. Les lentilles thorough tourneront sur la passe 3.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 16 | `center` perdu quand le résultat de l'export dépasse 200 octets : la ligne n'en garde que taille et empreinte, le rejeu recentre et diverge | high | bad_plan | `journal/summary.rs` `SUMMARY_LIMIT = 200`, `summarize` ; `Replay::run` lit `line["result"][champ]` ; un export réel (sources + centre) dépasse 200 octets → boucle 2 |

Passe 3 (2026-11-03, lentilles thorough) : high 0, medium 3, low 12, false 2.

| # | Constat (lentille) | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 17 | Centre en flottant à 16-17 chiffres : `serde_json` sans `float_roundtrip` peut le relire au dernier bit près, le rejeu diverge (edge) | medium | patch | `core/Cargo.toml` sans la fonction ; arrondir x, y du centre à 1e-6 dans le geste (chemin rapide exact) |
| 18 | Le formulaire du panneau montre `center` (et déjà `leave_out`) en champs JSON bruts ; README étape 1 dit `title` et `scale` (blind, gap) | low | patch | `form.py` `CORE_FILLED = ("out",)` ; y ajouter `leave_out` et `center` |
| 19 | Résumé : un résultat sans `scale` masque l'échelle des paramètres (blind) | low | patch | `summary._export_pdf` ne lit `p` que si `r` est vide ; repli sur `p["scale"]` |
| 20 | Sonde : `reproject_point` à None → TypeError dans le rappel (edge) | low | patch | garde directe dans `step_wgs84` |
| 21 | Cadre vide jugé sur les emprises des couches, pas sur les entités (blind, edge) | medium | defer | décision « aucune couche ne touche l'emprise » tenue au sens des emprises ; un cadre dans un trou d'une couche éparse passe → deferred-work |
| 22 | `canvas_center` réel jamais exécuté, ni par test ni par sonde (gap) | medium | defer | pré-vérifié ; demande une fenêtre QGIS → recette README étape 4 en attendant |
| 23 | Élagage des `$defs` non transitif (blind, edge) | low | reject | un seul `$defs` caché aujourd'hui ; correctif = parcours d'atteignabilité |
| 24 | `center` écrit par le modèle → question du garde-fou plutôt que redemande (blind) | low | reject | `center` est caché au catalogue ; cas improbable |
| 25 | `cassini run` / MCP ne retirent pas `center` (blind) | false | reject | seule une `Session` atteint QGIS : `cli/src/main.rs:100` sans projet n'a pas QGIS ; `cli/src/mcp.rs:471` route le noyau seul ; `serve.rs:940` passe par `session.call` |
| 26 | Projet en degrés : échelle nominale (blind) | low | reject | QGIS calcule l'échelle d'une carte géographique ; rare dans le public visé |
| 27 | SCR du projet changé entre exécution et rejeu → `identical: false` (blind) | false | reject | divergence voulue : le projet a changé |
| 28 | Repli silencieux si la reprojection de la vue échoue (blind, edge) | low | reject | rare ; garde = nouvelle branche |
| 29 | Ligne `replay` sans `replay_param`, centre corrompu (blind) | low | reject | un centre invalide est refusé par la validation au rejeu, erreur à l'étape |
| 30 | Bornes recopiées, famille étroite, formes en espace insécable, sans contre-exemples (blind) | low | reject | reportés de #11 et #14 (carried) |

## Design Notes

AD-24 : sa « version de chaîne » vaut pour les lignes de la table fermée des primitives de traitement ; l'export est un geste écrit à la main, sans table ni version dans le code (recherche faite). Le changement de schéma déplace l'empreinte `registry_version` (`journal/canonical.rs`), ce qui suffit à le dater dans le journal.

Ordre dans le fond : `zoomToExtent(petit rectangle centré)` puis `setScale(scale)` — `setScale` garde le centre de l'emprise courante.

Centre journalisé : le geste rend le centre qu'il a employé ; le journal le garde dans `result` ; `Replay::run` le recopie dans `params.center`, et le geste le rend inchangé (même EPSG), donc le résultat comparé est identique. Le mécanisme suit `leave_out` (AD-11 : le rejeu reproduit, le noyau décide), sans rien de propre à l'export dans `journal/mod.rs`.

## Verification

**Commands:**
- PowerShell : `cargo test --workspace` (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` vers `outils-cassini\libclang\clang\native`) -- expected: vert
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- `python -m unittest discover qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde `probe_layout_export.py` sous le Python de QGIS : `RÉSULTAT : OK`.
- Recette QGIS (installer noyau + extension d'abord) : « Exporte la carte au 1:25 000 » ; le modèle actuel n'a pas appris la forme, un écart est attendu jusqu'au ré-entraînement groupé — à consigner. Rejouer après avoir déplacé la vue : même cadrage. Contrôle hors ticket : le propriétaire mesure le PDF à l'échelle.
