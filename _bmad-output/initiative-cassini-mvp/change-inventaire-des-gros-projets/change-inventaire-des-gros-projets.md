# Changement de cap : inventaire des gros projets (2026-11-10)

Statut : **approuvé** par le propriétaire le 2026-11-10 (« ok pour tout, on démarre » aux cinq
pistes, puis « ok pour les 6 recommandations » au détail). Mode batch. Portée : **modérée**.

Décisions du propriétaire (2026-11-10), les six questions du §5 tranchées selon la
recommandation :

1. forme réduite `{layer_id, name, geometry}` ;
2. `MAX_LENGTH` de l'entraînement porté à 4 608 après un essai d'une heure sur la carte de
   8 Go, gros projets à environ 25 % du jeu ; repli : gros projets plafonnés à 40 couches ;
3. durée : < 30 s sous le seuil (inchangé) ; gros projets < 60 s sur CPU et < 30 s sur GPU ;
4. la question entre couches proches est apprise par le modèle (famille du générateur), pas
   écrite dans le noyau ;
5. seuil du tri ≈ 9 000 caractères, couche détaillée plafonnée à 30 champs ou 3 000 caractères,
   à revoir en 48 sur les mesures ;
6. la branche `generateur-grands-inventaires` est fusionnée sans entraînement propre ; ses
   exemples (aucun au-delà de 1 773 jetons) rejoignent l'entraînement groupé de 49.

## 1. Le problème

- **Déclencheur :** un géomaticien essaiera d'abord Cassini sur son vrai projet, pas sur un
  projet de démo à six couches. Aujourd'hui, un projet de 40 couches ou plus, à 15–20 champs
  par couche, ne se compose pas : l'agent rend `ai.prompt_too_long` (« retirez des couches du
  projet », `ai/src/lib.rs`). Il faut régler cela **avant la vidéo de démo**
  (`epic-export-et-demo`).
- **Cause (vérifiée dans le code) :** `core/src/agent_inventory.rs` borne la vue du modèle à
  `INVENTORY_BUDGET_CHARS` = 14 000 caractères (environ 5 100 jetons, pour un contexte
  `N_CTX` = 8 192). Pour tenir ce budget, seules les couches de résultat sortent
  (`within_budget_by_batches`, tickets 5.18 et 5.19). Les couches de l'utilisateur ne sortent
  jamais : au-delà, c'est le contrôle exact de `cassini-ai` qui tranche, et il refuse.
- **Nature :** une limite technique, découverte en préparant la démo. Le budget a été taillé
  pour des sessions où les résultats s'accumulent (5.18), pas pour un gros projet de départ.
- **Mesure sur les données réelles (2026-11-10).** Un projet d'environ 60 couches a été chiffré
  hors de QGIS, avec un script jetable qui suit les règles de `agent_inventory` (valeurs d'un
  champ texte gardées s'il en a 10 au plus). Le projet : BD TOPO 3.5 tous thèmes du 74 (un
  GeoPackage, 54 couches), le PLUi d'Annecy (7 couches) et les deux MNT, posés dans
  `../donnees-test-cassini/annecy/extrait/`. Le cadastre n'y est pas.

  | Mesure (63 couches) | Caractères | Jetons (0,364 par car.) |
  |---|---|---|
  | Inventaire complet, forme de l'entraînement | **84 463** | **≈ 30 700** |
  | Couche médiane | 1 237 | ≈ 450 |
  | `troncon_de_route` (86 champs, dont 22 avec valeurs) | 6 411 | ≈ 2 330 |
  | `troncon_de_route` sans les valeurs | 4 362 | ≈ 1 590 |
  | Les 30 premiers champs de `troncon_de_route` | 2 947 | ≈ 1 070 |
  | Forme réduite `{layer_id, name, geometry}`, 63 couches | 5 699 | ≈ 2 075 |
  | Forme réduite `{layer_id, geometry}`, 63 couches | 3 956 | ≈ 1 440 |
  | Identifiants seuls, 63 couches | 1 617 | ≈ 590 |

  L'inventaire complet fait **six fois le budget**. Il dépasse aussi un contexte de 16 k, et
  même de 32 k une fois la réponse et les consignes comptées.
