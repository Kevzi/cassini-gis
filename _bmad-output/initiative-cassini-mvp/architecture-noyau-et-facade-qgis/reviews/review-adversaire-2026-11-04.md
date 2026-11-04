# Relecture adverse — amendement du 2026-11-04 d'AD-24 (désignation d'un raster délégué)

Date : 2026-11-04. Objet : la phrase ajoutée à la sous-règle **Genres** d'AD-24 :

> Un raster délégué se désigne par son identifiant de couche (AD-9), ou par un chemin relatif au
> projet quand il sort d'une étape du noyau : une référence `step:k` vers le champ `file` d'une
> commande `runs_in: core` (AD-9, AD-23 ; amendé le 2026-11-04).

Lentille : construire, un niveau plus bas, deux unités qui obéissent chacune à la lettre de toutes
les AD et qui pourtant ne s'assemblent pas (formes de données partagées qui se heurtent, deux
propriétaires d'une même entité, deux chemins de mutation d'état). Chaque paire est un trou ; chaque
trou reçoit le texte de règle minimal qui le ferme. Le contrat n'est pas modifié par cette relecture.

Unités examinées : `raster.reclassify` (épopée 10, entrée 25), `raster.zonal_statistics` (entrée 12),
l'ouverture des sorties raster du noyau dans QGIS (épopée raster-et-terrain, entrée 3, faite),
`terrain.slope` (entrée 2, faite), le journal et le rejeu (AD-11), l'agent qui compose le plan
(AD-23), le relais d'IA externe (AD-29).

## Verdict

**À resserrer avant l'entrée 25.** L'intention est juste — permettre « pente puis reclassement » sans
inventer un identifiant de couche que le noyau ne connaît pas — mais la phrase autorise **deux formes
dans un même paramètre sans dire qui les distingue**, et le mécanisme qui la porte (`step:k` résolu en
chaîne, puis journalisé) contourne les repères `out:k.n` d'AD-11. Telle qu'écrite, deux constructeurs
fidèles au contrat produiront soit un journal qui contient un chemin absolu, soit un exécutant qui
reçoit un identifiant de couche transformé en chemin ; et dans les deux cas un rejeu qui relit la
sortie d'origine au lieu de la sortie rejouée, en rendant `identical: true`.

Sept trous ; les trois premiers sont bloquants pour l'entrée 25.

## Ce que le code montre (état existant)

- `resolve_references` (`core/src/plan.rs` l.128) remplace un paramètre **de premier niveau** entier
  par la valeur brute de la sortie référencée. Depuis le 2026-11-04, cette valeur brute porte des
  chemins **absolus** (`serve/plans.rs` l.~327 : « Résultat brut, chemins absolus compris »).
- `check_plan` (`plan.rs` l.~240) n'accepte `step:k.f` que si le schéma du paramètre est **égal** au
  schéma du champ `f` de la sortie. `TerrainSlopeOutput.file` est un `String` nu
  (`commands/terrain_slope.rs` l.47) : le paramètre consommateur doit donc être un `String` nu.
- Le journal ne relativise un chemin **que** pour les noms listés par `.path_params(..)`
  (`journal/mod.rs` l.~530, `resolve_param`). Tout autre champ est journalisé tel quel. Il n'existe
  aucune relativisation générale des paramètres.
- Un chemin de `path_params` qui ne commence pas par `out:` est traité comme **entrée utilisateur** :
  forme relative au journal, forme absolue à l'exécutant, et `inputs[]` avec taille et date
  (`input_entry`, l.964). Le rejeu vérifie taille et date (`check_inputs`, l.715) puis réexécute avec
  ce même chemin. Rien ne convertit une sortie de plan résolue en repère `out:k.n`.
- Le rejeu lie les identifiants de couche (`bound`, `substitute` l.951) uniquement pour les
  `layer_id` rendus par une étape journalisée.
- L'ouverture des rasters du noyau dans QGIS (entrée 3) est décidée par l'extension
  (`raster_output.rasters_to_open` : `runs_in == "core"`, `file` en `.tif`), n'est **pas
  journalisée** et ne rend au noyau **aucun** identifiant de couche (note de conception du plan de
  l'entrée 3 : « c'est un affichage »).
