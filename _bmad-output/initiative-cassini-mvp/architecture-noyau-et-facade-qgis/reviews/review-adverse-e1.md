# Revue adverse — delta E1 du 2026-10-18 (AD-6 précisée, AD-15 écart, AD-24 à AD-26)

Lentille : construire deux tickets du niveau inférieur (E2 primitives, E3 préparation
de réseau, E4 géocodage) qui obéissent chacun à la lettre à toutes les AD et se
construisent pourtant incompatibles. Chaque paire est un trou ; chaque trou reçoit
une AD nouvelle ou resserrée, avec le texte proposé. Le contrat n'est pas modifié par
cette revue.

Matériau lu : le contrat (delta non commité), l'epic 10, `qgis-plugin/cassini_qgis/analysis.py`
(table `ALGORITHMS`), `core/src/commands/layer_add.rs`,
`core/src/commands/analysis_zonal_statistics.rs`, `core/src/journal/summary.rs`,
`core/src/plan.rs` (références `step:k.champ`).

## Verdict

Le delta ferme bien la question « comment ajouter une primitive » (une commande Rust
plus une ligne de table), mais laisse **sans propriétaire unique** quatre choses que
trois tickets construits séparément vont chacun décider : la forme des résumés, le
sens exact d'une commande déléguée dans le temps (ligne de table et version de QGIS),
le contrat d'un réseau, et la frontière de lecture du géocodage. Dix paires trouvées :
deux critiques, cinq hautes, trois moyennes ou basses.

| # | Gravité | Trou | Correctif |
|---|---|---|---|
| P1 | critique | Résumés chiffrés de formes libres ; `truncated` entre en collision avec le marqueur du journal | AD-27 nouvelle |
| P2 | critique | Le sens d'une commande déléguée a deux propriétaires (registre et ligne de table) et dépend de la version de QGIS, que le journal ignore | AD-24 et AD-11 resserrées |
| P3 | haute | AD-25 vérifie l'existence des algorithmes, pas leurs paramètres ni leur annulabilité, et pas le moment où les fournisseurs sont chargés | AD-25 resserrée |
| P4 | haute | AD-10 (« fichier intermédiaire » nommé par le noyau) contredit AD-24 (« intermédiaires temporaires ») | AD-10 et AD-24 accordées |
| P5 | haute | Raster désigné par `layer_id` ou par chemin ; algorithmes qui écrivent en place | AD-28 nouvelle, liste blanche resserrée |
| P6 | haute | E3 et les commandes de réseau lisent le même réseau différemment ; seuils restés en constantes dans l'extension | AD-29 nouvelle, AD-24 précisée |
| P7 | haute | Géocodage : qui lit le CSV, une ou deux commandes, forme du SCR, rejeu quand la BAN change, fichier passé par réglage | AD-26 resserrée |
| P8 | moyenne | La liste des méthodes du protocole n'est pas fermée ; « agir » ne couvre pas la lecture | AD-6 resserrée |
| P9 | moyenne | Sorties multiples (atlas, réseau et erreurs, chemin et points non atteints) | AD-24 précisée |
| P10 | basse | Garde contre branchement dans une chaîne fixe | AD-24 précisée |

---

## P1 — critique — deux primitives E2, deux formes de résumé, et une collision avec le journal

**Ticket A** (`analysis.count_points_in_polygon` élargi au comptage pondéré) rend
`{zones: [{label, count}], truncated}` — la forme actuelle de
`summarize_zonal_statistics`, avec un poids qui devient `count: f64`.
**Ticket B** (`analysis.nearest_neighbor`, matrice de distances) rend
`{pairs: [{from, to, distance}], truncated}` avec `distance` nue, dans l'unité de la couche.
**Ticket C** (`raster.zonal_statistics`) rend `{zones: [{label, mean, min, max}], truncated}`.

Les trois obéissent à AD-24 : « une liste bornée par le schéma, avec un drapeau
`truncated` explicite ». Ils se contredisent pourtant :

1. **Trois formes partagées pour une même idée.** L'agent (5.10) apprend trois manières
   de lire « une valeur par zone » ; le panneau en affiche trois ; une référence de plan
   `step:k.zones` (vérifiée par égalité stricte de schéma dans `plan.rs`) ne se branche
   jamais d'une primitive sur l'autre. Les distances nues de B violent la convention
   « Mesures », mais rien dans AD-24 ne rappelle qu'elle s'applique au résumé.
