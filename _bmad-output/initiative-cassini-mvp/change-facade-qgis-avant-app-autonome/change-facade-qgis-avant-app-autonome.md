---
type: change-proposal
title: "La façade QGIS avant l'application autonome"
initiative: initiative-cassini-mvp
created: '2026-10-14'
status: 'approved'
scope: 'major'
mode: 'batch'
decided_by: "propriétaire du projet, approuvé le 2026-10-14"
---

# Proposition de changement — la façade QGIS avant l'application autonome

> Produit par `bmad-correct-course`. Tant que `status` n'est pas `approved`, aucune
> modification n'a été appliquée à la spec, aux epics ni à `CLAUDE.md`.

## 1. L'enjeu

**Type de changement :** virage stratégique. Ni défaillance technique, ni
exigence nouvelle d'un tiers.

**Déclencheur.** Aucune entrée n'a échoué. Le sujet est remonté à la fin de
l'entrée 1.5, en écrivant le `README.md` public : mettre noir sur blanc « ce qui
marche aujourd'hui » face à « ce qui est prévu » a rendu visible la proportion du
travail restant qui reconstruit QGIS. Le propriétaire a formulé le doute le
2026-10-14 et posé la question directement : un plugin QGIS ne serait-il pas
mieux ?

**Le problème, précisément.** Le plan actuel consacre les epics 2, 3, 4 et 6 —
soit l'essentiel de l'effort du MVP — à refaire ce que QGIS fait déjà : organiser
un projet, importer tous les formats, croiser des couches, sortir une carte en
PDF. QGIS est libre, hors ligne, organisé en projets, mûr depuis vingt ans, et
**déjà installé chez les utilisateurs visés**. Pendant ce temps, la seule capacité
que personne ne sert — un agent en langage naturel qui pilote un SIG sans réseau
(CAP-4, portée par le registre CAP-6) — est planifiée en epic 5 sur 7, donc en
dernier. L'effort va donc massivement là où Cassini est le plus faible, et la
promesse unique du produit reste la plus longtemps non vérifiée.

**Éléments factuels retenus.**

1. **QGIS est absent du positionnement.** La section *Why* de la spec nomme
   ArcGIS Pro comme l'existant à remplacer et ne mentionne pas QGIS une seule
   fois. Le raisonnement « ils paient une licence propriétaire » est faux pour la
   part des bureaux d'études qui travaille déjà sous QGIS gratuitement.
2. **L'interdiction de GEOS coûte des années.** Les *Constraints* de la spec
   excluent GEOS (LGPL-2.1), donc CAP-3 (zone tampon, intersection) et CAP-9
   (lecture des formats) doivent être réimplémentées en Rust. La robustesse
   numérique des prédicats géométriques est précisément ce qui fait la valeur de
   GEOS ; la refaire proprement est l'un des postes les plus coûteux du plan.
3. **La promesse du produit est une question ouverte planifiée en dernier.** Les
   *Open Questions* de la spec listent encore « Modèle et configuration
   minimale » et « Livraison du modèle local » comme non tranchées. Autrement dit,
   l'hypothèse dont dépend tout le produit — un modèle local quantifié sait
   composer une suite de commandes correcte — n'est vérifiée qu'à l'epic 5.
4. **Vitesse observée.** L'epic 1, pour la tranche la plus étroite possible
   (fenêtre, carte, fond PMTiles local, verrou hors ligne, recette), a demandé
   huit entrées. Extrapoler les epics 2, 3, 4 et 6 à ce rythme, sans GEOS, place
   la démo très loin.
5. **Passer par QGIS neutralise la contrainte de licence au MVP.** QGIS embarque
   GDAL et GEOS de son côté. L'extension les utilise dans le processus de QGIS ;
   le noyau Cassini ne les lie jamais. L'interdiction GPL/LGPL du noyau reste
   donc intacte **sans rien coûter au MVP**. Le problème de GEOS n'est pas
   résolu, il est reporté à l'application autonome.

**Ce qui n'est pas en cause.** Le verrou hors ligne dans le moteur, ses 17 tests,
le générateur déterministe de l'archive de recette, le chemin de compilation
Windows et le README sont tous soit encore sur le chemin critique, soit du
matériel de V2 bon marché. Rien de ce qui est construit n'est faux.

## 2. Analyse d'impact

### 2.1 Impact sur les epics

