---
title: 'Phrases de reference et validateur — partie 2 de l''entree 8.2'
type: 'chore'
ticket: '2'
created: '2026-10-15'
status: 'built'
baseline_revision: '2ebbb0f'
route: 'oneshot'
route_source: 'pinned'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'entrée 8.3 ne peut mesurer un taux de plans corrects sans phrases de référence portant leur plan attendu, ni sans un validateur qui dise si un plan rendu par un modèle est conforme au catalogue et à l'inventaire. La première moitié de 8.2 a livré le catalogue et l'inventaire.

**Approach:** ajouter dans `tests/epreuve-modele-local/` 25 phrases françaises avec leur plan attendu (au moins 5 cas de terrain civils, dont un refus), une dizaine de phrases anglaises qui renvoient aux plans français, et un validateur Python à bibliothèque standard seule, limité au sous-ensemble de JSON Schema du README, que 8.3 réutilise tel quel.

## Boundaries & Constraints

**Always:**
- Code jetable hors du produit ; bibliothèque standard seule ; commentaires et README en français.
- Un plan est conforme quand chaque commande est au catalogue, chaque paramètre valide son schéma (sous-ensemble du README), chaque couche citée existe dans l'inventaire ou est `step:k` avec *k* strictement inférieur au rang de l'étape et une étape qui produit une couche, et chaque chemin d'entrée figure parmi les fichiers non ouverts de l'inventaire (AD-23).
- Les paramètres `dem` et `raster` désignent un raster ; une coordonnée de terrain porte l'EPSG du modèle d'élévation visé, sinon le plan est rejeté (AD-19).
- Égalité de plans : on remplace chaque `step:k` par l'étape qu'il désigne, on compare les arbres obtenus, sans tenir compte de l'ordre des étapes ni des listes ; `title` et `sources_note` sont ignorés, une valeur égale à son défaut équivaut à l'absence. Une étape en trop est une erreur.
- Une carte PDF ne montre que la couche résultat, sauf si la phrase en nomme d'autres.
- Un refus se note `{"refusal": "<raison>"}` ; une phrase de refus a `expected: null` et `expect_refusal: true`.

**Never:**
- Toute fonction à usage militaire dans ce dépôt public : les cas de terrain n'emploient que les trois commandes du MVP.
- Modifier `catalog.json`, `inventory.json` ou le bloc figé du plan de la première moitié.
- Écrire le harnais de 8.3 ou appeler un modèle depuis ces fichiers.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan attendu | chacune des 25 phrases | accepté par le validateur | — |
| Paramètre mal typé | `distance.value` donné comme texte | rejeté | message nommant le chemin du paramètre |
| Commande ou couche absente | commande hors catalogue, identifiant hors inventaire, fichier absent | rejeté | message nommant l'intrus |
| Mauvaise référence | `step:k` vers l'avenir ou vers une étape sans couche | rejeté | message nommant l'étape |
| EPSG de terrain | observateur en 4326, MNT en 2154 | rejeté | message citant les deux EPSG |
| Même plan, autre ordre | étapes permutées | égal | — |
| Étape en trop | `layer.add` ajouté | différent | — |

</frozen-after-approval>

## Code Map

- `tests/epreuve-modele-local/schema_validator.py` -- à créer : valide une valeur contre un schéma du sous-ensemble, résout `#/$defs/...`.
- `tests/epreuve-modele-local/plan_validator.py` -- à créer : `check_plan`, `check_output`, `canonical`, `plans_equal`.
- `tests/epreuve-modele-local/sentences.json` -- à créer : 25 phrases `{id, text, expected, expect_refusal}`.
- `tests/epreuve-modele-local/sentences_en.json` -- à créer : une dizaine de phrases `{id, text, same_plan_as}`.
- `tests/epreuve-modele-local/check_sentences.py` -- à créer : contrôle les phrases et rejoue les cas négatifs.
- `tests/epreuve-modele-local/README.md` -- compléter : formats, règle d'égalité, état.
- `catalog.json`, `inventory.json`, `check_catalog.py` -- à réutiliser sans les modifier ; les identifiants de couche viennent de l'inventaire.

## Tasks & Acceptance

