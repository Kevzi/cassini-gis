---
type: change
title: "Une distance en mètres sur une couche en degrés : la commande calcule dans un système métrique"
date: 2026-11-12
status: approuvée, 2026-11-12
scope: moderate
mode: batch
---

# Changement de cap — distance en mètres sur une couche en degrés

Statut : **approuvée par le propriétaire le 2026-11-12**, avec les deux ajouts (paramètre `work_epsg` rempli par le noyau, `crs_unit` et `project_crs_unit` dans l'inventaire). Rédigée en mode « batch » (workflow `bmad-correct-course`) à partir de la décision prise
en party par le propriétaire le 2026-11-12.

## 0. Liste de contrôle (checklist.md)

| Point | État | Note |
|---|---|---|
| 1.1 Récit déclencheur | [x] | Répétition de la démo d'Annecy, 2026-11-12 ; constat déjà reporté à la sonde 11.18 (`deferred-work.md`). |
| 1.2 Problème | [x] | Approche qui a échoué : compter sur le modèle pour composer la reprojection. |
| 1.3 Preuves | [x] | `analysis.crs_not_metric` sur la couche « gares » en EPSG:4326, deux modèles (séance 3 et B). |
| 2.1 Epic en cours | [x] | Epic 9 (`epic-facade-qgis`) : une entrée de plus, la 16. |
| 2.2 Changement d'epic | [x] | Aucun epic créé ni retiré. |
| 2.3 Epics à venir | [!] | Epic 10 : six entrées planifiées reprennent la règle (§2.2). |
| 2.4 Epic rendu obsolète | [N/A] | Aucun. |
| 2.5 Ordre | [!] | L'entrée 9.16 passe avant la démo filmée (epic 7, `epic-export-et-demo`). |
| 3.1 Spec | [!] | CAP-3, critère de succès précisé. |
| 3.2 Architecture | [!] | Précisions datées sous AD-19 et AD-24 ; AD-5, AD-6, AD-11, AD-13, AD-23 tenues sans changement. |
| 3.3 UX | [!] | Mineur : le compte rendu de l'étape nomme le système de calcul. |
| 3.4 Autres | [!] | Catalogue (deux fiches), tests de l'extension et du noyau, recette QGIS. Pas de ré-entraînement. |
| 4.1 Ajustement direct | Viable | Effort moyen, risque élevé (rejeu, canal, analyse). |
| 4.2 Retour en arrière | Non viable | Rien à défaire. |
| 4.3 Revue du MVP | Non viable | Le MVP tient, c'est même la condition de la démo. |
| 4.4 Voie retenue | [x] | Ajustement direct (§3). |
| 5.x Composantes | [x] | Sections 1 à 5 ci-dessous. |
| 6.3 Approbation | [x] | Approuvée par le propriétaire le 2026-11-12, avec ses deux ajouts. |

## 1. Résumé du problème

**Déclencheur.** Répétition de la démo d'Annecy, le 2026-11-12, dans le vrai QGIS, avec le
vrai noyau et le vrai modèle. La phrase de démo :

> « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Annecy et
> sors-moi la carte en PDF. »

donne un plan **juste dans sa forme** :

1. `analysis.select_by_attribute` (zonage, `TYPEZONE` = `U`) ;
2. `analysis.buffer` (500 m autour des gares) ;
3. et 4. deux `analysis.intersect` ;
5. `layout.export_pdf`.

L'étape 2 échoue : `analysis.crs_not_metric`. La couche « gares » du projet d'Annecy est en
EPSG:4326 (elle vient de `gares-74-4326.csv`, en latitude et longitude). Le message actuel
dit à l'utilisateur de « reprojeter d'abord dans un système en mètres ».

**Preuves.**
- Le modèle installé (séance 3) **et** l'ancien (B) ratent de la même façon : aucun des deux
  n'a jamais ajouté de `data.reproject` avant le tampon dans ce cas.
- Le constat est connu depuis la sonde 11.18 du 2026-11-07 (`deferred-work.md` : « Une couche
  de points en EPSG:4326 […] ne passe pas un tampon […] la phrase de démo transposée à Annecy
  échoue pour cette raison »). On attendait `data.reproject` (10.7, faite depuis) ; la
  commande existe, mais le modèle ne la compose pas.
- Le jeu de Villeneuve donne ses gares en Lambert-93 : il ne montrait pas le cas.

**Nature.** Échec d'une approche. Faire composer au modèle une conversion mécanique a déjà
échoué une fois (points vers le système du MNT, réglé par 11.22). Le propriétaire ne veut pas
de pansement par phrase (« modèle entraîné, pas de pansements ») et ne veut pas non plus que
le noyau répare le plan.

**Décision du propriétaire (party du 2026-11-12), écrite ici comme décision :**

1. Une commande d'analyse qui reçoit une distance en mètres détecte que sa couche d'entrée
   est en système géographique (degrés) et fait elle-même la conversion nécessaire.
2. Le calcul se fait dans un système métrique. L'ordre de choix est tranché au §3.2.
3. Le résultat est dans un système cohérent avec la suite du plan.
4. **Aucune étape n'est ajoutée au plan.** Le noyau ne répare pas le plan du modèle (AD-23,
   récit 5.3 « validation sans réparation »).