| Epic actuel | Verdict | Ce qui change |
|---|---|---|
| 1 — `epic-socle-et-carte` | **rééchelonné** | Devient « socle, verrou hors ligne et recette ». Les entrées 1.6 (recette automatique) et 1.7 (nettoyage) restent valides et utiles dans tous les cas. La carte MapLibre et la coquille Tauri quittent le chemin critique du MVP et restent au dépôt comme fondation de V2 — l'entrée 1.7 ne doit pas les supprimer. |
| 2 — `epic-registre-projet-import` | **redéfini** | Le registre de commandes devient le cœur du MVP et remonte en tête. Projet et import passent à QGIS. L'epic est renommé « registre de commandes et journal rejouable » et récupère CAP-7. |
| 3 — `epic-analyses-et-historique` | **retiré** | CAP-3 part à la façade QGIS, CAP-7 part à l'epic 2. Rien n'y est construit, le retrait ne coûte rien. L'identifiant 3 n'est jamais réattribué. |
| 4 — `epic-raster-et-terrain` | **réduit** | Reste au noyau, mais passe de neuf commandes à trois (champ de vision, ligne de vue, pente). Motif en 2.4. |
| 5 — `epic-agent-et-mcp` | **remonté** | Devient le centre du MVP, juste après le registre et la façade, et il est précédé d'une épreuve de dérisquage. |
| 6 — `epic-export-et-demo` | **rééchelonné** | L'export PDF passe à QGIS (CAP-5 exécutée par son moteur de mise en page). L'epic ne garde que le scénario de démo de bout en bout. |
| 7 — `epic-publication` | **élargi** | Ajoute la publication sur le dépôt d'extensions QGIS, qui devient le canal de distribution principal. La question de l'installeur change de forme : extension plus modèle, et non application complète. |

### 2.2 Epics nouveaux

- **`epic-epreuve-du-modele-local`** — vérifier, avant de bâtir dessus, qu'un
  modèle local quantifié sait transformer une phrase française en suite de
  commandes typées correcte, sur un registre factice. Ne dépend de rien, donc
  passe en premier. C'est le test le plus rentable du projet : s'il échoue, tout
  le reste est un SIG de plus.
- **`epic-facade-qgis`** — l'extension QGIS, le canal local entre elle et le
  noyau, et les commandes du registre exécutées par QGIS (ouverture de fichiers,
  couches du projet, analyses vectorielles, export de la mise en page).

### 2.3 Dépendances touchées

- `epic 2 after epic 1` tient, mais son motif change : le registre a besoin du
  workspace et de la recette, plus de la coquille Tauri.
- Les epics 3, 4, 5 et 6 attendaient de l'epic 2 « le contrat du registre **et le
  format de projet sur disque** ». La seconde moitié tombe : c'est QGIS qui
  possède le projet. Le noyau garde malgré tout un artefact sur disque, le
  **journal rejouable** (CAP-7), désormais écrit à côté du projet QGIS et non
  dans un format de projet propre. Cette redéfinition doit être écrite, sinon
  trois epics attendent une chose qui n'existera pas.
- `epic 6 after epic 3 et 5` devient `after epic 5, 9 et 4`.
- L'epic 5 gagne une dépendance sur l'épreuve du modèle local et sur la façade :
  l'agent a besoin d'un endroit où afficher son plan et le faire valider.

### 2.4 Pourquoi réduire le terrain à trois commandes

Point à discuter en priorité, parce qu'il va à l'encontre d'une ambition
affichée. En mode extension, QGIS sait déjà faire du champ de vision par GRASS et
SAGA ; garder neuf commandes de terrain dans le noyau n'apporte donc **aucun
avantage visible au MVP**. Ce qui reste vrai, en revanche, c'est que GRASS est en
GPL : un moteur de terrain propre en licence MIT dans le noyau est la seule base
sur laquelle le module défense privé pourra être construit, puisque ce module ne
peut pas être en GPL. Trois commandes suffisent à établir cette base, à répondre
à la question ouverte sur la maturité de `whitebox_next_gen`, et à démontrer sur
un poste isolé. Les six autres sont reportées sans être abandonnées.

## 3. Conflits dans les documents

### 3.1 Spec (`spec-cassini-mvp/spec-cassini-mvp.md`) — contrat machine