- `intermediate_files` (5.34, `journal/mod.rs` l.~429) ne reconnaît une sortie consommée que par
  le `layer_id` qu'elle a rendu ; une étape du noyau n'en rend pas.

## Paires incompatibles

### T1 — Un seul champ, deux formes : qui décide si la chaîne est un id ou un chemin ? (bloquant)

- **Unité A** — `raster.reclassify` écrit selon la lettre : paramètre `raster: String` (seule forme
  que `check_plan` accepte pour `step:k.file`), déclaré dans `path_params` pour que le chemin venu
  d'une étape soit journalisé relatif (AD-9).
- **Unité B** — le même `raster.reclassify`, appelé avec un identifiant de couche (forme 1 de
  l'amendement, celle de l'entrée 12).
- **Heurt.** Avec `path_params`, l'identifiant `mnt_3f2a…` passe par `resolve_path` : il est
  journalisé comme un chemin relatif `mnt_3f2a…`, ajouté à `inputs[]` (taille et date nulles,
  donc jamais contrôlées), et l'exécutant reçoit `<projet>/mnt_3f2a…`, un chemin absolu qui
  n'existe pas. Sans `path_params`, l'identifiant passe bien, mais le chemin issu de `step:k.file`
  est journalisé **absolu** — contraire à AD-9 — et n'apparaît pas dans `inputs[]`. Aucune des deux
  constructions n'est fautive au regard du contrat ; elles sont incompatibles entre elles et chacune
  casse l'autre forme.
- **Côté extension**, la même chaîne doit être interprétée : chercher une couche par identifiant puis
  retomber sur un fichier est une décision de l'exécutant (AD-5). Le cadre de traitement de QGIS
  résout lui-même une chaîne de couche par identifiant, puis par source, puis par **nom** de couche
  (comportement de `QgsProcessingUtils::mapLayerFromString`, à vérifier sur les deux séries) : une
  chaîne qui n'est pas un identifiant peut désigner une couche par son nom, ce qu'AD-9 interdit.
- **Le schéma ne peut pas le dire.** `resolve_references` ne remplace qu'un champ de premier niveau
  entier et `check_plan` exige l'égalité de schéma : une forme étiquetée
  (`{"layer_id": …} | {"path": …}`, ou `oneOf`) refuse `step:k.file` en `plan.type_mismatch`. La
  phrase de l'amendement est donc inconstructible proprement avec la grammaire de référence actuelle.
- **Fermeture proposée** (une des deux, à trancher par le propriétaire) :
  1. **Une forme par genre, déclarée au registre.** « Un raster, délégué ou non, se désigne **toujours**
     par un chemin relatif au projet, recopié de l'inventaire ou obtenu d'une étape. Un vecteur se
     désigne toujours par son identifiant de couche. Aucun paramètre n'accepte les deux. » C'est la
     forme que prennent déjà les trois commandes `terrain.*` ; QGIS sait ouvrir un raster par chemin,
     et le chemin est la seule forme qui survit au rejeu (T2, T4). L'entrée 12 passe alors au chemin.
  2. **Deux champs exclusifs.** « Un paramètre raster délégué est deux champs de premier niveau
     exclusifs, `raster_layer` (identifiant, AD-9) et `raster_file` (chemin, dans `path_params`) ;
     exactement un est présent, vérifié par le noyau avant l'ordre ; l'extension n'interprète jamais
     une chaîne pour en deviner la nature. » Plus lourd pour le modèle (deux noms pour une notion).

### T2 — `step:k.file` contourne les repères `out:k.n` : le rejeu relit la sortie d'origine (bloquant)

- **Unité A** — le plan `terrain.slope(dem: "dem/mnt.tif")` puis
  `raster.reclassify(raster: "step:1.file")`, exécuté par `plan.execute` selon la lettre d'AD-23.
- **Unité B** — le rejeu d'AD-11, qui écrit ses sorties sous `replay/` et « lie chaque repère à
  l'identifiant neuf ».