2. **L'étiquette n'a pas de règle.** Le code actuel retombe sur `feature.id()` quand le
   champ d'étiquette est vide. Un ticket garde cela, un autre prend le rang : le
   rejeu compare deux résumés dont les étiquettes ne veulent pas dire la même chose.
3. **La borne a deux propriétaires.** `MAX_ZONES` vit dans le Rust
   (`analysis_zonal_statistics.rs`, `maxItems` du schéma) et `_MAX_ZONES = 100` dans
   l'extension. Un ticket qui monte la borne d'un côté seulement produit un résultat
   plus long que le schéma ; à ma lecture, le noyau ne valide pas le résultat reçu
   contre `output_schema` (AD-6 ne parle que des paramètres), donc rien ne le voit.
   Et si un ticket applique « tout seuil est un paramètre » à la borne, le modèle la
   règle.
4. **Collision de nom avec le journal.** `journal/summary.rs` utilise un `truncated: true`
   de premier niveau comme marqueur de « résultat résumé par le journal » :
   `same_result` compare alors seulement `bytes` et `fingerprint`. Un résultat de
   primitive de moins de 200 octets qui porte légitimement `truncated: true` (borne
   petite, étiquettes courtes) est pris pour un résumé du journal ; ni `bytes` ni
   `fingerprint` n'y figurent, la comparaison rend **vrai quel que soit le contenu**.
   AD-24 impose précisément ce nom au premier niveau. Le rejeu devient aveugle sur
   ces étapes, sans erreur.

**Correctif — AD-27 nouvelle : « Un seul gabarit de résumé chiffré, possédé par le noyau ».**

> - **Binds:** CAP-14, CAP-6, CAP-7
> - **Prevents:** une forme de résumé par primitive ; une borne réglée de deux côtés ;
>   un résultat de primitive pris pour un résumé du journal.
> - **Rule:** un résumé chiffré rendu par une commande déléguée a une seule forme,
>   définie une fois en Rust dans le noyau et réemployée par chaque commande :
>   `summary: { rows: [{ label, values: { <nom>: { value, unit } } }], total, truncated }`,
>   où `label` est la valeur d'un champ désigné par paramètre ou, à défaut, le rang
>   (1, 2, 3…) dans l'ordre de la couche produite — jamais l'identifiant d'entité ;
>   toute valeur porte son unité (`count` pour un dénombrement), conformément à la
>   convention « Mesures » ; `total` est le nombre d'entités de la couche produite. La
>   borne est une **constante du noyau**, unique, exprimée par `maxItems` dans le schéma
>   engendré, et transmise dans l'ordre (`summary_limit`) rempli par le noyau, jamais
>   par le modèle ni par l'extension. Le noyau **valide le résultat reçu** contre le
>   schéma de sortie avant de le journaliser ; un résultat non conforme est l'erreur
>   `qgis.bad_result`, jamais un succès dégradé (AD-21). Les clés réservées au journal
>   (marqueur de résumé, taille, empreinte) vivent sous une clé propre au journal,
>   jamais au premier niveau d'un résultat de commande.

---

## P2 — critique — le sens d'une commande déléguée a deux propriétaires, et le temps le change

**Ticket A** (E2, première livraison) écrit la ligne `analysis.nearest_neighbor` sur
`native:joinbynearest` avec `MAX_DISTANCE` vide.
**Ticket B** (correctif ultérieur de l'extension) ajoute `native:multiparttosingleparts`
en tête de chaîne parce que les gares sont multiparties, ce qui change les décomptes.

Les deux respectent AD-24 (chaîne fixe, une ligne) et AD-6 (le type Rust n'a pas bougé,
la version du registre non plus). Or AD-17 livre l'extension et le noyau séparément :
le même journal, rejoué avec une extension plus récente, exécute une autre chaîne sous
le même `id` et la même version de registre. AD-11 journalise « les versions du noyau,
du protocole et du registre » — **ni la version de l'extension, ni celle de la ligne,
ni celle de QGIS**. Même sans ticket B, un passage de QGIS 3.44 à 4.x change le
résultat de certains algorithmes (paramètres renommés, valeurs par défaut,
comportement de GEOS) : le rejeu échoue en « résultat différent » sans pouvoir dire
pourquoi, ou réussit à tort quand la différence est dans les octets non comparés
(écart du ticket 14). Symétriquement, rien n'empêche un ticket de changer le sens
d'une commande côté Rust (description, défaut) sans toucher la ligne.

**Correctif — AD-24 resserrée (ajout) :**

> Le registre fixe le **contrat observable** d'une commande déléguée (sa description,
> ses paramètres, son résultat) ; la ligne de table n'en est que l'exécution. Chaque
> ligne porte un entier `chain_version`, incrémenté à toute modification qui peut
> changer un résultat (algorithme ajouté, retiré ou remplacé, valeur traduite
> différemment), et jamais réutilisé. `session.hello` annonce, pour chaque `id`, sa
> `chain_version`, ainsi que la version de QGIS hôte. Un changement du contrat
> observable côté noyau est une nouvelle version de registre ; un changement de
> chaîne qui le trahirait est un nouvel `id`.