| Section | Conflit | Correction proposée |
|---|---|---|
| *Why* | Ne mentionne pas QGIS ; pose le prix d'ArcGIS comme le problème | Réécrire : le concurrent est QGIS, l'angle est l'interface en langage naturel hors ligne, pas le prix |
| CAP-1 Carte | Décrit une carte propre à Cassini | Réduire à ce que l'epic 1 a démontré : affichage hors ligne depuis des fichiers locaux, zéro réseau |
| CAP-3 Analyses | Promet une chaîne Rust `geo` | Garder la capacité, changer l'exécutant : commandes du registre **exécutées par le cadre de traitement de QGIS** |
| CAP-5 Export PDF | Export propre à écrire | Garder, exécuté par le moteur de mise en page de QGIS |
| CAP-9 Import | Cinq formats à implémenter | Réduire : QGIS ouvre les fichiers nativement ; le registre expose l'ajout et l'inventaire des couches |
| CAP-10 Projet | Format de projet Cassini sur disque | Réduire : le projet est un projet QGIS ; le noyau écrit son journal à côté |
| CAP-11 Terrain | Neuf commandes | Réduire à trois (voir 2.4), les six autres en non-objectif reporté |
| **CAP-13** *(nouveau)* | — | **Façade QGIS** : extension et canal local. Identifiant 13 parce que CAP-12 (fonction réservée) est réservé au module défense et qu'un identifiant n'est jamais réattribué |
| *Constraints* | Rien sur la frontière GPL | Ajouter : l'extension est en GPL et vit séparée ; le noyau reste Apache-2.0, ne lie jamais QGIS, communique à distance de bras par processus séparé, et **doit rester pleinement utilisable sans QGIS** — c'est ce qui fonde l'argument juridique et ce qui rend la V2 possible |
| *Non-goals* | « Système de plugins » est ambigu | Préciser : Cassini n'héberge pas de plugins, mais Cassini **est** une extension au MVP. Ajouter : application de bureau autonome reportée en V2 ; aucune parité visée avec QGIS |
| *Success signal* | La démo se déroule dans l'application Cassini | Réécrire : la démo se déroule dans QGIS, panneau Cassini ouvert |
| *Assumptions* | « Les étapes de §7 sont l'ordre de livraison » ; « un projet est un dossier » | Les deux tombent. Remplacer par les nouvelles |
| *Open Questions* | Fond de carte, isochrones, jeu de recette | Résolues ou sans objet : QGIS les porte. Nouvelles questions en §5 |

### 3.2 Architecture — `CLAUDE.md` §3

Pas de document d'architecture séparé ; le schéma de `CLAUDE.md` §3 en tient lieu
et doit être redessiné. Changements de fond : QGIS remplace Tauri et MapLibre
comme interface du MVP ; le noyau devient un processus local indépendant ; la
pile gagne Python et PyQGIS pour l'extension ; un **contrat de canal local**
apparaît, qui n'existe nulle part aujourd'hui et que deux moitiés construites
séparément pourraient choisir de façon incompatible. C'est exactement le cas où
`bmad-architecture` a sa place — recommandé avant l'epic de façade.

### 3.3 Interface et expérience

Aucun document d'interface n'existe, mais l'impact est réel et n'est pas écrit :
l'expérience devient un panneau ancré dans QGIS, outil dense et d'expert, alors
que l'argument de vente est « un opérateur qui n'est pas cartographe obtient sa
réponse en parlant ». La vision d'une application conversationnelle propre part
en V2. **Action requise** : esquisser le panneau avant d'ouvrir l'epic de façade,
sans quoi la démo ressemblera à QGIS avec un chat greffé.

### 3.4 Autres artefacts

- **`README.md`** — écrit à l'entrée 1.5, il encode l'ancienne feuille de route
  dans son tableau « ce qui est prévu » et l'ancienne architecture dans ses trois
  schémas Mermaid. **À corriger avant la poussée sur GitHub**, sinon le dépôt
  public annonce un plan abandonné.