5. Première commande : `analysis.buffer`, celle de la démo.
6. Les autres commandes à distance en mètres suivent après la démo, par tickets séparés
   (§2.2).
7. Le catalogue est figé depuis le 2026-11-05. Ce changement de cap est ce qui autorise à
   toucher la fiche de `analysis.buffer`.

## 2. Analyse d'impact

### 2.1 Epics et tickets

| Epic | Effet |
|---|---|
| **Epic 9** — `epic-facade-qgis` (Façade QGIS) | **Reçoit l'entrée 16** (`next_id` = 16). Il porte CAP-3 et a construit `analysis.buffer` (entrée 8) et `analysis.isochrone` (entrée 11). Une seule entrée y reste ouverte (15), l'epic n'est pas clos. |
| Epic 7 — `epic-export-et-demo` | Inchangé. Ses Boundaries disent : « Aucune nouvelle commande, aucune nouvelle analyse […] Ce qui manquerait ici est une entrée de l'epic 9, 5 ou 4. » D1 (la phrase de démo de bout en bout) attend 9.16. |
| Epic 10 — `epic-catalogue-etendu-qgis` | Reçoit l'entrée 32 (isochrone, après la démo). Six entrées planifiées reçoivent une note : 10.6, 10.10, 10.11, 10.13, 10.14 et 10.22 (§2.2). |
| Epic 5 — `epic-agent-et-mcp` | Aucun ticket. Pas de famille nouvelle au générateur, pas de ré-entraînement. Le modèle compose déjà le bon plan. |
| `epic-analyses-et-historique` | Retiré le 2026-10-14 : exclu. |

**Pourquoi l'epic 9 et pas l'epic 10.** Le changement corrige le comportement d'une commande
**faite** de CAP-3. L'epic 9 l'a livrée. L'epic 10 porte CAP-14 (catalogue étendu) et attend
la publication. Mettre 9.16 dans l'epic 10 obligerait à une dérogation à son ordre, comme
pour 10.7 et 10.27. Ici ce détour n'apporte rien.

### 2.2 Commandes à distance en mètres, pour après la démo

Recensement fait dans le catalogue (fiches `json fiche`) et dans la table fermée de
l'extension (`ALGORITHMS`, clé `metric`).

| Commande | Statut | Ce qui est en mètres | Suite proposée |
|---|---|---|---|
| `analysis.buffer` | faite | `distance` | **Entrée 9.16, avant la démo.** |
| `analysis.isochrone` | faite | Longueurs du réseau (coût) et accrochage de l'origine (`_ORIGIN_SNAP_METERS`, 100 m). La ligne a déjà `metric: True`. | Entrée 10.32, après la démo, `after = [6]`. L'entrée 10.6 la reprend sur `NetworkOptions`, on évite donc de la toucher deux fois. |
| `analysis.nearest` | planifiée (10.10) | `max_distance`, distance rendue | Note sur 10.10 : elle naît avec la règle. |
| `analysis.distance_matrix` | planifiée (10.10) | distances rendues en `m` | Même note sur 10.10. |
| `analysis.sum_line_lengths` | planifiée (10.11) | longueurs ; la fiche dit aujourd'hui « Couche en degrés : `analysis.crs_not_metric` » | Note sur 10.11 et fiche réécrite (§4.3). |
| `analysis.area` | planifiée (10.22) | surface plane ; même phrase dans la fiche | Note sur 10.22 et fiche réécrite (§4.3). |
| `network.shortest_path` | planifiée (10.13) | `network.tolerance` (`NetworkOptions`), longueurs | Note sur 10.13. |
| `network.prepare` | planifiée (10.14) | `tolerance` | Note sur 10.14. |

**« Sélection par distance ».** Aucune commande du catalogue ne la porte. La démo la compose
par tampon puis intersection. Une commande de ce genre ajoutée plus tard prendra la même
règle dans sa fiche.

**Hors de cette règle** :
- ~~`terrain.*` et `raster.*` : un MNT doit être projeté en mètres, et c'est le noyau qui le lit
  (AD-19, AD-20). Ces commandes ne prennent pas de couche vecteur. `raster.contour` a une
  `interval` verticale.~~ **Corrigé par le complément approuvé du 2026-11-12 (§6)** : les
  commandes qui lisent un MNT prennent la même règle, après la vidéo (entrée 10.33).
  `raster.contour` garde son `interval` verticale, qui n'est pas une distance sur la couche.
- `geo.*` : géodésie sur des points, déjà juste en degrés (AD-31).
- `layout.export_pdf` : `scale` n'est pas une distance sur une couche.
- `analysis.intersect`, `analysis.select_by_attribute`, `analysis.zonal_statistics`,
  `data.*` : aucune distance.

### 2.3 Conflits avec les documents

**Spec (CAP-3).** Le critère de succès ne dit rien du système de la couche d'entrée. Or la
démo et l'esprit « l'utilisateur apporte ses données, sans préparation » (CAP-9) supposent
que des gares en latitude et longitude passent. CAP-9 reprojette à l'ouverture par
`layer.add`, mais pas une couche que l'utilisateur a déjà ouverte dans son projet : c'est le
cas d'Annecy. **Précision proposée** au §4.1.

**Contrat d'architecture.** Le point clé à trancher : **où vit la conversion ?**

- **AD-5** : « Aucune logique métier, aucun calcul géographique et aucune décision de nommage
  ne vivent dans l'extension. »