- **Heurt.** À l'exécution, `step:1.file` est résolu en `/…/projet.cassini/7-terrain.slope.tif`
  (numéro de journal, pas de plan), puis journalisé comme **entrée utilisateur**
  `projet.cassini/7-terrain.slope.tif` avec taille et date. Au rejeu, l'étape 7 réécrit la pente sous
  `replay/7-terrain.slope.tif`, mais l'étape 8 relit `projet.cassini/7-terrain.slope.tif`, la sortie
  **d'origine**. Si elle est intacte, le rejeu rend `identical: true` sans avoir enchaîné ce qu'il a
  recalculé ; si le dossier de travail a été vidé, il échoue en `Diverged` sur une « entrée changée
  ou disparue » qui est en réalité une sortie. AD-11 dit l'inverse : les sorties se désignent par des
  repères locaux à l'étape.
- Ce trou n'est pas propre aux rasters : `geocode.ban` → `layer.add(path: step:k.path)` (entrée 16)
  le rencontrera aussi. L'amendement le rend atteignable dès l'entrée 25.
- **Fermeture proposée (AD-11, resserré) :** « Quand une référence `step:k.<champ>` d'un plan se
  résout en un fichier de sortie nommé par le noyau pour l'étape *k*, le journal l'écrit sous la
  forme du repère `out:<étape de journal>.<n>`, jamais comme une entrée ; le rejeu la résout dans
  son propre dossier de sortie. Un chemin du dossier de travail donné littéralement (hors référence)
  est refusé ou converti de la même façon. » Test associé : un rejeu de « pente puis reclassement »
  dont le reclassement lit `replay/…`.

### T3 — Le garde-fou de provenance n'existe pas : rien n'empêche un chemin littéral (bloquant)

- **Unité A** — l'amendement : le chemin n'est admis « que quand il sort d'une étape du noyau », par
  `step:k` vers `file`.
- **Unité B** — `check_plan` et la validation par schéma (AD-6, AD-23), qui ne voient qu'un `String`.
- **Heurt.** Un plan qui écrit `raster: "dem/mnt.tif"` (raster que l'utilisateur a lui-même ouvert,
  recopié de l'inventaire comme le modèle l'apprend pour `terrain.*` en 4.6) passe la validation.
  La condition de l'amendement n'a pas de lieu d'application, contrairement à la règle des
  coordonnées d'AD-23, qui nomme sa fonction pure, son type marqué et ses chemins de composition.
  Le champ `file` n'est pas un type : toute commande `core` future qui rend un `file` non raster
  (CSV, PDF) satisfait la lettre de la règle.
- **Le cas de l'utilisateur.** Le même MNT ouvert à la main est désigné par **chemin** pour
  `terrain.slope` et par **identifiant** pour `raster.reclassify` ou `raster.zonal_statistics` : une
  seule entité, deux désignations selon le `runs_in` du consommateur, que le modèle doit apprendre à
  distinguer (exemples du générateur de 5.10 et du ré-entraînement 5.41). C'est une source d'erreur
  de planification sans aucun bénéfice — l'agent ne sait pas, et ne doit pas savoir, qui exécute.
- **Fermeture proposée** (AD-23, sous-règle **Fichiers**, sur le patron de **Coordonnées**) :
  « Tout chemin de raster du registre est le type partagé `RasterPath` du noyau, marqué dans le
  schéma engendré. Un `RasterPath` d'un plan vient de la `source` d'une couche raster de
  l'inventaire, d'une référence `step:` vers un champ de sortie déclaré `RasterPath`, ou d'un choix
  de fichier dans le panneau ; sinon le plan n'est pas montré. Le contrôle est une fonction pure du
  noyau appliquée à tous les chemins de composition. » Avec la fermeture 1 de T1, la question
  « id ou chemin » disparaît : il n'y a plus qu'un genre de désignation par genre de donnée.

### T4 — Deux propriétaires de la couche de pente : le noyau (fichier) et l'extension (couche)

- **Unité A** — l'entrée 3 : l'extension ouvre d'elle-même tout GeoTIFF rendu par une commande
  `core`, le nomme du radical du fichier, ne journalise rien et ne rend aucun identifiant.
- **Unité B** — `raster.zonal_statistics` (entrée 12), qui prend un identifiant de couche, et le
  rejeu, qui ne lie que les identifiants rendus par une étape journalisée.
