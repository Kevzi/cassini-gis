---
title: 'Traceur : coord.convert de bout en bout, MGRS compris'
type: 'feature'
ticket: '2'
created: '2026-11-05'
status: 'built'
baseline_revision: 'bee726ba670314b6f5de0aaccb850bc89130bee7'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau ne sait convertir aucune coordonnée : une demande « donne-moi ce point en MGRS » ou une référence MGRS donnée par l'utilisateur ne peut entrer dans aucun plan, alors que toutes les commandes `geo.*` de l'epic en dépendent.

**Approach:** ajouter au noyau un module de géodésie (Mercator transverse de Karney, grille MGRS de la NGA), le type partagé `MgrsRef` sous la garde de provenance d'AD-23, et la commande `coord.convert` telle que la fige sa fiche du catalogue ; elle est servie sans autre câblage par `cassini run`, le canal et le serveur MCP, et le panneau affiche son résultat en clair.

## Boundaries & Constraints

**Always:** paramètres, résultat et erreurs exactement ceux de la fiche `coord.convert` et du type `MgrsRef` du catalogue (§3) ; systèmes acceptés 4326, 32601–32660, 32701–32760, tout autre → `geo.unsupported_crs` qui nomme `data.transform_point` ; zones spéciales 32V et 31X–37X ; vers MGRS on tronque ; latitude hors [-80, 84] en UTM/MGRS → `coord.out_of_range` ; toute trigonométrie par `libm` (mêmes octets sous Windows et Linux, rejeu) ; chaque jeu de référence cité (source, licence) dans le test ; tests sans QGIS ; `MgrsRef` marqué `x-cassini: mgrs` et accepté par la garde seulement s'il figure dans la demande (espaces et casse ignorés) ou vient d'un `step:` vers un champ marqué `MgrsRef`.

**Never:** aucune conversion d'une couche ni d'un autre système (Lambert-93 compris) ; pas d'UPS (bandes A, B, Y, Z) ; aucune conversion insérée en silence par le noyau avant un autre calcul ; pas de logique de calcul dans l'extension (affichage seulement) ; pas de famille d'entraînement ni de ré-entraînement (entrées 12 et 16) ; aucune dépendance ajoutée sans l'accord du propriétaire, licence dite.

**Decisions (propriétaire, 2026-11-05):** les calculs sont écrits par nous dans `core/src/geodesy.rs`, d'après les publications de Karney et la spécification MGRS de la NGA, avec le seul `libm` déjà accepté, sans nouvelle dépendance ; la démo dans QGIS se fait avec Qwen3.5-9B installé (prompt long tiré du registre), le modèle ajusté venant au ré-entraînement groupé (entrée 16) ; le plan est gardé entier (~3 000 tokens).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| WGS84 → UTM + MGRS | point 4326 en France, `to: [utm, mgrs]` | `utm` en 32631, `mgrs` canonique à 5 chiffres, conformes aux références | — |
| MGRS → WGS84 | `mgrs: "31u dq 48251 11750"` | coin sud-ouest de la case, `wgs84` conforme | — |
| Zone spéciale | point en Norvège (32V) ou au Svalbard (31X–37X) | zone de l'exception, pas la zone normale | — |
| Zone imposée | `utm_zone` voisine de la zone du point | coordonnées dans la zone imposée | plus d'une zone d'écart → `coord.out_of_range` |
| Précision | `mgrs_precision: 2` | 2 chiffres par axe, tronqués | hors 1–5 → erreur de paramètre |
| Entrées | ni `point` ni `mgrs`, ou les deux | — | `coord.one_input` |
| MGRS mal formé | bande I, chiffres impairs, carré inexistant, bande incohérente | — | `coord.invalid_mgrs`, partie fautive nommée |
| Système hors liste | point 2154 | — | `geo.unsupported_crs` nommant `data.transform_point` |
| Latitude polaire | lat 85 vers `mgrs` | — | `coord.out_of_range` |
| Garde, MGRS absent | plan du modèle avec une `mgrs` absente de la demande | question à provenance `guard` | — |
| Garde, MGRS repris | `mgrs: "step:1.mgrs"` | plan montré | — |

</frozen-after-approval>

## Code Map