- **Constat qui pèse sur l'entraînement :** le modèle n'a jamais vu une séquence de plus de
  2 048 jetons (`training/entrainement/pipeline.py`, `MAX_LENGTH`). À l'entraînement, la
  médiane est de 749 jetons et le maximum de 1 659 (`docs/entrainement-du-modele.md`). Dès
  aujourd'hui, un inventaire proche du budget (≈ 5 100 jetons) sort donc de ce que le modèle
  connaît.
- **Constat connexe, à mesurer :** QGIS met du temps à relever les valeurs d'un projet aussi
  gros (`uniqueValues`, `qgis-plugin/cassini_qgis/inventory.py`). La lecture SQL hors QGIS a
  pris environ 6 min pour les 54 couches, mais sur un disque Windows lu depuis WSL, donc ce
  chiffre ne vaut pas mesure. Le relevé est gardé d'une phrase à l'autre (e25bbdc) : seule la
  première phrase de la session le paie.

## 2. Analyse d'impact

| Endroit | Effet |
|---|---|
| Spec, CAP-4 | Le critère vise « la phrase de démo sur un projet où les couches nécessaires sont ouvertes » : la taille du projet n'y figure pas. On propose d'y ajouter un projet réel d'environ 60 couches (§4). Le MVP reste atteignable. |
| AD-5 (le noyau tient la boucle) | Respecté. Le tri et la redemande élargie vivent dans le noyau (`core/src/serve/plans.rs`, `plan.propose`). L'extension ne voit rien de nouveau. |
| AD-23 (le modèle n'émet que des appels du registre) | Respecté, avec une précision à écrire : une **redemande élargie**, la même demande avec une vue plus détaillée, n'est pas une réparation. Le plan rendu reste jugé par les mêmes schémas, et aucune sortie n'est corrigée à la main. Le garde-fou des coordonnées s'applique à chaque composition, comme avant. |
| AD-11 (journal, rejeu) | Non touché. Le journal garde les vrais identifiants, et la vue du modèle n'est pas journalisée (règle de 5.18). Le rejeu ne compose pas. `plan_step` (5.34) ne change pas. |
| AD-29, CAP-8 (MCP, IA externe) | Non concerné. L'IA externe lit `project.inventory` brut et appelle les commandes une à une ; `model_inventory` ne sert qu'à `plan.propose`. Un test le confirmera. |
| AD-9 et AD-15 | Inchangés. Le modèle désigne une couche par son identifiant court, que le noyau retraduit (5.19). Le modèle reste lié au processus. |
| 5.10 t4a (forme apprise) et `training/tests/forme_inventaire.rs` | Une clé nouvelle (`index`) et une sorte de couche nouvelle (la forme réduite). Le test doit les voir à l'entraînement : il passe au rouge si le noyau rend la forme avant que le générateur la produise. D'où l'ordre du §4. |
| 5.18 (budget) | Sous le seuil du tri, rien ne change. Au-dessus, le budget de 14 000 caractères est partagé : d'abord l'index, puis les détails. Le contrôle exact de `cassini-ai` reste le dernier filet. Son message change (« le projet a trop de couches pour le modèle »). |
| 5.19 (ordre stable, ShortIds, cache) | **L'index est stable**. Il contient toutes les couches, utilisateur d'abord puis résultats ; un plan exécuté n'ajoute ses couches qu'à la fin, donc « l'invite d'avant reste un préfixe » tient jusqu'à la fin de l'index. **Les détails changent avec la phrase**, et ils sont relus à chaque phrase dont le jeu de couches détaillées change. Les découpes de `layer_cuts` et l'instantané d'ancrage (`SLOT_ANCHOR`) se placent à la fin de l'index. ShortIds ne change pas : la table est faite sur toutes les couches. |
| 5.22 et 5.34 (intermédiaires, résultats sans valeurs) | Gardés. Les résultats récents sont montrés en détail, sans valeurs ; les plus anciens passent dans l'index. |
| 5.33 (couches retirées, signalées) | La mention garde son sens, mais devient rare : un vieux résultat reste dans l'index au lieu de disparaître. Elle ne joue plus que si l'index déborde. |
| Durée par phrase (exigence < 30 s sur CPU) | Sous le seuil, inchangée. Au-dessus, voir le chiffrage ci-dessous : sur CPU, une phrase qui change de couches détaillées dépasse 30 s. |
| Générateur `training/` | Une famille « gros projet » de 20 à 60 couches, rendue **par la fonction du noyau**, et une famille « deux couches proches → question ». Le `MAX_LENGTH` de l'entraînement passe au-dessus de 2 048 (question 2). |
| Branche `story/generateur-grands-inventaires` | Elle n'est pas touchée. Ses couches à 15–20 champs sont la matière de la partie détaillée. Un seul ré-entraînement, après les deux (§4). Elle bute sans doute déjà sur `MAX_LENGTH` (six couches à 1 300 caractères font ≈ 2 900 jetons) : à lui signaler. |
| Panneau, UX | Aucun changement visible. Une redemande élargie allonge seulement l'attente de la composition. Les questions passent par le chemin des questions qui existe déjà. |
| Recette 5.45 (Annecy, 16/20) et `plan_reel`, `question_reel`, `ecart_reel` | Petits inventaires, sous le seuil : ils servent de non-régression et doivent garder leurs scores. |

**Chiffrage de la durée sur CPU** (lecture d'environ 57 jetons/s à 8 fils,
`docs/mesure-duree-par-phrase.md`) :

- Première phrase sur le projet de 63 couches : l'index (≈ 2 075 jetons, ≈ 36 s) s'ajoute à la
  première lecture. Ensuite il est gardé en cache.
- Phrase suivante, sur les mêmes couches détaillées : aucune relecture en plus, comme
  aujourd'hui.
- Phrase suivante, sur d'autres couches : il faut relire les détails, 3 000 à 6 000 caractères
  (≈ 1 100 à 2 200 jetons), soit **19 à 38 s de lecture**, plus l'écriture du plan (≈ 15 s).
  C'est donc **35 à 55 s sur CPU**, et quelques secondes sur GPU (5.21).
- Redemande élargie : elle ne relit que les détails, plus une seconde écriture, soit environ
  +20 à 40 s sur CPU. Elle ne joue que si le tri a manqué une couche.

**Contexte 16 k ou 32 k (piste 5), en une ligne :** l'inventaire mesuré (≈ 30 700 jetons) ne
tient pas dans 16 k et à peine dans 32 k ; sa première lecture prendrait environ 9 min sur CPU,
le cache du moteur grossirait d'autant, et ce serait sur un modèle jamais entraîné au-delà de
2 048 jetons. Piste écartée, gardée en tout dernier recours.

## 3. Voie recommandée

**Ajustement direct**, en cinq entrées dans l'epic 5, sans retour en arrière ni réduction du
MVP.

1. **Seuil.** Tant que la vue complète des couches de l'utilisateur tient sous un seuil de
   ≈ 9 000 caractères, la vue reste exactement celle d'aujourd'hui. Villeneuve fait 4 007
   caractères ; Annecy est sous le seuil, à vérifier. Le chemin de la démo, le cache et les
   barres actuelles ne bougent donc pas. Le tri ne joue qu'au-dessus.
2. **Vue triée, au-dessus du seuil.** Elle est déterministe et écrite en Rust, sans dépendance
   nouvelle ; c'est une fonction pure de (inventaire, phrase, journal). Elle a deux parties.
   - **`index`** : toutes les couches, dans l'ordre stable de 5.19, en forme réduite. Les
     couches détaillées y figurent aussi : c'est ce qui rend l'index identique d'une phrase à
     l'autre.
   - **`layers`** : les couches en détail, dans la forme apprise, rangées dans l'ordre stable.
     On détaille, par priorité : les couches que la phrase désigne, les résultats du dernier
     plan exécuté (sans valeurs, 5.22), les couches citées par ce plan (lues dans le journal),
     et les rasters (leur forme complète est à peine plus longue que la forme réduite).
     On s'arrête au budget des détails (14 000 caractères moins l'index).