**Execution:**
- [x] `schema_validator.py` -- valider type, enum, bornes, motif, objets fermés, tableaux bornés, `$ref` -- sous-ensemble du README seul.
- [x] `plan_validator.py` -- règles de conformité et égalité de plans de la section Always.
- [x] `sentences.json` -- 25 phrases : ouverture, description, inventaire, retrait, reprojection, sélection, zone tampon, export, intersection, statistiques, isochrone, PDF, la chaîne de démo, six cas de terrain dont un refus WGS84.
- [x] `sentences_en.json` -- 10 phrases anglaises renvoyant à des plans français.
- [x] `check_sentences.py` -- accepte les 25 plans, rejette les cas négatifs de la matrice, vérifie l'égalité.
- [x] `README.md` -- formats et règles.

**Acceptance Criteria:**
- Given `python check_sentences.py`, when il s'exécute, then il sort 0 en annonçant 25 phrases françaises (au moins 5 de terrain), 10 anglaises et les cas négatifs refusés.
- Given deux lectures indépendantes des 25 phrases sans leur plan attendu, when on les compare avec la règle d'égalité, then chaque phrase donne le même plan.

## Implementation Notes

Route oneshot épinglée : l'estimation dépasse 100 lignes, mais aucun sous-agent de codage n'est autorisé ; la seconde lecture indépendante, elle, est faite par un sous-agent relecteur, comme l'exige le `verify` de l'entrée.

Fait : `schema_validator.py`, `plan_validator.py`, `sentences.json` (25 phrases), `sentences_en.json` (10), `check_sentences.py`, `reading_independent.json` (la seconde lecture), README. `check_sentences.py` : 25 phrases dont 5 cas de terrain avec plan et un refus, 19 plans faux refusés, égalité de plans vérifiée. Seconde lecture indépendante par un sous-agent qui ne voyait ni les plans attendus ni le dépôt : 24 plans sur 25 identiques ; S14 (« zones à urbaniser ») avait deux lectures valides (libellé, ou `typezone` commençant par `AU`), la phrase a été reformulée (« dont le libellé est « Zone à urbaniser » ») et relue seule : 25 sur 25. Ce relecteur n'a donc pas relu les 24 autres après reformulation de S14, qui ne les touche pas.

## Design Notes

Le cas « pentes de plus de 15 % » annoncé plus haut est retiré : aucune commande du catalogue ne filtre un raster. À sa place, pente en degrés puis statistique par commune. La phrase de démo de CLAUDE.md §1 emploie « constructible », qui n'a pas de valeur unique dans `typezone` : elle est écrite avec « zone U », et ce manque de vocabulaire est noté pour 8.6.

## Verification

**Commands:**
- `python -I tests/epreuve-modele-local/check_sentences.py` -- expected: sortie 0
- `python -I tests/epreuve-modele-local/check_catalog.py` -- expected: sortie 0, toujours cohérent

## Review Triage Log

Relecture quick, 10 constats.
- medium, patch : le bytecode `__pycache__` était indexé ; retiré de l'index et ignoré par un `.gitignore` local au dossier.
- medium, patch : `$` accepte un saut de ligne final en Python, donc `step:1` suivi d'un saut de ligne passait ; les motifs sont désormais appariés en entier (`fullmatch`) et `check_catalog.py` exige qu'ils soient ancrés. Un cas négatif le couvre.
- medium, patch : une `command` qui n'est pas un texte faisait planter `check_plan` au lieu d'être rejetée ; contrôle ajouté, cas négatif ajouté.
- medium, patch : la règle AD-19 n'était pas appliquée quand `dem` est `step:k` ; l'EPSG est maintenant suivi à travers les étapes de terrain et de reprojection, cas négatif ajouté.
- low, patch : le type de la valeur d'un filtre n'était pas contrôlé ; il l'est pour les couches de l'inventaire. Le champ d'une couche produite par une étape reste non contrôlé, et les `values` d'exemple n'enferment pas la valeur : dit au README.
- low, patch : critère 2 sans trace ; `reading_independent.json` est versé et rejouable, et le README et ce plan consignent le résultat.
- low, patch : README périmé (« la seconde session ») corrigé.
- low, reporté : unités non normalisées dans l'égalité de plans (500 m contre 0,5 km). Le bloc figé ne le prévoit pas ; limite écrite au README, entrée dans `deferred-work.md`.
- low, noté sans correction : S19 croise des bâtiments en 4326 avec une zone tampon en 2154 sans reprojeter ; hypothèse écrite au README, à vérifier à l'epic du registre.
- false : la phrase de décompte de `check_sentences.py` (5 cas de terrain plus un refus) respecte le critère « au moins 5 ».