- **Heurt.** Au tour suivant, l'inventaire montre la couche de pente avec un identifiant `X` que le
  noyau n'a jamais reçu. Un plan `raster.zonal_statistics(raster: "X")` est valide (AD-23 : `X` est
  connu de la session). Au rejeu, `X` n'est lié à rien : dans la même session, l'étape lit la pente
  **d'origine** encore ouverte ; dans une nouvelle session, elle échoue sur une couche absente. La
  même entité a deux propriétaires — le noyau possède le fichier et son repère, l'extension possède
  la couche et son identifiant — et seule la moitié du noyau est rejouable.
- **Écart au contrat déjà là.** AD-6 (précision du 2026-10-18) : « toute action sur le projet ou
  sur les données est une commande du registre » ; Conventions, *Mutation d'état* : « le projet est
  muté par QGIS seul, **sur ordre du noyau** ». `addMapLayer` décidé par l'extension sur un critère
  qu'elle évalue elle-même (`runs_in`, extension `.tif`) est une mutation du projet hors registre.
  L'entrée 3 l'a qualifiée d'« affichage », mais rien dans le contrat ne définit cette catégorie.
- **Fermeture proposée** (une des deux) : soit une règle explicite « Afficher une sortie du noyau
  n'est pas une commande : l'extension ajoute au projet, sans journal, le fichier que nomme un champ
  marqué `RasterPath` d'un résultat `core` ; la couche ainsi posée n'est jamais désignée par son
  identifiant dans un plan, seulement par son chemin » (cohérent avec T1-1) ; soit l'ouverture
  devient un ordre du noyau qui rend l'identifiant, journalisé comme `layer.add` (écart assumé du
  ticket 14), et alors l'identifiant est rejouable.

### T5 — Asymétrie : QGIS → noyau n'est pas couvert

- **Unité A** — `raster.reclassify` dont la sortie est, d'après la note de l'entrée 3 de l'épopée
  10, une « sortie `layer` d'AD-24 » (identifiant de couche).