3. **Correspondance phrase ↔ couche.** On normalise : minuscules, accents ôtés par une table
   écrite à la main (pas de dépendance), découpe sur tout ce qui n'est pas une lettre ou un
   chiffre (dont `_` et `-`), petite liste de mots vides, pluriel simple (`s` et `x` finals
   ôtés au-delà de 3 lettres, `eaux` → `eau`). Un mot de la phrase d'au moins 3 lettres trouve
   une couche s'il égale un mot de son nom, du radical de son identifiant ou du radical de son
   fichier : `gares` donne `gare`, qui trouve `gare_sncf`. Il la trouve aussi, à rang plus
   faible, s'il égale un mot d'un nom de champ ou d'une valeur ; une valeur de 1 ou 2 signes
   (`U`, `AU`) doit être égale exactement, casse comprise. Priorité : nom, puis champ, puis
   valeur ; à égalité, l'ordre stable.
4. **Une couche de 88 champs.** Une couche détaillée garde la forme apprise, mais au plus 30
   champs ou 3 000 caractères. On choisit d'abord les champs que la phrase désigne, puis les
   champs texte qui ont des valeurs, puis les champs numériques, puis les autres, dans l'ordre
   de QGIS à chaque rang. Pas de clé nouvelle : le modèle a déjà vu des couches à peu de
   champs. Pour `troncon_de_route`, on passe de 6 411 à moins de 3 000 caractères en gardant
   `nature`, `importance`, `vitesse_moyenne_vl` et les autres champs à valeurs. La règle est
   générique : elle ne sait rien de la BD TOPO.