**AD-11 resserrée (ajout à la liste de ce que porte chaque ligne) :**

> … et, pour une commande `runs_in: qgis`, la `chain_version` annoncée et la version de
> QGIS hôte. Avant de commencer, le rejeu signale toute étape dont la `chain_version` ou
> la version majeure de QGIS diffère de l'exécution d'origine, comme il signale un
> fichier non portable ; une différence de résultat sur une telle étape est rapportée
> comme due à ce changement, pas comme une divergence inexpliquée.

---

## P3 — haute — une commande annoncée qui ne peut pas tourner, ou pas s'annuler

**Ticket A** (E2) écrit `cancellable(true)` en Rust pour `raster.zonal_statistics`,
comme toutes les commandes actuelles. **Ticket B** (E2, la ligne) choisit
`native:zonalstatisticsfb`, ou une variante `gdal:*` / `grass:*` dont l'exécution
ignore le retour annulable. AD-12 exige que l'annulabilité soit déclarée au registre ;
mais la vérité dépend de la chaîne, qui ne vit que dans l'extension. Les deux tickets
sont conformes et la promesse est fausse.

Même structure pour la disponibilité :
- AD-25 vérifie que chaque **algorithme existe**. Elle ne vérifie pas que les **clés
  de paramètres** produites par la traduction existent : `native:serviceareafrompoint`
  a des clés dépréciées ou renommées entre séries (le code actuel écrit `TRAVEL_COST2`).
  Annoncée sur 4.x, la commande échoue à l'exécution — exactement ce qu'AD-25 voulait
  empêcher.
- AD-25 vérifie « à l'annonce ». Les fournisseurs de traitement (GRASS, extensions
  tierces) s'enregistrent au chargement de leurs extensions ; une extension Cassini
  qui envoie `session.hello` dès son `initGui` les voit absents et déclare à tort la
  commande indisponible pour toute la session. Un ticket envoie l'annonce tôt, un autre
  après `initializationCompleted` : deux comportements conformes, deux catalogues.

**Correctif — AD-25 resserrée (règle complète) :**

> À `session.hello`, l'extension n'annonce une commande que si, pour chaque étape de sa
> chaîne, l'algorithme existe dans le registre de traitement du QGIS hôte **et chaque
> clé de paramètre que la traduction produit figure dans ses définitions de
> paramètres** ; et, si le registre déclare la commande annulable, que chaque
> algorithme de la chaîne accepte l'annulation (drapeau d'annulation du cadre de
> traitement). Sinon la commande est indisponible (AD-7). La déclaration
> `cancellable` du registre est la promesse ; l'extension la vérifie, elle ne la
> corrige jamais. L'annonce n'est envoyée qu'après la fin de l'initialisation de QGIS
> (fournisseurs chargés) ; la disponibilité est figée pour la session, jamais
> réévaluée en cours de plan.

---

## P4 — haute — AD-10 et AD-24 disent deux choses des intermédiaires

AD-10 : « toute sortie — couche créée, PDF, **fichier intermédiaire** — est nommée par
le noyau dans un dossier de travail […] rien n'est écrit en dehors de ce dossier ».
AD-24 : « seule la dernière étape écrit la sortie nommée par le noyau, les
intermédiaires étant temporaires ».