- **Unité B** — `terrain.slope` ou `terrain.viewshed`, qui exigent un chemin (`dem`, AD-20 : « QGIS
  ne passe qu'un chemin de fichier »).
- **Heurt.** « Découpe le MNT puis calcule la pente », ou « reclasse puis champ de vision », n'a
  aucune forme valide : la sortie déléguée n'expose qu'un identifiant, et l'amendement ne prévoit le
  chemin que dans le sens noyau → QGIS. Le fichier existe pourtant, nommé par le noyau (AD-10).
- **Fermeture proposée (AD-24, Genres) :** « Une sortie de genre raster expose toujours son fichier
  dans un champ `RasterPath`, qu'elle soit écrite par le noyau ou par QGIS ; tout consommateur de
  raster le reçoit par `step:k.<champ>`, journalisé en repère `out:k.n` (T2). »

### T6 — Les intermédiaires : deux politiques qui se contredisent

- **Unité A** — 5.34 : le noyau cache au modèle les sorties consommées dans un plan, reconnues par
  le `layer_id` qu'elles ont rendu.
- **Unité B** — l'entrée 3 : l'extension ouvre dans la carte toute sortie raster du noyau, y compris
  une étape intermédiaire de plan ; et AD-24 **Chaîne** : les intermédiaires d'une commande déléguée
  ne sont jamais nommés ni montrés.
- **Heurt.** Dans « pente puis reclassement », la pente consommée par chemin n'est pas reconnue
  intermédiaire (`intermediate_files` ne regarde que les `layer_id`) : elle reste dans l'inventaire
  envoyé au modèle **et** apparaît dans la carte, alors que l'intermédiaire équivalent d'une chaîne
  déléguée est invisible. Trois règles, trois réponses à « une sortie consommée est-elle montrée ? ».
- **Fermeture proposée (AD-11, complété 5.34) :** « Une sortie est intermédiaire quand une étape
  ultérieure du même plan la consomme, par identifiant **ou par repère de fichier** ; elle n'est ni
  montrée au modèle ni ouverte dans la carte. » L'extension apprend lesquelles ouvrir par le noyau
  (réponse de `plan.execute`), pas par son propre critère.

### T7 — Le relais d'IA externe ne compose pas de plan : la condition `step:k` y est vide

- **Unité A** — l'amendement, qui fonde la forme chemin sur une référence `step:k`.
- **Unité B** — AD-29 : l'IA externe appelle les commandes une à une (AD-23, hors règle) ; elle voit
  les chemins relatifs nettoyés et l'inventaire, et le relais refuse tout `..`.
- **Heurt.** Sur MCP il n'y a pas de `step:k` : l'IA externe, pour reclasser la pente qu'elle vient
  de calculer, écrit littéralement `projet.cassini/7-terrain.slope.tif`, ce que l'amendement
  interdit et que rien ne refuse (chemin relatif, dans le projet). Elle peut aussi passer
  l'identifiant `X` de T4. Et un MNT d'utilisateur hors du projet (`../donnees/mnt.tif`, admis par
  l'entrée 3) est désignable par l'agent local mais refusé au relais (`external.path_refused`).
- **Fermeture proposée (AD-29) :** une phrase qui dise ce que vaut la règle de provenance des
  chemins hors plan : soit « hors règle, comme les coordonnées » (risque accepté, écrit), soit
  « un chemin du dossier de travail donné par l'IA externe est converti en repère `out:k.n` par le
  noyau » (même mécanisme que T2).

## Constats secondaires

- **Nom de couche dérivé de chaque côté.** AD-10 : « l'extension n'invente jamais un nom de
  couche : elle reçoit celui que le noyau lui donne ». L'entrée 3 le **déduit** du radical du
  fichier. Même résultat aujourd'hui, mais la convention vit en deux endroits ; si le noyau change
  `output_file` (sorties multiples : `k-id-nom.tif`), l'extension doit suivre. Préciser que le nom
  est un champ rendu, ou que la règle « radical du fichier » est celle d'AD-10.
- **Fichiers tenus ouverts à l'insu du noyau.** AD-10 interdit au noyau de supprimer un fichier
  « que l'exécutant peut tenir ouvert » et lui fait demander de le libérer. Avec l'entrée 3,
  l'extension tient ouverts des fichiers du dossier de travail sans que le noyau ait émis d'ordre ;
  il ne peut pas savoir lesquels libérer. À noter dans AD-10 : « toute sortie raster du noyau est
  réputée tenue ouverte ».
- **Repli « nom seul » de l'inventaire.** Pour un raster sur un autre lecteur, l'inventaire rend
  `mnt.tif` (décision du propriétaire, entrée 3). Le noyau le résout en `<projet>/mnt.tif`, qui
  n'existe pas : erreur à l'exécution, après validation par l'utilisateur. Avec T1-1, mieux vaut
  que l'inventaire marque ce raster « non désignable » et que l'agent pose une question.
- **Rejeu et carte.** Le rejeu n'ouvre pas les sorties raster (`REPLAY_DONE` ignoré), alors que
  5.36 veut que « la carte du rejeu ne montre que ce que l'exécution d'origine montrait ». Écart
  mineur, à écrire plutôt qu'à corriger.
- **Résolution de chaîne par QGIS.** Si l'extension passe une chaîne au cadre de traitement, QGIS
  peut la résoudre par nom de couche (voir T1). Écrire dans AD-24 que le geste générique passe à
  l'algorithme une couche résolue explicitement par l'extension (objet couche, ou chemin vérifié),
  jamais une chaîne laissée à l'interprétation de QGIS.

## Ce qui tient

- La ligne rouge « aucune commande `raster.*` ne refait un calcul de `terrain.*` » est nette et
  vérifiable.
- L'exception d'AD-9 (chemins absolus sur le seul canal de l'extension, journal relatif, relais
  nettoyé) est cohérente avec le code de l'entrée 3 ; les trous ci-dessus viennent de ce que
  `step:k` transporte maintenant ces valeurs brutes vers le journal, pas de l'exception elle-même.
- Le `unknown` de l'entrée 25 avait vu la question ; la relecture montre que l'option retenue
  (« accepte aussi un chemin ») demande T1 à T3 pour être construite sans casser le rejeu.

## Ordre de fermeture suggéré

1. Trancher T1 (une forme par genre, ou deux champs exclusifs) — tout le reste en dépend.
2. T2 dans AD-11, avec son test de rejeu enchaîné, avant l'entrée 25 (et avant l'entrée 16).
3. T3 : type `RasterPath` marqué et contrôle de provenance, sur le patron de `Point`.
4. T4 à T7 dans le même amendement, en une phrase chacun.