5. **Redemande élargie.** Si le plan rendu fait porter un **paramètre de champ** (champ, valeur,
   `label_field`) sur une couche montrée réduite, le noyau recompose **une fois**, cette couche
   détaillée en plus. Un plan qui ne prend la couche réduite que comme géométrie (« à moins de
   500 m des gares ») est accepté tel quel : il n'avait besoin que de son identifiant. C'est
   l'alternative « refus du plan → redemande », restreinte au seul cas où le modèle a dû
   deviner. Elle ne demande ni sortie nouvelle du modèle, ni ré-entraînement, ni règle de
   prompt.
6. **Question entre couches proches.** Quand deux couches se disputent un mot (deux couches de
   gares), le tri les détaille toutes les deux, et le modèle apprend à poser la question par une
   famille du générateur. Le noyau ne pose pas cette question lui-même (décision du 2026-10-22 :
   une phrase mal comprise devient une famille d'exemples, jamais un pansement).
7. **Un seul ré-entraînement**, après la branche `generateur-grands-inventaires` et la famille
   « gros projet ». Le générateur rend ses gros projets en appelant la fonction du noyau, ce qui
   garantit la forme ; `forme_inventaire.rs` le vérifie.
8. **Recette réaliste** sur le projet BD TOPO + PLUi + MNT : 20 phrases à plan attendu, barre
   ≥ 16/20, comme 5.45. Annecy 16/20 et les trois jeux GPU servent de non-régression.

**Effort :** cinq entrées, dont un ré-entraînement d'une nuit (deux graines). **Risque : élevé**,
parce que le changement touche la composition, le cache et l'entraînement : relecture
`thorough` (CLAUDE.md §8). **Calendrier :** l'epic de démo attend la recette réaliste.

**Alternatives écartées :**

- Contexte agrandi : chiffré au §2.
- Détailler toutes les couches sauf les plus grosses : ne tient pas à 63 couches.
- Une sortie « donne-moi la couche X » apprise au modèle : grammaire nouvelle, ré-entraînement,
  et un appel de plus à chaque fois.
- Index et détails mêlés dans une seule liste `layers` : le cache casserait à la première
  couche qui change de forme, soit environ la moitié de l'inventaire relue à chaque phrase.

## 4. Changements proposés

### Spec — `spec-cassini-mvp/spec-cassini-mvp.md`, CAP-4, critère de réussite

ANCIEN :

> - **success:** sur un projet où les couches nécessaires sont déjà ouvertes, la phrase de démo
>   de CLAUDE.md §1 produit une suite de commandes affichée étape par étape dans le panneau
>   Cassini de QGIS, que l'utilisateur valide avant exécution et peut rejouer ensuite à
>   l'identique.

NOUVEAU :

> - **success:** sur un projet où les couches nécessaires sont déjà ouvertes, la phrase de démo
>   de CLAUDE.md §1 produit une suite de commandes affichée étape par étape dans le panneau
>   Cassini de QGIS, que l'utilisateur valide avant exécution et peut rejouer ensuite à
>   l'identique ; sur un projet réel d'environ 60 couches (une BD TOPO départementale et un
>   PLU), au moins 80 % des phrases de la recette réaliste donnent le plan attendu, sans
>   `ai.prompt_too_long` (changement de cap du 2026-11-10).

### Architecture — `architecture-noyau-et-facade-qgis.md`, AD-23, nouvel alinéa sous **Rule**

> - **Vue triée d'un gros projet (2026-11-10).** Au-delà d'un seuil, le modèle voit un index
>   réduit de toutes les couches et le détail de celles que la demande désigne. La vue est une
>   fonction pure du noyau (inventaire, demande, journal), jamais journalisée. Si un plan fait
>   porter un paramètre de champ sur une couche montrée réduite, le noyau recompose une fois
>   avec cette couche détaillée : c'est une redemande, pas une réparation. Le plan rendu passe
>   par les mêmes schémas et le même garde-fou, et le rejeu (AD-11) n'en dépend pas.

Rien d'autre ne change dans l'architecture : AD-5, AD-11 et AD-29 sont respectées telles
qu'écrites (§2).

### Code et docs (dans les entrées ci-dessous, pas maintenant)

- `core/src/agent_inventory.rs` : doc du module et nouvelle fonction de vue ; le message de
  `ai.prompt_too_long` dans `ai/src/lib.rs` ; les découpes de `ai/src/plan.rs`.
- `docs/entrainement-du-modele.md` : forme de la vue, `MAX_LENGTH`.
- `docs/mesure-duree-par-phrase.md` : mesures sur le gros projet.

### Epic 5 — `epic-agent-et-mcp/tickets.toml`, entrées nouvelles, dans cet ordre

| id | Titre | Risque | Après | Vérification |
|---|---|---|---|---|
| 46 | Vue triée de l'inventaire d'un gros projet : index réduit et couches détaillées par la demande | high | — | Tests du noyau sans modèle : sous le seuil, vue identique à aujourd'hui, octet pour octet. Au-dessus : `gares` détaille `gare_sncf` ; une valeur `U` ne trouve que `U` ; une couche de 88 champs garde au plus 30 champs ou 3 000 caractères ; l'index est identique pour deux phrases différentes ; l'inventaire du projet BD TOPO, figé en fixture synthétique de même forme, tient au budget ; la vue est déterministe. La fonction n'est pas encore branchée à `plan.propose`. |
| 47 | Générateur : gros projets de 20 à 60 couches rendus par la vue du noyau, et deux couches proches → question | medium | 46 | `forme_inventaire.rs` voit `index` et la forme réduite ; `recouvrement.rs` vert ; part des gros projets et longueurs de séquence consignées (médiane, maximum) ; aucune phrase des recettes ne vient au générateur. Construit sur `generateur-grands-inventaires` une fois fusionnée. |
| 48 | Vue triée branchée à la composition, cache placé à la fin de l'index, redemande élargie | high | 46, 47 | Tests sans modèle : un plan qui prend un champ d'une couche réduite déclenche une seule redemande, avec la couche détaillée ; un plan qui ne prend que sa géométrie n'en déclenche pas ; `project.inventory` du MCP reste inchangé ; le rejeu est identique. Tous les chemins de composition passent par la vue (canal, `cli` dont `cli/src/main.rs`, mesures). Durées CPU et GPU consignées sur le gros projet ; `ai.prompt_too_long` rend son nouveau message. |
| 49 | Ré-entraînement groupé : grands inventaires, gros projets, couches proches | high | 47, 48, entrée de `generateur-grands-inventaires` | D'abord une mesure de mémoire à la nouvelle `MAX_LENGTH` sur la carte de 8 Go (essai d'une heure). Puis deux graines la même nuit. Barres : Annecy ≥ 16/20, `plan_reel`, `question_reel` et `ecart_reel` pas en baisse. Installé seulement si les barres tiennent. |
| 50 | Recette réaliste : projet BD TOPO 74 + PLUi + MNT, 20 phrases à plan attendu (barre 16/20) | medium | 49 | Sonde versionnée comme 5.45 ; données hors dépôt ; 20 phrases enchaînées dans une seule session QGIS, dont au moins deux avec couches proches (question attendue) et deux dont la couche n'est trouvée que par une valeur ou un synonyme ; score N/20 ; rejeu identique ; aucune `ai.prompt_too_long` ; durées par phrase (CPU et GPU) et durée du premier inventaire QGIS consignées. |