**Ticket A** (E3, `network.prepare` : accrochage, découpe, nettoyage) lit AD-10 et
demande au noyau trois chemins intermédiaires dans le dossier de travail, journalisés
comme sorties 1 à 3 de l'étape. **Ticket B** (E2, `raster.zonal_statistics` sur une
chaîne `gdal:*`) lit AD-24 et passe `TEMPORARY_OUTPUT` : QGIS écrit alors un fichier
dans **son propre** dossier temporaire, hors du dossier de travail — AD-10 enfreinte à
la lettre, AD-24 respectée à la lettre. À l'annulation (AD-12), A sait nettoyer ses
intermédiaires (par l'extension, AD-10), B ne sait même pas qu'ils existent. Et A
expose au journal des repères `out:k.2` que les étapes suivantes peuvent citer, alors
que B n'en expose aucun : deux modèles d'intermédiaires pour une même table.

**Correctif — AD-24 précisée et AD-10 accordée.**

AD-24, remplacer la phrase sur les intermédiaires par :

> Les étapes intermédiaires d'une chaîne écrivent en `TEMPORARY_OUTPUT` du cadre de
> traitement, et nulle part ailleurs : elles ne sont ni nommées par le noyau, ni
> journalisées, ni citables par une autre étape, et leur cycle de vie appartient à
> QGIS. Seule la dernière étape écrit les sorties nommées par le noyau.

AD-10, préciser :

> « Toute sortie » s'entend des sorties **déclarées** d'une commande. Les fichiers
> temporaires que le cadre de traitement de QGIS gère lui-même pour une étape
> intermédiaire (AD-24) ne sont pas des sorties Cassini ; Cassini n'en écrit aucun
> lui-même hors du dossier de travail.

---

## P5 — haute — un raster désigné de deux façons, et un algorithme qui écrit dans l'original

**Ticket A** (E2, `raster.zonal_statistics`) suit AD-9 : le raster est une couche du
projet, `raster_layer_id`. **Ticket B** (terrain, CAP-11) suit AD-20 : le noyau lit le
modèle d'élévation **par chemin**, et `terrain.slope` écrit un GeoTIFF dans le dossier de
travail, sans `layer_id`. La phrase naturelle « pente moyenne par commune » enchaîne
B puis A : la sortie de B est un chemin, l'entrée de A un `layer_id`, et `plan.rs`
refuse la référence (`plan.type_mismatch`). Pour la combler il faudrait `layer.add` —
mais `layer.add` est déclaré « couche vectorielle » (`layer_add.rs`), et le geste
générique refuse toute couche non vecteur (`ProcessingGesture._begin`,
`layer.not_vector`) et recharge la sortie en `ogr`. Un troisième ticket peut trancher
en prenant un `raster_path` dans `raster.zonal_statistics` : conforme aussi, et voilà
deux conventions de désignation des rasters.

Second trou, sur la même commande : QGIS offre `native:zonalstatisticsfb` (couche
neuve) et `qgis:zonalstatistics`, qui **ajoute les champs à la couche de polygones
d'entrée**, donc au fichier de l'utilisateur. La liste blanche d'AD-24 n'interdit que
le réseau et le géocodeur : une ligne sur l'algorithme en place est conforme à AD-24 et
viole AD-10 et le principe 6 de CLAUDE.md. Même risque pour tout algorithme
« modifie la couche » (ajout de champ en place, édition en place).

**Correctif — AD-28 nouvelle : « Les rasters délégués se désignent comme les autres couches ».**

> - **Binds:** CAP-14, CAP-9, CAP-11
> - **Prevents:** deux désignations d'un raster ; une sortie du terrain que les
>   commandes `raster.*` ne savent pas consommer.
> - **Rule:** une commande `raster.*` désigne son raster par `layer_id` (AD-9), comme
>   toute couche du projet. Une sortie raster écrite par le noyau (terrain) n'entre dans
>   le projet que par une étape `layer.add` explicite du plan, que `layer.add` accepte
>   donc pour un GeoTIFF ; le geste générique distingue vecteur et raster par une clé
>   de la ligne, jamais par une décision au vol. Le terrain continue de lire par chemin
>   (AD-20) : un `layer_id` raster ne traverse jamais vers une commande `terrain.*`.

**AD-24, liste blanche resserrée (ajout) :**

> … ni un algorithme qui écrit dans l'une de ses entrées (statistiques ajoutées à la
> couche d'entrée, édition en place) : toute primitive produit une couche neuve à
> l'emplacement nommé par le noyau, et les couches d'entrée sont ouvertes en lecture.

---

## P6 — haute — E3 fabrique un réseau que deux commandes lisent différemment

Aujourd'hui `analysis.isochrone` fige, dans l'extension : deux sens de circulation,
une vitesse par mode (`_SPEEDS_KMH`), `TOLERANCE: 0`, et un accrochage de l'origine à
100 m (`_ORIGIN_SNAP_METERS`), vérifié avant de lancer.

**Ticket A** (E3, `network.prepare`) accroche les extrémités à une tolérance passée en
paramètre, découpe aux croisements, et — pour bien faire — ajoute un champ `oneway`
repris des données.
**Ticket B** (E2, `network.shortest_path`) passe `TOLERANCE` = paramètre (pour combler
les trous du réseau), lit `DIRECTION_FIELD` s'il existe, et accroche l'origine à 50 m.

Tous deux conformes. Résultat : sur le réseau produit par A, l'isochrone ignore les
sens uniques et ne comble aucun trou, le plus court chemin respecte les sens et comble
les trous de 2 m : « à 10 min à pied » et « chemin le plus court » se contredisent sur
la même carte. La notion de réseau n'a pas de propriétaire.

En outre, AD-24 dit « tout seuil est un paramètre, jamais une constante », et le code
actuel garde trois seuils en constantes de l'extension (vitesse, accrochage, segments
du tampon). Un ticket E2 les migre vers les paramètres, un autre ne touche pas
l'existant : deux régimes dans la même table, et la valeur par défaut d'un seuil vit
soit dans le schéma Rust, soit dans la ligne.