- `core/src/point.rs` -- `Point` (l. 36, marque `x-cassini: point`), `is_point`/`admits_point`/`contains_point`, garde `guard_coordinates` (l. 526), `guard_rejected`, `invents_a_point` (l. 551), `walk` (l. 447), `reference_is_a_point` (l. 490), `request_numbers`. Étendre ici pour `MgrsRef` (nouvelle marque, collecte des MGRS littéraux du plan, comparaison à la demande normalisée, référence vers un champ `MgrsRef`). Ne pas toucher aux règles numériques existantes.
- `core/src/contract.rs` -- `check_contract` (l. 51) : vérifier qu'un `MgrsRef` n'y déclenche rien ; aucune règle nouvelle.
- `core/src/registry.rs` -- trait `Command` (l. 172), `CommandFailure::new(code, message)`, `CommandSpec::new::<P,O>(...).runs_in(RunsIn::Core).example(...).tests(&[...])`, auto-enregistrement `inventory::submit!` ; aucune liste à éditer.
- `core/src/commands/system_about.rs`, `terrain_slope.rs` -- modèles de commande et de tests en ligne (`Registry::standard().call`).
- `core/src/commands/mod.rs` -- déclarer le module de la commande.
- `core/src/lib.rs` -- déclarer le module de géodésie (repris par l'entrée 3).
- `core/tests/catalogue.rs` -- `NAMED_TYPES` contient déjà `MgrsRef` ; une fiche `planifiee` enregistrée fait échouer la recette : passer la fiche en `faite`.
- `ai/src/plan.rs` -- `offered()` tire prompt long et grammaire du registre : `coord.convert` y entre seul ; le prompt court (modèle ajusté) ne change pas.
- `qgis-plugin/cassini_qgis/summary.py` -- `_SENTENCES` (l. 285) ; sans entrée, le panneau écrit « …Terminée » sans les coordonnées.
- `qgis-plugin/tests/test_summary.py` -- tests des phrases du panneau.
- `cli/src/main.rs` (l. 804) -- modèle d'un test `cassini run` + rejeu.
- `training/` -- ne pas modifier ; ses tests doivent rester verts.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/geodesy.rs` -- WGS84, Mercator transverse de Karney (séries de Krüger d'ordre 6), zones UTM et exceptions, grammaire et lettrage MGRS (schéma AA), lecture tolérante, forme canonique -- module unique réutilisé par l'entrée 3.
- [x] `core/src/point.rs` -- type `MgrsRef` (chaîne validée, marque `mgrs`) et extension de la garde ; tests de la garde (absent, présent avec espaces/casse, référence d'étape, référence vers un champ non `MgrsRef`).
- [x] `core/src/commands/coord_convert.rs` (+ `mod.rs`, `lib.rs`) -- la commande, ses erreurs, ses tests : jeux de référence publics (GeographicLib, NGA) dont zones 32V et 31X–37X, et toute la matrice.
- [x] `cli/src/main.rs` -- test `cassini run coord.convert` sans QGIS, rejoué à l'identique depuis le journal.
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiche et tableau §4 en `faite`, bloc `MgrsRef` aligné sur le schéma engendré.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `tests/test_summary.py` -- phrase française et anglaise qui montre les formes rendues (« … en MGRS : 31UDQ4825111750 ; en UTM 31N : 448251 E, 5411751 N »).

**Acceptance Criteria:**
- Given le noyau sans QGIS, when on lance `cargo test`, then les références publiques sont retrouvées (UTM au millimètre, MGRS à l'identique) et la recette du catalogue passe.
- Given le serveur MCP lancé, when on liste ses outils, then `coord.convert` y figure et s'appelle.
- Given QGIS avec l'extension et le projet de test, when Claude envoie une phrase de conversion par la sonde de bout en bout, then un plan `coord.convert` est composé, validé, exécuté, et le panneau montre les coordonnées.

## Implementation Notes

- 2026-11-05 : relecture thorough (CLAUDE.md §8, entrée `high`) au lieu de la quick configurée. Sept correctifs appliqués ; écart accepté : un appel refusé est journalisé comme étape en erreur (AD-11), le test CLI le vérifie.
- 2026-11-05 : démo dans QGIS 3.44 (paquet `--features gpu`, Qwen3.5-9B, sonde `C:\ct\probe-11-2.py`) sur un projet vide : deux phrases sur trois donnent un plan `coord.convert` exécuté et affiché (≈ 36 s) ; la troisième (« 31u dq 4825 1175 ») rend une question du modèle, à l'entraînement (entrées 12 et 16) ; rejeu `{"steps":4,"identical":true}`. Sur le projet de test, `ai.prompt_too_long`, antérieur au ticket (reporté).
- 2026-11-05 : contrôle croisé MGRS demandé par le propriétaire, contre GEOTRANS de la NGA (paquet Python `mgrs` 1.7.0, hors du dépôt), sur 8 500 points (monde, 31V/32V, Svalbard, bords de zones et de bandes) : 8 490 identiques, 3 longitudes d'entrée invalides refusées des deux côtés, 7 écarts d'un mètre où la valeur exacte (PROJ) est à moins de 0,5 mm sous un mètre entier — GEOTRANS arrondit avant de tronquer, le noyau tronque la valeur exacte. Lecture : coin sud-ouest à moins de 0,71 m du centre de case de GEOTRANS. 30 références figées dans `geotrans_references_are_found`.
- 2026-11-05 : précisions ajoutées à la fiche du catalogue sans changer identifiants ni paramètres : coin sud-ouest, case qui touche la bande, arrondis ; champs du résultat omis quand la forme n'est pas demandée.

## Review Triage Log

Boucle 0 (relecture thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment) — high 0, medium 3, low 5, false 9, maybe-false 0 ; rejetés en low 6.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Zones inexistantes (32X, 34X, 36X) et case 31V hors de 0–3° E acceptées | medium | patch | essai : `32XNG1591463320` rend un point à 73,56° N au lieu de `coord.invalid_mgrs` |
| Point UTM d'entrée non contrôlé (abscisse aberrante, mauvais hémisphère) | medium | patch | essai : x = 5 000 000 en 32631 rend 52,2° E ; y = 9 889 470 en 32631 rend 89° N sans erreur |
| `without_mgrs` efface les nombres d'une lecture MGRS bien formée non reprise : régression de la garde sur les commandes existantes | medium | patch | essai : « Ligne de vue à 25m de 652000 6862000 … » rend la question, « à 25 mètres de … » non |
| Espace entre zone et bande (« 31 U DQ … ») non reconnu par la garde | low | patch | essai : plan `31UDQ4825111750` avec cette demande → question ; `Mgrs::parse` l'accepte |
| Test CLI : « rien n'est journalisé de plus » sans assertion | low | patch | `cli/src/main.rs`, aucune relecture du journal après l'appel refusé |
| Panneau : UTM arrondi au mètre à côté d'un MGRS tronqué (5411751 contre 11750) | low | patch | `summary.py` `_metres`, montré par le test |
| Fixture Python incohérente (entrée 3, -1 ; WGS84 rendu -2,29, -48,86) | low | patch | `test_wgs84_shows_latitude_then_longitude_with_hemispheres` |
| Démo dans QGIS sur le projet de test : `ai.prompt_too_long` avec Qwen | medium | defer | l'ancien noyau installé échoue de même (rapport de l'implémenteur) : antérieur à ce ticket |
| Sortie UTM/MGRS dans la zone normale pour une entrée d'une zone voisine | false | rejeté | la fiche figée dit « absente, la zone normale du point » |
| `utm_zone` sans effet sur la sortie MGRS | false | rejeté | la fiche : « zone UTM imposée à la sortie `utm` » |
| MGRS → MGRS rendu à 5 chiffres | false | rejeté | la fiche fixe le défaut à 5 ; GeoConvert fait de même |
| Lecture prolongée par des chiffres suivants (« … 10 m ») | false | rejeté | `written_mgrs` pousse une lecture à chaque groupe : le préfixe exact est trouvé |
| Paramètre admettant `Point` et `MgrsRef` à la fois | false | rejeté | aucun schéma du registre ne le fait |
| Panneau : conversion partielle montrée comme complète | false | rejeté | le noyau rend toutes les formes demandées ou une erreur |
| `MgrsRef` non validé à la désérialisation | false | rejeté | catalogue §3 : la commande lit la grammaire et rend `coord.invalid_mgrs` (AD-21) |
| Plan absent du diff | false | rejeté | exclu exprès : c'est le fichier des déclarations de la relecture |
| Canal non testé en propre | false | rejeté | `cli/tests/parity.rs` passe les exemples de toute commande `core` par run, serve et MCP |
| Bande différente à la relecture d'une case à cheval ; équateur à 1 µm ; bord courbe de bande ; `cell_size` > 5 par champs publics ; `series()` recalculée ; descriptions FR/EN de longueurs différentes ; tests bas niveau de `geodesy.rs` | low | rejeté | rares ou inatteignables par la commande, et la correction ajoute de la complexité |

## Design Notes

Garde : un MGRS littéral du plan est normalisé (majuscules, sans espaces, zone sur deux chiffres) et doit se trouver dans la demande normalisée de la même façon ; il n'utilise aucun nombre du multiensemble de `request_numbers`, mais les chiffres de la demande qui lui appartiennent ne doivent pas servir de x ou de y à un `Point`.

Lecture MGRS → point : coin sud-ouest de la case à la précision donnée (convention NGA) ; la latitude obtenue doit tomber dans la bande annoncée, sinon `coord.invalid_mgrs` (« bande »).

Vérification croisée possible pendant le développement : PROJ de QGIS sous Windows pour WGS84 ↔ UTM, jamais dans les tests.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout vert, dont `catalogue` et les tests de `training`
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde de bout en bout dans QGIS 3.44 (vrai `cassini.exe`, Qwen) : plan `coord.convert` exécuté, coordonnées affichées, journal rejouable.
