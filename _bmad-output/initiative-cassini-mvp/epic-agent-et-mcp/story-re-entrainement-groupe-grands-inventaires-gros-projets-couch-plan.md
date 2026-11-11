---
title: 'Ré-entraînement groupé : grands inventaires, gros projets, couches proches'
type: 'feature'
ticket: '49'
created: '2026-11-10'
status: 'built'
baseline_revision: 'd0d2f522d7737ba1bc2502b5e4c22caa0249c6c4'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le jeu engendré après les fusions de 5.46–5.48, 10.28–10.31, 2.12 et 5.45 ne tient
plus sa barre de gros projets (1 962 gardés, 22,8 % < 23 %) : depuis le seuil du tri à 17 000
caractères (5.48), les gros projets vus en entier dont le prompt dépasse 12 900 caractères sont
écartés, donc aucun projet de la taille d'Annecy n'est vu à l'entraînement. Le modèle installé
(fusion A+B du 11-10) n'a vu ni les familles nouvelles ni les gros projets.

**Approach:** régler le jeu par sa borne, pas par le test : `MAX_PROMPT_CHARS` et `MAX_LENGTH`
relevés après un essai mémoire court sur la carte de 8 Go ; vérifier que `gisement` et le
carroyage du PDF n'enseignent pas de plans contradictoires ; recette verte ; régénérer et mesurer ;
puis une nuit à deux graines, la fusion, les essais GPU et la recette d'Annecy à 20 phrases.

Décision tranchée en party le 2026-11-10 (ingénieur ML, architecte, géomaticien), à confirmer par
le propriétaire : option B, `MAX_PROMPT_CHARS` 12 900 → 19 000 et `MAX_LENGTH` 4 608 → 5 632,
après un essai `--longs-d-abord` court ; repli si la carte se remplit (pic > 7,2 Gio ou vitesse
effondrée) : bornes gardées et poids de `gros_projet` relevé pour revenir à 25 %. Option C
(remplissages allégés) écartée : elle déplace le trou sans le fermer.

## Boundaries & Constraints

**Always:** graines du générateur 1 et 2, `DEFAULT_COUNT` 8 600, poids inchangés (sauf repli) ;
borne du test de 23 % gardée ; aucun exemple au-delà de `MAX_LENGTH` ; même base, même méthode,
même commande d'entraînement ; essais sur le commit de la branche `story/5-49-reentrainement`.

**Never:** installer ou supprimer un modèle ; pousser, fusionner dans `main` ; changer de branche
dans le dépôt principal ; nouvelle dépendance ; baisser une borne de test.

</frozen-after-approval>

## Code Map

- `training/src/lib.rs` -- `MAX_PROMPT_CHARS` (l. 37), test `the_default_set_holds_a_quarter_of_big_projects_and_close_layers`.
- `training/entrainement/pipeline.py` -- `MAX_LENGTH` (l. 81) et son commentaire.
- `training/src/families.rs` -- `gisement` (l. 6415), `carroyage_utm_projet` (l. 2570), test `a_grid_bearing_says_grid_and_names_its_zone_only_when_written`.
- `docs/entrainement-du-modele.md` -- section 5.47 (borne), nouvelle section 5.49 (mesures, nuit).
- `~/cassini-entrainement/nuit-5-49.sh`, `fusion-5-49.sh` -- hors dépôt, copies adaptées de la nuit du 11-10.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/lib.rs`, `training/entrainement/pipeline.py` -- bornes relevées, commentaires
- [x] `training/src/families.rs` -- garde : une demande de gisement ne parle jamais de carte ni de PDF
- [x] `docs/entrainement-du-modele.md` -- décision, mesures, essai mémoire, résultats de la nuit

**Acceptance Criteria:**
- Given le jeu par défaut, then 23 à 27 % de gros projets, aucun exemple au-delà de `MAX_LENGTH` jetons de Qwen3.5.
- Given la recette locale (fmt, clippy, tests Rust, tests Python), then verte.
- Given la nuit, then trois candidats mesurés sur toutes les barres et Annecy 20 phrases, aucun installé.

## Implementation Notes

Route oneshot : quelques lignes de code ; l'essentiel du ticket est la nuit d'entraînement et ses
mesures.

- Mesure sans borne (graine 1) : 254 prompts au-delà de 12 900 caractères, aucun au-delà de 19 000
  (18 506), séquence la plus longue 5 176 jetons. Avec les nouvelles bornes : 2 165 gros projets
  (25,2 %), 132 écartés (couche utile réduite seulement), 10,04 M de jetons.
- Essai mémoire `--longs-d-abord --pas-max 30` : pic 5,64 Gio, 65 / 54 / 44 s par pas ; arrêté
  avant la chaîne GGUF. Repli inutile.
- « Quadrillage UTM » : aucun exemple ne mêle `geo.*` et `layout.*` ; garde ajoutée au test du
  gisement (ni carte, ni export, ni carroyage, ni grille, ni « tous les », aucun `layout.*`).
  Première version de la garde trop stricte (`geo.*` seul) : la queue MGRS d'un gisement ajoute
  `coord.convert`, d'où « aucun `layout.*` ».
- `core/src/agent_inventory/sorted_view.rs` : commentaire seul (borne citée), budget inchangé.
- Recette locale verte : fmt, clippy (4 crates), tests Rust (4 crates), tests Python du pipeline
  (43) et de l'extension (717), garde de l'extension.
- Nuit : `~/cassini-entrainement/nuit-5-49.sh` et `fusion-5-49.sh`, essais Windows depuis
  `CASSINI-nuit` remis sur le commit, noyau construit dans `C:\ct-5-48` (`--features gpu`, non
  installé). Résultats : `docs/entrainement-du-modele.md`.
- Nuit (41ede5d) : a 289/300, b 285, fusion 290 ; Annecy 18/20 pour les trois, rejeu identique ;
  `plan_reel` 24 / 22 / 22 (barre 23) : seule la graine a tient toutes les barres chiffrées, mais
  `plan_reel`, `question_reel` et `ecart_reel` baissent face au modèle installé (règle du ticket :
  « pas en baisse »). Aucun modèle installé.

## Review Triage Log

Passe 1 (quick). Verdicts : high 0, medium 1, low 4, false 3, maybe-false 0. Corrigés 5, rejetés 3.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Critère de la nuit non tenu | false | reject | la nuit suit la relecture ; ses résultats sont ajoutés ensuite |
| `risk: high` relu en `quick` (CLAUDE.md §8) | false | reject | relecture `quick` épinglée par la demande du ticket |
| Bornes changées sans accord du propriétaire | false | reject | délégation : question tranchée en party, notée « à confirmer » |
| Essai de 30 pas ne voit pas la montée tardive du pic (5.47 : après le pas 20) | medium | patch | doc nuancée ; le pic de la nuit est relevé dans ses journaux, repli si > 7,2 Gio |
| Borne en jetons mesurée, pas construite ; graine 2 non couverte | low | patch | doc : la garantie tient au compte de la chaîne, 0 écarté sur les deux graines à l'essai |
| La doc prête au test une garde plus large qu'il ne l'est | low | patch | doc : le test garde le côté du gisement, le reste est mesuré |
| Doc périmée l. 802 et l. 1060 | low | patch | corrigées |
| Phrase sans verbe | low | patch | corrigée |

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