**Correctif — AD-29 nouvelle : « Un seul contrat de réseau ».**

> - **Binds:** CAP-14, CAP-3
> - **Prevents:** un réseau lu de deux façons par deux commandes ; une préparation de
>   réseau qui produit ce qu'aucune commande ne lit.
> - **Rule:** un réseau est une couche de lignes, dans un SCR en mètres, dont les
>   tronçons se rejoignent par leurs extrémités aux croisements. Toutes les commandes
>   qui le lisent (`analysis.isochrone`, `network.*`) le parcourent de la même façon :
>   mêmes règles de sens (deux sens tant qu'aucun paramètre neutre ne désigne un champ
>   de sens), même tolérance d'algorithme (zéro : combler les trous est le travail de
>   `network.prepare`, pas des commandes de calcul), même accrochage d'origine, exprimé
>   par un paramètre de distance avec unité dont le défaut est unique et défini dans le
>   noyau. `network.prepare` produit exactement cette forme et n'ajoute aucun champ
>   qu'une autre commande lirait implicitement.

**AD-24, précision sur les seuils :**

> Un seuil (distance, durée, vitesse, tolérance, surface minimale) est un paramètre du
> registre ; sa valeur par défaut, s'il en a une, est déclarée dans le type Rust et donc
> dans le schéma engendré, jamais dans la ligne de table. La ligne ne garde en constante
> que des réglages de forme sans effet métier (nombre de segments d'un arc), énumérés
> dans un commentaire de la ligne. L'existant (`analysis.isochrone`, `analysis.buffer`)
> s'y conforme dans le premier ticket E2 qui touche la table.

---

## P7 — haute — le géocodage : qui lit, combien de commandes, quelle forme, quel rejeu

AD-26 dit : le noyau lit deux CSV, apparie, écrit un CSV à coordonnées qui « déclare
son code EPSG », puis « `layer.add` l'ouvre ensuite par QGIS ». Cinq paires conformes
et incompatibles :

1. **Fichier ou couche.** Ticket A prend `addresses_path` (relatif au projet). Ticket B
   remarque que l'utilisateur a déjà ouvert son CSV d'adresses dans QGIS et prend
   `addresses_layer_id` ; pour que le noyau le lise, il demande à l'extension le
   chemin source de la couche — un chemin absolu au journal (AD-9) ou un geste
   d'export (une écriture de plus). Et même avec A, **deux lecteurs CSV** lisent le
   même fichier : QGIS (détection de séparateur, encodage, en-têtes) et la crate du
   noyau. Le panneau montre 1 000 adresses, le noyau en lit 980 parce que le
   séparateur est `;` ou l'encodage Latin-1. CAP-9 confie la lecture des formats à
   QGIS ; AD-26 crée une seconde lecture sans fixer ses règles.
2. **Une ou deux commandes.** Ticket A rend un chemin et laisse l'agent planifier
   `layer.add` (deux étapes, deux lignes de journal). Ticket B fait ajouter la couche
   par le noyau dans la foulée, en émettant lui-même l'ordre `layer.add` : une étape du
   plan, deux écritures de journal, et une commande `runs_in: core` qui délègue, ce
   qu'AD-6 (« unique ») interdit en esprit sans l'interdire en lettre, puisque c'est la
   boucle du noyau qui émet.
3. **Forme du SCR.** Un CSV ne peut pas « déclarer » son code EPSG : aucune convention ne
   le porte. `layer.add` exige `crs` pour un CSV, sous la forme `"EPSG:2154"` (chaîne),
   avec `x_field`/`y_field` (défauts `X`/`Y`) ; l'origine d'isochrone porte `epsg`
   entier. Ticket A rend `epsg: 2154`, ticket B `crs: "EPSG:2154"` : la référence
   `step:k.crs` ne se branche que sur l'un des deux. La BAN porte à la fois
   `lon`/`lat` (WGS84) et `x`/`y` (Lambert-93 en métropole, d'autres projections outre-mer) :
   lequel écrire n'est fixé nulle part.
4. **Rejeu quand la BAN change.** AD-11 garde taille et date des fichiers d'entrée, mais
   ne dit pas ce que fait le rejeu quand elles diffèrent. Le millésime de la BAN change
   tous les mois : un ticket échoue, un autre rejoue et rapporte une divergence de taux
   d'appariement. Un index de la BAN mis en cache (un fichier départemental fait
   plusieurs centaines de milliers de lignes) ajoute un troisième état, périmé sans
   que rien ne le détecte.
5. **Fichier par réglage.** La convention « Réglages » admet des variables
   d'environnement au noyau (`CASSINI_MODELS`). Un ticket désigne la BAN par
   `CASSINI_BAN`, un autre par paramètre : avec le premier, le journal ne connaît pas le
   fichier lu et le rejeu sur un autre poste lit autre chose sans le savoir.

S'y ajoute : les adresses non appariées. Un ticket les liste dans le résultat (non
borné, contraire à AD-8), un autre les écrit dans une colonne du CSV.