Décision à ajouter à `epic-agent-et-mcp.md` :

> - Decision: 2026-11-10 — changement de cap `change-inventaire-des-gros-projets` : entrées 46 à
>   50, un seul ré-entraînement (49) après la branche des grands inventaires ; la démo filmée
>   attend 50.

### Epic de démo — `epic-export-et-demo/tickets.toml`, entrée 6

ANCIEN : `after = [5]` — NOUVEAU : `after = [5, "5.50"]`. La répétition de la démo, puis la
vidéo, attendent la recette réaliste.

## 5. Passage à la réalisation

- **Portée : modérée.** Elle réorganise le backlog (cinq entrées, une dépendance entre epics),
  amende AD-23 et CAP-4, et suppose un ré-entraînement. Le MVP n'est ni réduit ni replanifié.
- **À appliquer avec `bmad-ticket`** : les entrées 46 à 50 de l'epic 5, la décision de l'epic 5
  et le `after` de l'entrée 6 de l'epic de démo. Les amendements de CAP-4 et d'AD-23 vont
  directement dans leurs fichiers, une fois le document approuvé.
- **Puis `bmad-build`** dans l'ordre 46, 47, 48, 49, 50. On attend que la branche
  `generateur-grands-inventaires` soit fusionnée avant 47, et qu'aucun entraînement ne tourne
  avant de changer de branche.
