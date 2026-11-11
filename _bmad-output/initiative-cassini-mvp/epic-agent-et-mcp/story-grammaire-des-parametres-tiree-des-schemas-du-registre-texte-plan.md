---
title: 'Grammaire des paramètres tirée des schémas du registre, textes bornés'
type: 'feature'
ticket: '56'
created: '2026-11-11'
status: 'built'
baseline_revision: 'e912a11e58fd112fd3e5c42a522855fb4a326391'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problème :** `plan_grammar` ne contraint que l'enveloppe du plan et les identifiants de commande. Les paramètres sont du JSON libre, et une question ou un refus n'a pas de longueur bornée. Le modèle peut donc écrire un champ inconnu, un type faux (l'angle rendu en `0`) ou une valeur hors énumération, ou un refus qui ne finit jamais (S15, refus coupé).

**Approche :** construire la grammaire depuis le schéma JSON des paramètres de chaque commande offerte, tel que le modèle le voit dans le catalogue. Elle couvre les champs connus, les types, les `enum`/`const`, les champs requis et les références `step:k`/`step:k.champ`, là où `check_plan` les admet. Les textes de question et de refus ont une longueur bornée. `judge`/`check_plan` restent inchangés après la génération. La grammaire est identique pour un registre donné. Sa durée de construction et celle du décodage sont mesurées.

## Boundaries & Constraints

**Always :**
- La grammaire part du même schéma que le catalogue montré au modèle (`catalog_entry`). Les paramètres cachés (sorties, `leave_out`, rejeu, `epsg`/`squares` du carroyage `mgrs`) en sont absents, et les commandes dictées par le noyau aussi (`offered`).
- Toute sortie conforme aux exemples du prompt et au jeu d'entraînement engendré reste acceptée par la grammaire.
- Le format reste compact, sans espaces. Le noyau garde le dernier mot (AD-23) : rien n'est réparé.
- Le noyau tourne et se teste sans QGIS (AD-1). Aucune nouvelle dépendance.

**Never :**
- Toucher au texte du prompt, aux exemples, au générateur d'entraînement ou au modèle installé.
- Changer l'échantillonneur : pas de glouton d'abord, pas de grammaire paresseuse.
- Exprimer dans la grammaire les contrôles propres à `deserialize_with` (distance > 0, intervalle MGRS 1/10/100 km, EPSG…) : ils restent à `check_plan`.
- Filtrer les commandes selon la déclaration du modèle : c'est l'entrée 5.52, qui partira de cette grammaire.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Attendu | Erreur |
|---|---|---|---|
| Plan des exemples | les 8 exemples du prompt, tout le jeu engendré (graine 1) | acceptés tels quels | — |
| Champ inconnu | `"params":{"layer_id":"x","foo":1}` | impossible sous la grammaire | — |
| Type faux | `"azimuth":45` à la place de `{"value":45,"unit":"deg"}` | impossible | — |
| Hors énumération | `"unit":"km2"`, `"operator":"like"` | impossible | — |
| Requis absent | `analysis.buffer` sans `distance` | impossible | — |
| Référence | `"layer_id":"step:1.layer_id"` ; un élément de tableau de `Point` en `step:k.champ` | accepté (le type reste jugé par `check_plan`) | `check_plan` rejette une étape k ≥ courante |
| Texte long | question ou refus de plus de 240 caractères | impossible | — |
| Schéma non traduisible | construction inconnue du traducteur | ce champ seul retombe sur le JSON libre (`value`) | aucun repli sur le registre standard (test) |

## Décisions (propriétaire, 2026-11-11)

- **Borne des textes** : 240 caractères au plus pour une question et pour un refus (un caractère ou un échappement JSON compte pour un).
- **Ordre des clés** : l'ordre trié imposé, celui du prompt et de l'entraînement, dans les sous-objets aussi.
- **Repli** : un schéma non traduisible fait retomber ce seul champ sur le JSON libre (`value`). Un test exige zéro repli sur le registre standard.
- **Bornes** : seuls le signe des entiers (`minimum ≥ 0`) et `minItems`/`maxItems` sont traduits. Les autres bornes numériques, les bornes exclusives et `maxLength` restent à `check_plan`.
- **Barre de durée** : si la durée par phrase sort de la barre de 5.19, on s'arrête, on rend la mesure, et une entrée à part décide d'un échantillonneur glouton d'abord. Rien de tel dans ce ticket.

</frozen-after-approval>

## Code Map

- `ai/src/plan.rs:110-141`. `offered` et `plan_grammar`, à garder comme point d'entrée. `plan_grammar` délègue au nouveau module, et sa signature reste `&Registry -> String` pour que 5.52 la filtre.
- `ai/src/plan.rs:237-287` (`catalog_entry`, `hide_mgrs_core_fields`, `inline_raster_paths`, `strip_marks`). C'est la source unique du schéma vu par le modèle : la grammaire lit `catalog_entry(spec)["params"]`.
- `ai/src/plan.rs:541-605` (`judge`), à ne pas changer. `compose_plan_anchored:654` construit la grammaire à chaque phrase.
- `ai/src/plan.rs:1240-1310`. Les tests de grammaire existants (règles émises, commandes dictées absentes à l'octet) doivent rester verts ou être adaptés sans perdre leur portée.
- `core/src/plan.rs:98-122` (`parse_reference` : `step:<k>` ou `step:<k>.<champ>`, k ≥ 1 sans zéro de tête, un seul champ). `:233-343` : une référence est admise en tête de n'importe quel paramètre, et comme élément d'un tableau de `Point` (`point_items_schema:443`), nulle part ailleurs (`nested_error:420`).
- Schémas (schemars 1.2.2, draft 2020-12). On y trouve `$defs`/`$ref` sans récursion, et `Option` sous trois formes : `type:[T,"null"]`, `anyOf:[$ref,{type:null}]`, ou `$ref` avec `default` (champ seulement omissible). Les unités s'écrivent en `enum` ou en `oneOf` de `const`. Deux `oneOf` étiquetés par `kind` : `MapGrid` (`core/src/commands/layout_export_pdf.rs:72`) et `StyleSpec` (dictée). On trouve aussi `integer`/`number` + `minimum`/`maximum`/`exclusive*`, `minItems`/`maxItems`, `maxLength`, et `additionalProperties:false`. Pas de `pattern`, pas d'`allOf`. `Angle` (`core/src/angle.rs:52`) vaut `{value:number, unit?:"deg"|"gon"}`.
- **Ordre des clés** : `serde_json` sans `preserve_order`, donc les objets sont triés par clé. C'est l'ordre du prompt et des complétions d'entraînement (`training/src/prompt.rs:102`). La grammaire impose cet ordre trié, dans les sous-objets aussi.
- `ai/src/lib.rs:537-551,762-770`. `Timings.write` couvre déjà l'analyse de la grammaire par llama.cpp et le décodage contraint. Le sampler est créé à chaque appel.
- `cli/src/main.rs:319-350`. La ligne `cassini-timings` : ajouter les champs à la fin, sans réordonner (`qgis-plugin/tests/probe_recette.py` la lit).
- llama.cpp (`llama-cpp-sys-2 0.1.160`) accepte `x{m,n}`, sous un plafond de 2000 règles dépliées (`MAX_REPETITION_THRESHOLD`). Pas de répétition bornée imbriquée.
- Jeu d'entraînement (graine 1, 8600 exemples) : question de 173 caractères au plus, refus de 190 au plus. Nombres flottants seulement dans `x`, `y`, `value`, `z_factor`, souvent écrits en entiers.

## Tasks & Acceptance

**Execution :**
- [x] `ai/src/grammar.rs` (nouveau) -- traducteur schéma → règles GBNF, déterministe : règles nommées par commande et par `$defs`, noms assainis (les points y sont interdits), ordre stable. Il traduit les objets à clés triées (requis obligatoires, facultatifs omissibles, virgules justes), `string`, `integer` (sans signe si `minimum ≥ 0`), `number`, `boolean`, `null`, `enum`/`const`, `oneOf`/`anyOf` en alternatives, `type` en tableau, et les tableaux bornés par `minItems`/`maxItems`. Chaque paramètre de tête admet aussi la référence `step:`, et chaque élément d'un tableau de `Point` aussi. Les textes sont bornés. Le repli et les bornes suivent les Décisions. -- La règle vit à un seul endroit, testable sans modèle.
- [x] `ai/src/plan.rs` -- `plan_grammar` assemble l'enveloppe et les règles de paramètres, avec `step ::=` en alternative par commande. Il ajoute les tests de la matrice. -- Point d'entrée inchangé pour `compose_plan_*`, `duree_reel` et 5.52.
- [x] `ai/src/grammar.rs` (tests) -- un petit vérificateur d'appartenance, réservé aux tests, qui lit le GBNF émis (pas l'arbre interne). Il sert à prouver l'acceptation des exemples et de tout le jeu engendré (`cassini-training`, déjà en dépendance de dev) et le rejet des cas de la matrice. -- La seule preuve sans modèle que la grammaire n'exclut rien de ce que le modèle a appris.
- [x] `ai/src/plan.rs` (tests) -- grammaire identique sur deux constructions et sur deux registres standard, durée de construction mesurée et imprimée, aucun repli sur le registre standard. -- Exigences « identique » et « mesurée ».
- [x] `ai/src/lib.rs`, `cli/src/main.rs` -- ajouter à `Timings` la durée de création du sampler de grammaire, et à la ligne `cassini-timings` la durée de construction de la grammaire. -- Mesurer construction et décodage séparément.
- [x] `ai/tests/plan_reel.rs` (ou un test ignoré voisin) -- un essai ignoré qui crée `LlamaSampler::grammar` sur la grammaire du registre réel. -- Prouve que llama.cpp l'analyse (plafond de répétition, syntaxe).

**Acceptance Criteria :**
- Étant donné le registre standard, quand on construit la grammaire, alors chaque commande offerte a sa règle de paramètres, et aucune commande dictée n'y apparaît.
- Étant donné le modèle installé et la grammaire nouvelle, quand on rejoue `plan_reel`, `question_reel`, `ecart_reel` et la recette d'Annecy, alors aucun score ne baisse face à la même mesure faite sur `e912a11`. La durée par phrase reste dans la barre de 5.19.

## Design Notes

Une suite de clés triées dont certaines sont facultatives s'écrit en chaîne, chaque règle sachant si une virgule précède. Exemple illustratif (champs non vérifiés) : `distance` est requis et `layer_id` aussi, tandis que `segments` est facultatif :

```
p-analysis-buffer ::= "{" "\"distance\":" (d-Distance | ref) "," "\"layer_id\":" (string | ref) ("," "\"segments\":" (int-nn | ref))? "}"
ref ::= "\"step:" [1-9] [0-9]* ("." [a-zA-Z0-9_]+)? "\""
text ::= "\"" text-char{1,240} "\""
```

Si le premier champ est facultatif, la règle suivante se décline en deux variantes, avec ou sans virgule de tête. Les bornes numériques (`minimum`, `exclusive*`) et `maxLength` ne sont pas exprimées : c'est `check_plan` qui les tient.

## Verification

**Commandes** (sous WSL, `LIBCLANG_PATH` vers un libclang.so et `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`) :
- `cargo test -p cassini-ai` et `cargo test -p cassini-core` -- verts. Les tests du prompt à l'octet restent inchangés.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --check` -- sans avertissement.

**Avec le GPU, seulement après « TOUT FINI » dans `~/cassini-entrainement/nuit-apres-5-49.etat`** (le modèle installé, `--features cuda`) :
- `--ignored plan_reel question_reel ecart_reel`, d'abord sur `e912a11` puis sur la branche. Attendu : aucune baisse, `plan_reel` ≥ 80 % et `question_reel` ≥ 9/12.
- `duree_reel_par_phrase`, et la recette `probe_recette.py` sur `annecy.json`. Attendu : score ≥ 16/20 et pas en dessous de la mesure de base, phrases 2 à 10 sous 30 s sur GPU, temps d'écriture comparé à la base et noté dans `docs/mesure-duree-par-phrase.md`.

## Implementation Notes

- **Enveloppe des appels, deux ordres admis.** Les exemples du long prompt sont écrits par
  `Value::to_string`, clés triées : `{"calls":[…],"kind":"calls"}`. L'entraînement écrit `kind` en
  tête (`Plan` sérialisé). Pour accepter les deux « tels quels », `calls` admet les deux ordres
  (`steps` partagé). Question et refus n'ont qu'un ordre (`kind` est déjà trié en tête).
- **`GOOD` des tests de `plan.rs`** est écrit à la main dans un ordre non trié : la grammaire
  l'exclut désormais (vérifié par un test). Les tests de composition l'emploient sans grammaire
  (générateur scripté), ils restent valides.
- **Point** : la grammaire lit le schéma du catalogue avant le retrait des marques `x-cassini`
  (`model_params`, partagé avec `catalog_entry`, qui les ôte ensuite) ; le texte du catalogue est
  inchangé à l'octet.
- **Références imbriquées** : une chaîne libre imbriquée (`criteria[i].raster`) peut toujours
  porter le texte `step:k` ; la grammaire n'y voit qu'une chaîne, `check_plan` la refuse (test).
- **Mesures sans modèle** : registre standard, 348 règles, ~26 Ko, construite en ~15 ms (debug) ;
  llama.cpp la lit en 0,7 ms (essai ignoré `grammaire_reel`, vocabulaire seul, modèle
  `ajuste-nuit-11-10-fusion`). Aucun repli sur le registre standard. Tout le jeu engendré
  (graine 1, 8600) et les 8 exemples du prompt sont acceptés par le vérificateur.
- **Essai rapide sur processeur** (pas une mesure : machine chargée par l'entraînement en cours) :
  `cassini plan` en release, modèle `ajuste-nuit-11-10-fusion`, trois phrases (tampon, démo
  courte, PDF) : plans justes, clés triées.
- **Limites** : la borne de 240 caractères règle la question et le refus seulement ; un paramètre
  texte (`value`, `field`, chemins, titres) reste une chaîne sans borne. L'acceptation de la
  grammaire par llama.cpp n'est vérifiée que par l'essai ignoré `grammaire_reel`, à lancer à la
  main à chaque changement de `grammar.rs` ou d'un schéma.
- **Replis** : un champ libre (`true`, nœud sans type) compte comme repli ; un repli des paramètres
  entiers rend `object` et se note `<id> (tous les paramètres)` ; les replis d'une définition
  partagée sont reportés sous chaque chemin. En production, la liste est écrite une fois sur la
  sortie d'erreur, et `replis=<n>` finit la ligne `cassini-timings`.
- **Tableaux de `Point`** : détectés par `cassini_core::plan::point_items_schema` (rendue
  publique), la même fonction que `check_plan`.
- **Reste à faire** : les mesures GPU de la section Verification (`plan_reel`, `question_reel`,
  `ecart_reel`, `duree_reel_par_phrase`, recette d'Annecy, base `e912a11` contre la branche), à
  lancer après « TOUT FINI » dans `~/cassini-entrainement/nuit-apres-5-49.etat` ; la note dans
  `docs/mesure-duree-par-phrase.md` suit ces mesures.

## Plan Change Log

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 6, low 9, false 1, maybe-false 0. Corrigés 11 (patch, même implémenteur), reportés 3, rejetés 1.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Schéma `true` / nœud sans type rendus libres sans repli noté | medium | patch | comptés comme replis |
| Repli de commande en `value` (params non objet) | medium | patch | `object`, noté « tous les paramètres » |
| Références par élément seulement pour la forme directe d'un tableau de points | medium | patch | `point_items_schema` du noyau, rendu `pub` |
| Cache des `$defs` perd les replis | medium | patch | replis gardés et reportés |
| `minItems`/`maxItems` hors plafond de llama.cpp | medium | patch | repli noté |
| Replis jetés en production | medium | patch | sortie d'erreur une fois, `replis=` dans `cassini-timings` |
| Durée de la grammaire imprimée à chaque essai ; échantillonneur chronométré sans grammaire | low | patch | premier essai seul ; mesure seulement avec grammaire |
| Paramètres cachés autres que `out` non vérifiés absents | low | patch | tests `center`, `leave_out`, carroyage `mgrs` à `epsg`/`squares` |
| Référence sur un entier non testée, commentaire faux | low | patch | test `target_epsg` = `step:1.epsg`, commentaire corrigé |
| Échappements `~0`/`~1` d'un `$ref` | low | patch | décodés, sinon repli noté |
| Test qui écrit un fichier temporaire | low | patch | écriture retirée |
| Paramètres texte sans borne (S15 seulement réglé pour question et refus) | low | defer | écrit dans les notes et la doc ; décision du propriétaire limitée aux textes |
| Acceptation par llama.cpp vérifiée seulement par l'essai ignoré | medium | defer | il faut un vocabulaire GGUF en CI ; essai `grammaire_reel` à la main à chaque changement |
| Coût du décodage contraint non mesuré | medium | defer | critère d'acceptation 2 : mesures GPU contre e912a11 après l'entraînement |
| `probe_recette.py` lirait `cassini-timings` | false | reject | aucun lecteur dans le dépôt (vérifié) |

Hors relecture : `an_autonomous_call_while_another_session_holds_the_lock_is_read_only` (cli/tests/mcp.rs) a échoué une fois sous charge (machine occupée par la fusion et une autre recette), vert 3 fois sur 3 seul et dans la recette relancée ; test fragile antérieur, sans lien avec ce ticket, reporté dans `deferred-work.md`.

**Critère d'acceptation 2 encore ouvert** : `plan_reel`, `question_reel`, `ecart_reel`, Annecy et la durée par phrase, base e912a11 contre cette branche, sur GPU. Ne pas fusionner avant.