**Correctif — AD-26 resserrée (règle complète) :**

> Une commande `geocode.*` est `runs_in: core` et ne délègue rien. Ses deux entrées sont
> des **chemins** de fichiers CSV, paramètres du registre relatifs au projet (AD-9),
> journalisés avec taille et date ; jamais un `layer_id`, jamais un réglage ni une
> variable d'environnement. Le noyau les lit selon une règle unique et documentée dans
> la description de la commande : UTF-8 (BOM toléré), séparateur `;` ou `,` désigné par
> paramètre ou détecté sur la seule ligne d'en-tête, et toute ligne illisible comptée
> et nommée, jamais ignorée (AD-21). Il écrit une sortie unique, un CSV UTF-8 à
> séparateur `,` dans le dossier de travail (AD-10), avec les colonnes `X` et `Y`
> reprises **telles quelles** d'une seule paire de colonnes de la BAN désignée par
> paramètre (`lonlat` → `EPSG:4326`, `xy` → le SCR que l'utilisateur déclare), une
> colonne de statut d'appariement par ligne, et les colonnes d'origine. Son résultat
> porte `path`, `crs` sous la forme de chaîne de `layer.add` (`"EPSG:2154"`), et un
> résumé selon AD-27 (appariées, ambiguës, non appariées). Ouvrir la couche est une
> étape `layer.add` **distincte et explicite** du plan, qui cite `step:k.path` et
> `step:k.crs`. Au rejeu, une BAN dont la taille ou la date diffère est signalée avant
> de commencer ; un résumé différent est alors rapporté comme dû à ce changement. Un
> index éventuel de la BAN est un cache du dossier de travail, clé par chemin, taille
> et date, reconstruit dès que l'une change, et jamais journalisé.

**Convention « Familles de commandes » ou « Mesures », ajout :**

> Un SCR qui traverse le canal comme valeur est toujours la chaîne `"EPSG:<code>"` ;
> la forme `{x, y, epsg}` reste réservée aux coordonnées ponctuelles.

---

## P8 — moyenne — une méthode du canal pour contourner le journal

La précision d'AD-6 énumère `session.*`, `plan.*`, `order.execute`, `journal.replay`,
`request.cancel`, `agent.models`, puis pose que « toute action sur le projet ou sur
les données est une commande ». Deux brèches conformes :

- **Ticket A** (E4) ajoute `geocode.preview`, méthode du canal qui lit les dix premières
  lignes du CSV pour que le panneau propose le séparateur. Elle n'« agit » pas : elle
  lit. Elle lit pourtant un fichier de l'utilisateur hors registre et hors journal.
- **Ticket B** (E4) ajoute `geocode.index`, méthode qui construit l'index de la BAN « en
  avance ». Elle écrit dans le dossier de travail sans ligne de journal.

`agent.models` a ouvert le précédent (écart d'AD-15) ; la liste n'est pas dite fermée,
et `session.*` / `plan.*` sont des familles ouvertes.

**Correctif — AD-6, précision resserrée :**

> Les méthodes du protocole forment une **liste fermée**, énumérée ici et nulle part
> ailleurs ; une famille (`session.*`, `plan.*`) n'est pas une autorisation d'y ajouter.
> Aucune méthode ne lit ni n'écrit un fichier de l'utilisateur ou du dossier de travail,
> à la seule exception de `journal.replay`, qui lit le journal. Ajouter une méthode
> amende cette précision et incrémente la version du protocole (AD-7) ; tout ce qui
> lit des données pour l'utilisateur, même pour un aperçu, est une commande.

---

## P9 — moyenne — combien de sorties a une primitive

Le noyau remplit un seul paramètre de sortie (`output_params(&["out"])`), AD-24 parle de
« la sortie nommée », AD-11 de « sortie *n* de l'étape *k* ».

- **Ticket A** (`layout.atlas`) écrit un PDF par entité : le noyau ne peut pas nommer à
  l'avance un nombre de fichiers qu'il ignore (AD-10), et le journal ne sait pas les
  désigner.
- **Ticket B** (`network.prepare`) rend le réseau **et** une couche d'erreurs de
  topologie, comme le font les algorithmes de nettoyage ; un ticket C
  (`network.shortest_path`) rend le chemin **et** les points non atteints.

Conformes tous trois, et le mécanisme de nommage ne les porte pas.

**Correctif — AD-24, précision :**

> Le nombre de sorties d'une commande est fixe et déclaré au registre ; le noyau en
> nomme chacune (AD-10), et le journal les repère `out:k.1`, `out:k.2`… Une primitive
> dont l'algorithme produirait un nombre variable de fichiers est réduite à une sortie
> unique (l'atlas est un seul PDF de plusieurs pages) ; une sortie secondaire que la
> chaîne ne déclare pas reste temporaire (P4).

---

## P10 — basse — garde ou branchement

AD-24 : « la chaîne ne comporte ni branchement ni choix ». La ligne `analysis.isochrone`
contient déjà une garde (`empty_error` : la première étape n'a rien produit, on échoue
avec `analysis.origin_off_network`) et un contrôle d'origine avant lancement. Un ticket
lit « ni branchement » comme interdisant ces gardes et les retire ; un autre y voit
une autorisation de faire, sur sortie vide, « une autre étape à la place ».

**Correctif — AD-24, précision :**

> Une ligne peut porter des **gardes** : un contrôle qui, s'il échoue, arrête la chaîne
> avec une erreur nommée (AD-21). Elle ne porte jamais d'alternative : aucune condition
> ne choisit entre deux suites d'algorithmes, ni ne modifie une valeur de paramètre.

---

## Ce qui tient

- AD-22 et AD-23 suffisent à empêcher une commande passe-partout : la paire « ticket qui
  expose `processing.run` générique » n'est pas constructible sans enfreindre AD-24 à la
  lettre.
- AD-1 et AD-16 tiennent pour E4 tant qu'il reste un lecteur CSV permissif ; la lecture de
  CSV n'introduit ni GDAL ni PROJ.
- L'écart d'AD-15 (`agent.models` hors registre) est correctement motivé ; le risque qu'il
  ouvre est traité en P8, pas par son retrait.