- **Réussite :** sur le projet d'environ 60 couches, au moins 16 phrases sur 20 donnent le plan
  attendu, sans `ai.prompt_too_long`. Annecy reste à 16/20 au moins, les jeux GPU ne baissent
  pas, et sous le seuil les durées d'aujourd'hui sont inchangées.

**Questions qui restent au propriétaire (avec recommandation) :**

1. **Forme réduite.** Choix entre `{layer_id, name, geometry}` (5 699 caractères pour 63
   couches) et `{layer_id, geometry}` (3 956). *Recommandation : avec `name`.* Le nom affiché
   peut différer de l'identifiant (un PLUi renommé) ; l'index n'est lu qu'une fois par session.
2. **`MAX_LENGTH` de l'entraînement.** Une séquence de gros projet fait ≈ 3 500 à 4 100 jetons,
   au-delà des 2 048 actuels. *Recommandation : 4 608, mesuré d'abord par un essai d'une heure
   sur la carte de 8 Go ; gros projets limités à ≈ 25 % du jeu, pour ne pas multiplier la durée
   de la nuit.* Repli : plafonner les gros projets à 40 couches.
3. **Durée sur CPU des gros projets.** Au-dessus du seuil, une phrase qui change de couches
   détaillées dépasse 30 s sur CPU (estimé 35 à 55 s). *Recommandation : garder < 30 s sous le
   seuil, et se donner pour les gros projets une cible de < 60 s sur CPU et < 30 s sur GPU,
   mesurée en 48 et 50.*
4. **Qui pose la question entre couches proches.** *Recommandation : le modèle, appris par une
   famille du générateur ; le tri garantit que les deux couches sont détaillées.* Pas de
   question écrite dans le noyau.
5. **Seuil du tri (≈ 9 000 caractères) et plafond d'une couche détaillée (30 champs ou 3 000
   caractères).** *Recommandation : ces valeurs, à revoir en 48 sur les mesures.*
6. **Branche `generateur-grands-inventaires`.** *Recommandation : la fusionner sans lancer son
   propre entraînement, et lui signaler la limite de 2 048 jetons.* Ses exemples rejoignent
   l'entraînement groupé de 49.