- **AD-24, Chaîne** : « Une étape peut porter une garde […] ; aucune étape ne choisit entre
  deux suites. Toute valeur vient des paramètres du noyau. »

Si le geste choisissait lui-même le système (celui du projet, sinon UTM, sinon refus), il
enfreindrait les deux. Il prendrait une décision et choisirait entre des suites. Le rejeu
dépendrait aussi de l'état du projet au moment du rejeu : un projet repassé dans un autre
système donnerait un autre calcul, sans que le journal le dise. **La règle doit donc être
décidée par le noyau, et le geste n'applique qu'une valeur reçue** (§3.2).

Les autres décisions d'architecture concernées :

- **AD-19** (« QGIS possède les systèmes de coordonnées ; le noyau ne reprojette pas ») est
  tenue. Le noyau **choisit un code EPSG** et QGIS **reprojette**.
  - Son amendement du 2026-11-05 dit qu'« une conversion n'est jamais insérée en silence
    avant un calcul, elle est une étape `coord.convert` visible du plan ». Ce texte vise les
    coordonnées ponctuelles d'une commande de terrain.
  - Ici la conversion n'est pas silencieuse sans être une étape. Elle est déclarée par la
    description de la commande, que lit le modèle. Elle est portée par un paramètre
    journalisé. Elle est visible dans le résultat (`crs`) et dans le compte rendu de l'étape.
  - Une **précision datée** l'écrit (§4.2).
- **AD-24** : la ligne `analysis.buffer` de la table fermée passe à une **chaîne fixe de deux
  algorithmes** (`native:reprojectlayer` vers un temporaire, puis `native:buffer`). Sa
  version de chaîne augmente. Une précision datée l'écrit (§4.2).
- **AD-23 et récit 5.3** : tenus. Le plan du modèle n'est ni complété ni réparé. Il est
  montré tel que le modèle l'a composé.
- **AD-11 (journal rejouable)** : tenu par construction.
  - Le système de calcul est un paramètre canonisé de la ligne. Le rejeu le relit au lieu de
    le recalculer.
  - Le changement de version de chaîne est signalé avant le rejeu, sans refus (AD-11,
    « Resserré »).
  - Le temporaire reprojeté n'est ni nommé ni journalisé (AD-24, écart à AD-10 déjà admis).
- **AD-13** : un seul registre pour quatre points d'entrée. Le panneau, la CLI `run`, le MCP
  et l'agent ont le même comportement. C'est un avantage net sur une complétion du plan à la
  manière de 11.22, qui laisse l'appel direct refusé.
- **AD-6** : le schéma reste engendré depuis le type Rust. Le paramètre ajouté est rempli par
  le noyau, hors de la grammaire du modèle, comme `out` (`ai/src/plan.rs` retire déjà `out`
  du schéma montré au modèle).
- **AD-8, AD-22** : le paramètre est un entier EPSG, neutre et documenté. Aucun vocabulaire
  de QGIS ne passe dans le noyau.
- **AD-21** : le refus garde son code `analysis.crs_not_metric`. Son message dit pourquoi
  aucun système métrique n'a été trouvé.
- **AD-12, AD-25** : `native:reprojectlayer` existe dans tout QGIS et s'annule. La recette
  qui confronte annulation et chaîne couvre la nouvelle ligne.

**UX (`ux-panneau-cassini`).** Mineur. Le compte rendu en langage courant de l'étape de
tampon dit dans quel système le calcul a été fait quand la couche était en degrés, par
exemple « calculé en Lambert-93, la couche des gares étant en degrés ». Il est composé là où
il l'est aujourd'hui (lot 1 de la reprise du panneau) et lit le résultat `crs` qui existe
déjà. Aucun écran nouveau.

### 2.4 Effets techniques

- **Noyau.**
  - `analysis_buffer.rs` : un paramètre `work_epsg` rempli par le noyau, et des descriptions
    FR et EN réécrites.
  - Le choix du système est une fonction pure, testée sans QGIS.
  - Pour une couche du projet, le noyau lit le système et son unité dans l'inventaire.
  - Pour une couche `step:k`, il les lit dans le `crs` rendu par l'étape *k*.
- **Extension.**
  - Ligne `analysis.buffer` : chaîne `native:reprojectlayer` puis `native:buffer`, version de
    chaîne augmentée.
  - La garde `metric` porte désormais sur le système de calcul reçu.
  - L'inventaire rend l'unité du système de chaque couche et du projet. C'est un fait lu dans
    QGIS (`crs().mapUnits()`), pas une décision.
- **Catalogue.** Fiches `analysis.buffer` et `project.inventory` (§4.3). La recette
  `core/tests/catalogue.rs` les confronte.