- **`CLAUDE.md`** — §1 vision, §3 architecture, §5 formats, §6 périmètre, §7
  étapes, §9 structure du dépôt (ajout du dossier de l'extension), §10 licence
  (extension GPL face à noyau Apache-2.0). Réécriture importante.
- **Recette automatique (entrée 1.6, non construite)** — doit désormais contrôler
  aussi l'extension Python, pas seulement Rust et TypeScript. Sa description
  change avant d'être construite.
- **`app/`** — reste au dépôt comme fondation de V2. L'entrée 1.7 (nettoyage) ne
  doit pas la supprimer : sa description doit le dire explicitement.
- **`fixtures/`** — l'archive PMTiles perd son rôle au MVP, QGIS fournissant les
  fonds. À garder : elle prouve le verrou hors ligne.
- **`tests/`** — les tests d'intégration auront besoin d'un harnais côté Python.
- **`docs/compiler-sous-windows.md`** — reste valable pour le noyau ; une section
  QGIS viendra plus tard.

## 4. Chemin retenu

| Option | Verdict | Effort / Risque |
|---|---|---|
| **1. Ajustement direct** — modifier les entrées dans la structure d'epics actuelle | **Non viable** : le changement déplace une frontière entre deux produits, pas un détail dans un epic. Quatre capacités changent d'exécutant, deux epics apparaissent, un epic est retiré | faible / élevé — laisserait la spec mentir sur ce qui est construit |
| **2. Retour en arrière** — annuler du travail récent pour simplifier | **Non viable et inutile** : rien de ce qui est construit n'est faux. Annuler détruirait du travail testé sans rien gagner | moyen / élevé |
| **3. Révision du périmètre MVP** | **Viable — c'est le chemin** : le MVP change d'exécutant pour CAP-1, 3, 5, 9, 10, réduit CAP-11, ajoute CAP-13, et place le dérisquage en premier | élevé (documents) / moyen (frontière GPL, apprentissage de PyQGIS, dessin du panneau) |

**Retenu : option 3, avec un epic de dérisquage en tête.**

Justification : le délai avant de rencontrer de vrais utilisateurs passe
d'années à mois ; la promesse unique est vérifiée en première semaine au lieu de
la deuxième année ; la contrainte de licence cesse de coûter au MVP ; rien de ce
qui est déjà construit n'est jeté ; et le noyau reste propre en licence, donc la
V2 autonome et le module défense privé restent tous deux possibles.

Risques assumés, nommés : l'extension sera en GPL, ce qui interdit un module
défense construit *dans* l'extension — il devra s'appuyer sur le noyau ; le
raisonnement de distance de bras doit être confirmé le jour où de l'argent entre ;
et l'expérience utilisateur est désormais contrainte par celle de QGIS.

## 5. Nouvelle séquence proposée

| Ordre | Epic | `id` | `covers` | Attend |
|---|---|---|---|---|
| 1 | `epic-socle-et-recette` *(rééchelonné)* | 1 | CAP-1 | — |
| 2 | `epic-epreuve-du-modele-local` *(nouveau)* | 8 | — | — |
| 3 | `epic-registre-et-journal` *(redéfini)* | 2 | CAP-6, CAP-7 | 1, 8 |
| 4 | `epic-facade-qgis` *(nouveau)* | 9 | CAP-13, CAP-9, CAP-10, CAP-3, CAP-5 | 2 |
| 5 | `epic-agent-et-mcp` *(remonté)* | 5 | CAP-4, CAP-8 | 2, 8, 9 |
| 6 | `epic-terrain-noyau` *(réduit)* | 4 | CAP-11 | 2 |
| 7 | `epic-demo` *(rééchelonné)* | 6 | — | 4, 5, 9 |
| 8 | `epic-publication` *(élargi)* | 7 | — | 6 |
| — | `epic-analyses-et-historique` | 3 | *retiré* | — |

L'epic 2 est l'épine dorsale : il fixe le contrat du registre, que la façade,
l'agent et le terrain consomment tous les trois.

### Questions ouvertes créées par ce changement

- **Forme du canal local.** Processus séparé joint par quoi — entrée et sortie
  standard, socket local, fichiers d'échange ? C'est la décision qui porte
  l'argument de distance de bras, donc elle est juridique autant que technique.
- **Confirmation de la frontière GPL.** À faire confirmer avant toute vente, pas
  avant d'écrire du code.
- **Version plancher de QGIS** et surface de l'API PyQGIS réellement nécessaire.
- **Livraison du modèle.** Le dépôt d'extensions QGIS n'acceptera pas plusieurs
  gigaoctets de modèle GGUF. Comment l'extension le trouve-t-elle sur un poste
  isolé, et qui l'installe ?
- **Dessin du panneau** (voir 3.3).
- Reste ouverte, inchangée : la demande de classement ([retiré]) pour le module
  défense.

## 6. Classement et passation

**Classement : majeur** — replanification de fond.

| Ordre | Qui | Quoi |
|---|---|---|
| 1 | `bmad-spec` | Mettre à jour la spec : *Why*, CAP-1, 3, 5, 9, 10, 11, nouvelle CAP-13, *Constraints*, *Non-goals*, *Success signal*, *Assumptions*, *Open Questions* |
| 2 | Claude + propriétaire | Réécrire `CLAUDE.md` §1, §3, §5, §6, §7, §9, §10 |
| 3 | `bmad-architecture` | Contrat de frontière entre le noyau et l'extension — recommandé, pas optionnel : c'est le cas d'école de deux moitiés qui peuvent diverger |
| 4 | `bmad-ticket` | Appliquer la nouvelle séquence : rééchelonner 1, 2, 4, 6, 7, retirer 3, créer 8 et 9, écrire les entrées de l'epic 8 |
| 5 | Claude | Corriger `README.md` (feuille de route et schémas) **avant** la poussée sur GitHub |
| 6 | `bmad-build` | Entrée par entrée, en commençant par l'épreuve du modèle local |

**Critère de réussite de cette passation :** la spec, `CLAUDE.md`, l'arbre de
tickets et le README décrivent tous le même plan, et la première entrée prête à
construire est l'épreuve du modèle local.