- **Modèle.** Aucun ré-entraînement. Seule la description de `analysis.buffer` change dans le
  prompt (la phrase « La couche d'entrée doit être dans un système en mètres » disparaît).
  Les barres en cours se rejouent une fois avant installation, par prudence.
- **Données de l'utilisateur.** Intactes (principe 6). La reprojection écrit un temporaire de
  QGIS, jamais dans l'entrée (AD-24, liste blanche).

## 3. Voie recommandée

### 3.1 Choix

**Ajustement direct.** On ajoute une entrée à l'epic 9, avant la démo filmée, et des notes
aux entrées planifiées de l'epic 10. Pas de retour en arrière ni de révision du MVP.

Alternatives écartées :

| Alternative | Pourquoi non |
|---|---|
| Le modèle apprend à composer `data.reproject` (exemples d'entraînement) | Deux modèles ratent déjà. Le précédent des points (11.12, 11.16, 11.18, 11.18b) a montré que l'apprentissage d'une règle mécanique ne tient pas. Le coût est un ré-entraînement avant la démo. |
| Le noyau ajoute une étape `data.reproject` au plan (comme 11.22 pour les points) | Écarté par le propriétaire (pas d'étape ajoutée). Cela ne couvre pas l'appel direct (CLI, MCP, formulaire), ajoute une couche intermédiaire au projet et allonge le plan montré. |
| Le geste de l'extension choisit le système lui-même | Enfreint AD-5 et AD-24 (« toute valeur vient des paramètres du noyau »). Le rejeu dépendrait du projet au moment du rejeu. |
| Garder le refus, avec un meilleur message | La démo échoue. |

### 3.2 La règle, tranchée

**Qui décide :** le noyau, au moment où il dicte l'ordre (`plan.execute`, `command.run`,
MCP, CLI `run`). Il remplit le paramètre `work_epsg` (entier), qui n'est montré ni au modèle,
ni au client MCP, ni au formulaire. Il le remplit comme `out`.

**Ce qu'il choisit :**

1. **Couche d'entrée en mètres → `work_epsg` = son propre système.** Aucune conversion
   effective : la reprojection vers le même système est l'identité. Cela respecte « couche
   métrique inchangée ».
2. **Couche en degrés, système du projet en mètres → le système du projet.** C'est le cas
   d'Annecy (Lambert-93). Le tampon sort dans le système de la carte, des parcelles et des
   intersections qui suivent. C'est le résultat le plus cohérent avec la suite du plan.
3. **Couche et projet en degrés → le fuseau UTM sur WGS84 de l'étendue.**
   - Le noyau le calcule déjà : c'est `project.utm_epsg`, tiré de `extent.lonlat` par
     `utm_epsg()` dans `core/src/agent_inventory.rs` (32632 sur Annecy).
   - Il prend l'étendue du projet, qui contient la couche, et non une étendue propre à la
     couche, que l'inventaire ne porte pas. Pour une couche à l'échelle d'une commune ou d'un
     département, le résultat est le même.
   - Ce fuseau est refusé, comme aujourd'hui, si l'étendue est hors des latitudes UTM ou
     trop large (`MAX_UTM_SPAN_DEGREES`).
4. **Sinon → refus clair, avant tout ordre :** `analysis.crs_not_metric`, avec un message
   qui nomme la couche et son système, et qui dit qu'aucun système en mètres n'a pu être
   choisi (projet en degrés, étendue trop large ou inconnue). Il propose de passer le projet
   en Lambert-93 ou d'utiliser « reprojette la couche … ».
5. **Unité inconnue du noyau** (couche `step:k` dont le `crs` n'est pas un système dont la
   session connaît l'unité) → `work_epsg` = le système de la couche. La garde du geste
   refuse alors par `analysis.crs_not_metric` si ce système n'est pas en mètres. Le geste ne
   choisit jamais.

**Système du résultat.** `work_epsg`. C'est déjà le champ `crs` du résultat, qui ne change
pas de forme. La suite du plan reçoit une couche en mètres. Les intersections suivantes
reprojettent leur seconde couche à la volée dans QGIS, comme aujourd'hui quand deux couches
diffèrent. Le test de 9.16 le vérifie.

**Rejeu.**
- Le rejeu relit `work_epsg` dans la ligne du journal et ne le recalcule pas. Il est
  identique même si le projet a changé de système depuis.
- Une ligne écrite avant ce changement n'a pas `work_epsg`. Le noyau la complète, au rejeu,
  par le système de la couche (règle 5). Cela reproduit à l'identique l'ancien comportement,
  succès sur couche métrique ou refus sur couche en degrés.

**Point à confirmer par le propriétaire.** La décision dit « identifiants et paramètres
inchangés ». Les paramètres que voit le modèle sont inchangés. En revanche, la fiche du
catalogue gagne un paramètre facultatif « fourni par le noyau » (`work_epsg`), comme `out`.
C'est le seul moyen de tenir AD-24 et un rejeu identique sans faire décider l'extension. Ce
changement de cap l'autorise. Le propriétaire doit l'approuver explicitement.

**Point laissé au plan de 9.16 (inconnu).** Lors d'un appel direct sans inventaire tenu par
la session, comment le noyau obtient-il l'unité de la couche ? Il peut relire l'inventaire,
une lecture qui n'est ni une étape du plan ni une ligne du journal. À vérifier dans
`serve.rs` avant de construire.

### 3.3 Effort, risque, calendrier

- **Effort :** moyen. Une fonction pure et ses tests dans le noyau, une ligne de table en
  deux algorithmes, l'unité dans l'inventaire, deux fiches du catalogue, la recette QGIS.
- **Risque :** **élevé** (rejeu, canal, analyses). Relecture `thorough` à plusieurs lentilles
  (CLAUDE.md §8, rétro de l'epic 9, A6).
- **Calendrier :** 9.16 passe avant la démo filmée. Elle débloque D1 de l'epic 7. Le reste
  vient après la démo, sans effet sur la date de publication.

## 4. Propositions de changement détaillées

### 4.1 Spec — `spec-cassini-mvp/spec-cassini-mvp.md`, CAP-3, `success`

ANCIEN :

> - **success:** zone tampon, intersection, sélection par attribut, statistiques par zone et
>   isochrones à pied et en voiture sont chacune une commande du registre, **exécutée par le
>   cadre de traitement de QGIS**, et donnent un résultat vérifiable par un test, hors ligne.

NOUVEAU :

> - **success:** zone tampon, intersection, sélection par attribut, statistiques par zone et
>   isochrones à pied et en voiture sont chacune une commande du registre, **exécutée par le
>   cadre de traitement de QGIS**, et donnent un résultat vérifiable par un test, hors ligne.
>   Une distance donnée en mètres s'applique aussi à une couche en degrés : la commande
>   calcule dans un système en mètres choisi par le noyau, sans étape ajoutée au plan
>   (changement de cap du 2026-11-12, tampon d'abord).

Justification : la phrase de démo doit passer sur des gares en latitude et longitude. Le MVP
n'est pas réduit, il est précisé.

### 4.2 Contrat d'architecture — `architecture-noyau-et-facade-qgis.md`

**Sous AD-19, après l'amendement du 2026-11-08 (ticket 11.22), ajouter :**

> - **Précision (2026-11-12, changement de cap `change-distance-en-metres-sur-couche-en-degres`) :**
>   une commande déléguée qui applique une distance en mètres à une couche vecteur calcule
>   dans un système en mètres **choisi par le noyau** et passé dans un paramètre rempli par
>   lui (`work_epsg`), absent de la grammaire du modèle, du MCP et du formulaire, comme
>   `out`.
>   - Couche en mètres : son propre système.
>   - Couche en degrés : le système du projet s'il est en mètres, sinon le fuseau UTM sur
>     WGS84 de l'étendue du projet (`project.utm_epsg`), sinon `analysis.crs_not_metric`
>     avant tout ordre.
>   - Unité inconnue : le système de la couche, que la garde de l'extension refuse s'il
>     n'est pas en mètres.
>
>   La reprojection est faite par QGIS dans la chaîne fixe de la ligne de table (AD-24). Le
>   noyau ne reprojette rien.
>
>   Ce n'est pas une conversion « en silence » au sens de l'amendement du 2026-11-05 :
>   - elle est déclarée par la description de la commande ;
>   - elle est journalisée dans les paramètres ;
>   - elle est rendue par le `crs` du résultat.
>
>   Aucune étape n'est ajoutée au plan (AD-23). L'unité du système est un fait lu par QGIS et
>   rendu par `project.inventory`. Le rejeu relit `work_epsg` ; une ligne qui ne le porte
>   pas est complétée par le système de la couche. Première commande : `analysis.buffer`
>   (entrée 9.16). Les autres suivent par leurs propres entrées.

**Sous AD-24, après l'amendement du 2026-11-05, ajouter :**

> - **Précision (2026-11-12, même changement de cap) :** une ligne dont la commande porte
>   `work_epsg` commence par une reprojection fixe (`native:reprojectlayer` vers
>   `work_epsg`, en temporaire), **toujours exécutée**, identité comprise. Ce n'est pas un
>   choix entre deux suites. La garde « système en mètres » porte sur `work_epsg`.
>   L'ajout incrémente la version de chaîne de la ligne (AD-11).

AD-5, AD-6, AD-11, AD-13 et AD-23 : aucun texte changé. Ce changement les applique.

Ces deux précisions n'ont pas encore de réponse de revue. Une relecture adverse rapide sur
AD-19 est conseillée avant approbation, sur le modèle de `reviews/review-adverse-*.md`.

### 4.3 Catalogue — `spec-cassini-mvp/catalogue-des-commandes.md`

**Fiche `analysis.buffer`, paragraphe d'en-tête.**

ANCIEN :

> Zone tampon autour des entités d'une couche, ajoutée comme nouvelle couche. La couche doit
> être dans un système en mètres (`analysis.crs_not_metric` sinon).

NOUVEAU :

> Zone tampon autour des entités d'une couche, ajoutée comme nouvelle couche. Une couche en
> degrés est d'abord reprojetée par QGIS, dans la même étape, vers le système en mètres
> choisi par le noyau (`work_epsg`) :
> - celui de la couche si elle est déjà en mètres ;
> - sinon celui du projet s'il est en mètres ;
> - sinon le fuseau UTM de l'étendue du projet.
>
> Si aucun ne convient : `analysis.crs_not_metric`, avant tout ordre. Le tampon sort dans ce
> système (`crs` du résultat). Changement de cap du 2026-11-12.

**Tableau des paramètres, ligne ajoutée :**

> | `work_epsg` | entier | non | code EPSG | Fourni par le noyau : système en mètres du calcul. |

**Bloc `json fiche`.**

ANCIEN :

```json fiche
{"id": "analysis.buffer", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "distance": {"type": "Distance", "required": true},
            "dissolve": {"type": "boolean", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

NOUVEAU (jusqu'à la livraison de 9.16, par `planned_params` et `planned_by`, règle de gel
§1 ; 9.16 le reporte dans `params`) :

```json fiche
{"id": "analysis.buffer", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "distance": {"type": "Distance", "required": true},
            "dissolve": {"type": "boolean", "required": false},
            "out": {"type": "string", "required": false}},
 "planned_params": {"work_epsg": {"type": "integer", "required": false}},
 "planned_by": "epic 9, entrée 16",
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

Le résultat ne change pas.

**Fiche `project.inventory`, tableau du résultat.**

ANCIEN (ligne `layers`) :

> | `layers` | liste de couches | `layer_id`, `name`, `crs`, `kind`, `source`, `geometry`, `feature_count`, `fields`. |

NOUVEAU :

> | `layers` | liste de couches | `layer_id`, `name`, `crs`, `crs_unit`, `kind`, `source`, `geometry`, `feature_count`, `fields`. `crs_unit` : `m`, `deg` ou `other`, unité du système lue par QGIS ; lue par le noyau, jamais montrée au modèle. |

Ligne ajoutée :

> | `project_crs_unit` | chaîne | Unité du système du projet : `m`, `deg` ou `other`. |

Et `"result"` du bloc : ajouter `"project_crs_unit"` par `planned_result` et `planned_by`
(epic 9, entrée 16).

**Fiches `analysis.area` et `analysis.sum_line_lengths` (planifiées).**

ANCIEN : « Couche en degrés : `analysis.crs_not_metric`. »

NOUVEAU : « Couche en degrés : calcul dans le système en mètres choisi par le noyau
(`work_epsg`), comme `analysis.buffer` (changement de cap du 2026-11-12). »

Le paramètre `work_epsg` s'ajoute à leur bloc quand leur entrée les livre, sous la même
forme.

### 4.4 Tickets — à créer avec `bmad-ticket`

**Epic 9, `epic-facade-qgis/tickets.toml`, nouvelle entrée 16 :**

```toml
[[entry]]
id = 16
type = "story"
title = "Tampon sur une couche en degres : calcul dans un systeme en metres choisi par le noyau"
description = "analysis.buffer recoit work_epsg, rempli par le noyau comme out, absent de la grammaire du modele, du MCP et du formulaire. Le noyau le choisit par une fonction pure : couche en metres -> son systeme ; couche en degres -> systeme du projet s'il est en metres, sinon fuseau UTM de l'etendue du projet (project.utm_epsg), sinon analysis.crs_not_metric avant tout ordre, message qui dit pourquoi ; unite inconnue -> systeme de la couche. Unite lue dans l'inventaire (crs_unit par couche, project_crs_unit), ou dans le crs rendu par une etape step:k. La ligne de table devient native:reprojectlayer (temporaire) puis native:buffer, version de chaine augmentee, garde metrique portee par work_epsg. Aucune etape ajoutee au plan (AD-23). Rejeu : work_epsg relu dans le journal ; une ligne ancienne sans work_epsg est completee par le systeme de la couche. Descriptions FR/EN de la commande reecrites. Fiches analysis.buffer et project.inventory : planned_* reportes. Compte rendu de l'etape : nomme le systeme de calcul quand la couche etait en degres."
verify = "Tests du noyau sans QGIS : choix de work_epsg sur les cinq cas (metrique, projet metrique, UTM, refus avec message, unite inconnue), et sur une couche step:k. Tests de l'extension : un tampon de 500 m sur une couche de points en EPSG:4326 donne le meme resultat, a quelques metres pres (ecart de surface et distance de Hausdorff mesures), qu'un data.reproject manuel vers le meme systeme suivi du tampon ; une couche deja metrique donne un resultat inchange (meme systeme, meme nombre d'entites, meme geometrie) ; work_epsg non metrique refuse par analysis.crs_not_metric. Rejeu identique dans la meme session et dans une session neuve, y compris apres un changement du systeme du projet ; une ligne ancienne sans work_epsg se rejoue comme avant. Message clair si aucun systeme en metres n'est trouvable (projet et couche en degres, etendue trop large). core/tests/catalogue.rs vert. Recette QGIS reelle : la demo d'Annecy, phrase de CLAUDE.md §1, rejouee jusqu'au PDF, rendu en PNG et regarde a l'oeil, puis rejouee depuis le journal apres fermeture et reouverture du projet."
covers = ["CAP-3"]
after = []
hitl = false
risk = "high"
references = [
  "change — _bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md, §3.2 et §4",
  "architecture — architecture-noyau-et-facade-qgis.md, AD-19 et AD-24 (precisions du 2026-11-12), AD-5, AD-11, AD-23",
]
unknown = "Appel direct sans inventaire tenu par la session : comment le noyau obtient l'unite de la couche (relecture de l'inventaire, hors journal) — a verifier dans serve.rs au plan."
notes = ["Decision: 2026-11-12 — proprietaire, en party : la commande convertit elle-meme, aucune etape ajoutee au plan ; tampon d'abord, le reste apres la demo."]
```

**Epic 10, `epic-catalogue-etendu-qgis/tickets.toml`, nouvelle entrée 32 (après la démo) :**

```toml
[[entry]]
id = 32
type = "story"
title = "Isochrone sur un reseau en degres : calcul dans un systeme en metres choisi par le noyau"
description = "Applique a analysis.isochrone la regle de 9.16 (work_epsg, chaine native:reprojectlayer en tete, version de chaine augmentee) : longueurs du reseau et accrochage de l'origine en metres sur un reseau en EPSG:4326 ; l'origine est transformee vers work_epsg. Fiche du catalogue mise a jour."
verify = "Une isochrone sur un reseau de test en EPSG:4326 rend la meme emprise, a quelques metres pres, que sur le meme reseau reprojete a la main ; reseau metrique inchange ; rejeu identique ; core/tests/catalogue.rs vert."
covers = ["CAP-3"]
after = [6, "9.16"]
hitl = false
risk = "high"
```

**Notes à ajouter aux entrées planifiées de l'epic 10** (sans changer leur ordre) : 10.6,
10.10, 10.11, 10.13, 10.14, 10.22.

> « Changement de cap du 2026-11-12 : une distance ou une mesure en mètres sur une couche en
> degrés se calcule dans le système `work_epsg` choisi par le noyau, comme `analysis.buffer`
> (9.16) ; la commande naît avec cette règle. »

**Epic 9, `epic-facade-qgis.md`, Notes :**

> - Decision: 2026-11-12 — une distance en mètres sur une couche en degrés est calculée dans
>   un système en mètres choisi par le noyau (changement de cap
>   `change-distance-en-metres-sur-couche-en-degres`) ; entrée 16, avant la démo.

**`deferred-work.md`**, entrée de la sonde 11.18 « Une couche de points en EPSG:4326 […] » :
ajouter `status: à résoudre par 9.16`, puis `resolution` à la livraison.

## 5. Passage à la réalisation

- **Portée : modérée (Moderate).**
  - Ce n'est pas une replanification : le MVP et l'ordre des epics tiennent.
  - Mais deux documents de contrat changent (AD-19, AD-24 ; deux fiches du catalogue), une
    entrée passe avant la démo et sept entrées de l'epic 10 sont touchées. Cela demande une
    réorganisation du carnet, coordonnée entre le propriétaire et l'agent de développement.
- **Destinataires :**
  - **Propriétaire** :
    - approuver la proposition et le point du §3.2 (paramètre `work_epsg` fourni par le
      noyau) ;
    - confirmer l'ordre de choix (projet, puis UTM du projet, puis refus).
  - **Agent de développement**, une fois approuvé :
    - porter les éditions du §4.1 à 4.3 ;
    - créer les entrées du §4.4 avec `bmad-ticket` ;
    - puis `bmad-build` sur 9.16, relecture `thorough`.
- **Changements de carnet, à appliquer avec `bmad-ticket` :**
  - ajoutée : 9.16 (avant la démo) ;
  - ajoutée : 10.32 (après la démo, après 10.6) ;
  - précisées par note : 10.6, 10.10, 10.11, 10.13, 10.14, 10.22 ;
  - ordre : 9.16 avant la démo filmée de l'epic 7, aucune autre entrée déplacée.
- **Critères de réussite :**
  1. La phrase de démo d'Annecy passe jusqu'au PDF, sans étape ajoutée au plan, avec le
     modèle installé, et le PDF est regardé à l'œil.
  2. Un tampon de 500 m sur une couche en EPSG:4326 égale, à quelques mètres près, le tampon
     fait après une reprojection manuelle.
  3. Une couche déjà en mètres donne un résultat inchangé.
  4. Le rejeu est identique, y compris après un changement du système du projet, et un
     journal ancien se rejoue comme avant.
  5. Le refus est clair quand aucun système en mètres n'est trouvable.
  6. `core/tests/catalogue.rs` et la recette restent verts.

## 6. Complément approuvé (2026-11-12) : MNT en degrés

Approuvé par le propriétaire le 2026-11-12, après l'approbation des sections 1 à 5. Il
corrige la ligne « Hors de cette règle : `terrain.*` et `raster.*` » du §2.2.

### 6.1 Problème

Les commandes qui lisent un MNT refusent aujourd'hui un MNT en degrés :

- `terrain.*` (pente, exposition, ombrage, champ de vision, champ de vision cumulé, ligne de
  vue, points hauts, surface de coût, chemin de moindre coût) ;
- `raster.reclassify`, `raster.weighted_overlay`, `raster.contour` ;
- toute commande qui ouvre un MNT par `core/src/dem.rs`.

Le refus est `dem.not_projected`, avec le conseil « reprojetez-le dans QGIS (Raster >
Projections > Reprojeter) ». Or les MNT publics les plus courants, SRTM et Copernicus
GLO-30, sont livrés en EPSG:4326. L'utilisateur qui apporte ses données (principe 6) tombe
donc sur un refus que la commande pourrait éviter, comme le tampon avant 9.16.

### 6.2 Règle

Même règle que le tampon (§3.2) :

1. **Qui décide :** le noyau. Il choisit le système en mètres de travail et le passe dans
   un paramètre qu'il remplit lui-même (`work_epsg`), absent de la grammaire du modèle, du
   MCP et du formulaire, comme `out`.
2. **Ordre de choix :**
   - le système du MNT s'il est en mètres (aucune reprojection effective) ;
   - sinon le système du projet s'il est en mètres ;
   - sinon le fuseau UTM sur WGS84 de l'étendue du projet (`project.utm_epsg`) ;
   - sinon un refus clair avant tout ordre, `dem.not_projected` (code inchangé, AD-21), dont
     le message dit pourquoi aucun système en mètres n'a pu être choisi.
3. **Qui reprojette :** l'extension, par un geste fixe délégué à QGIS, sans aucun choix
   (AD-5, AD-24). Elle écrit le MNT reprojeté dans `work_epsg` en fichier temporaire, à
   l'emplacement décidé par le noyau (AD-10), **avant** que le noyau le lise. L'original
   n'est jamais modifié (principe 6).
4. **Ce que lit le noyau :** toujours un MNT métrique. Il ne reprojette rien et ne lie pas
   GDAL (AD-1, AD-19, AD-20) : `dem.rs` garde son refus d'un MNT en degrés, qui ne lui
   parvient plus que si l'extension n'a pas reprojeté.
5. **Aucune étape ajoutée au plan** (AD-23) : la reprojection fait partie de l'étape qui lit
   le MNT, comme pour le tampon.
6. **Journal et rejeu :** `work_epsg` est journalisé dans les paramètres de l'étape ; le
   rejeu le relit et ne le recalcule pas, et le résultat est identique même si le projet a
   changé de système (AD-11). Une ligne ancienne sans `work_epsg` est complétée par le
   système du MNT, ce qui reproduit l'ancien comportement.
7. **Points :** les points donnés en WGS84 restent convertis comme depuis 11.22 (AD-19,
   amendement du 2026-11-08), vers le système que lit le noyau, c'est-à-dire `work_epsg`.
8. **Plusieurs rasters :** des rasters dérivés d'un même MNT reprojeté gardent une même
   grille ; sinon `raster.grid_mismatch` reste la règle (AD-20, amendement du 2026-11-05).

Points laissés au plan de l'entrée :
- l'algorithme de reprojection raster de QGIS et la méthode de rééchantillonnage, fixés
  dans la ligne de table, jamais par le modèle ;
- la taille de cellule du MNT reprojeté ;
- le message qui porte l'ordre de reprojection au noyau et la forme de `work_epsg` sur les
  fiches du catalogue concernées (une fiche par commande, sous `planned_params`, au moment
  où l'entrée les livre).

### 6.3 Décisions d'architecture concernées

- **AD-19** : tenue. Le noyau choisit un code EPSG, QGIS reprojette. Précision datée
  ci-dessous.
- **AD-20** : tenue. Le module unique `dem.rs` continue de ne lire qu'un MNT métrique.
  Précision datée ci-dessous.
- **AD-24** : la reprojection est un geste fixe, toujours exécuté (identité comprise),
  sans choix entre deux suites ; toute valeur vient des paramètres du noyau.
- **AD-5** : aucune décision dans l'extension.
- **AD-10** : le noyau décide de l'emplacement du MNT reprojeté.
- **AD-11** : `work_epsg` journalisé, rejeu identique.
- **AD-21** : le code `dem.not_projected` est gardé.
- **AD-23** : aucune étape ajoutée au plan.
- **AD-1** : aucune GDAL dans le noyau.

**Précision datée proposée, sous AD-19 (appliquée au contrat d'architecture) :**

> - **Précision (2026-11-12, complément du changement de cap
>   `change-distance-en-metres-sur-couche-en-degres`) :** la même règle vaut pour un MNT en
>   degrés lu par une commande du noyau (`terrain.*`, `raster.reclassify`,
>   `raster.weighted_overlay`) ou déléguée (`raster.contour`). Le noyau choisit `work_epsg`
>   (système du MNT s'il est en mètres, sinon celui du projet s'il est en mètres, sinon le
>   fuseau UTM de l'étendue du projet, sinon `dem.not_projected` avant tout ordre) ;
>   l'extension reprojette le MNT dans ce système par un geste fixe de QGIS, en fichier
>   temporaire à l'emplacement décidé par le noyau (AD-10), avant que le noyau le lise. Le
>   noyau ne reprojette rien. Aucune étape n'est ajoutée au plan (AD-23) ; `work_epsg` est
>   journalisé et relu au rejeu (AD-11). Les points en WGS84 restent convertis par la
>   complétion de 11.22, vers `work_epsg`. Entrée 10.33, après la vidéo.

**Précision datée proposée, sous AD-20 (appliquée au contrat d'architecture) :**

> - **Précision (2026-11-12, même complément) :** le module unique ne lit toujours qu'un MNT
>   projeté en mètres et garde son refus `dem.not_projected`. Un MNT en degrés lui parvient
>   déjà reprojeté par QGIS dans `work_epsg` (AD-19, précision du 2026-11-12) ; le noyau ne
>   lie ni GDAL ni PROJ.

### 6.4 Ticket

**Epic 10, `epic-catalogue-etendu-qgis/tickets.toml`, nouvelle entrée 33**, après la vidéo
(décision du propriétaire), avec `after` vers la publication et vers 9.16.

Pourquoi l'epic 10 et pas un epic de terrain : les epics 4 (`epic-raster-et-terrain`) et 11
(`epic-geometrie-et-terrain-etendu`) sont des prérequis de l'epic de démo. Une entrée qui
attend la vidéo y créerait un cycle entre epics (l'epic de démo attend l'epic de terrain,
qui attendrait la publication). L'epic 10 attend déjà la démo et la publication. Il porte
`raster.contour` (10.26) et 10.32, qui applique la même règle à l'isochrone.

