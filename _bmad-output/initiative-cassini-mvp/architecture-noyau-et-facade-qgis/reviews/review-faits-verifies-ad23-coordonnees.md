# Relecture — AD-23, sous-règle « Coordonnées (2026-10-23) »

Lentille : chaque décision engagée par l'amendement est-elle vérifiée contre le dépôt, ou seulement affirmée ?
Objet : la sous-règle « Coordonnées (2026-10-23) » d'AD-23 et l'ajout à sa clause *Prevents*. Rien d'autre n'a changé dans le contrat.
Date : 2026-10-23. Le contrat n'a pas été modifié.

## Verdict

L'intention tient et ne contredit ni AD-5, ni AD-19, ni AD-21, ni AD-22. En revanche, deux de ses affirmations sont fausses dans l'état actuel du code. La première dit qu'un point se reconnaît à son type. La seconde dit que le déclenchement est journalisé, ce qui heurte AD-11. Une troisième affirmation ne précise pas où vit le contrôle, et cet endroit décide de ce que les mesures verront. Il faut retoucher le texte avant le build de 5.16.

## Constats

### 1. HAUT — « un paramètre point se reconnaît à son type dans le schéma engendré (AD-6) » : faux aujourd'hui

Ce qu'on trouve dans le code :
- La seule commande qui prend une coordonnée est `analysis.isochrone`. Son paramètre `origin` est une struct `Origin { x: f64, y: f64, epsg: u32 }`, privée au fichier `core/src/commands/analysis_isochrone.rs`. Aucune autre commande ne prend de point : `buffer`, `intersect`, `select_by_attribute` et `zonal_statistics` ne portent que des `crs: String`, et seulement dans leurs résultats.
- Le noyau n'a aucun type « point » partagé. `schemars` émet ce paramètre comme un `$ref` vers `#/$defs/Origin`. La seule étiquette qui distingue ce type dans le schéma est donc la clé `Origin`, c'est-à-dire un **nom**, ce que la règle interdit justement.
- Le précédent du dépôt reconnaît d'ailleurs par le nom : `contract.rs::is_geometry` cherche « geometry » ou « geojson » dans le `$ref` ou le `title`, puis regarde la forme des propriétés.

Il n'existe donc rien aujourd'hui qui permette de reconnaître un point « à son type ». 5.16 devrait l'introduire.

**Correctif proposé :** « Une coordonnée du registre a un seul type, le `Point` partagé du noyau (x, y, epsg) ; le contrôle de contrat (AD-6) refuse toute autre forme de coordonnée, et le garde-fou reconnaît un paramètre point à ce type, jamais au nom du paramètre. » Le ticket 5.16 crée ce type et y fait passer `Origin`.

### 2. HAUT — « son déclenchement est journalisé » : contredit AD-11

Ce qu'on trouve dans le code :
- AD-11 prévoit « une ligne JSON par étape **exécutée** ». Dans `core/src/journal/mod.rs`, seule l'exécution d'une étape écrit dans le journal, et le rejeu relit toutes les lignes non vides.
- La composition n'est jamais journalisée. `serve/plans.rs::propose` le dit en toutes lettres : « l'inventaire n'est pas journalisé ». Une question n'a ni `plan_id` ni ligne de journal.
- La crate `ai/` n'a aucun accès au journal.
- Ce que vise le ticket, c'est que les **mesures** sachent d'où vient une question. Or `plan_reel`, `question_reel`, `ecart_reel` et `duree_reel` appellent `compose_plan_with` directement, sans session et sans journal. Une ligne de journal ne leur servirait à rien.

**Correctif proposé :** « …et la question qu'il rend se distingue, dans le résultat de la composition, d'une question du modèle ; le journal de AD-11 n'en porte rien. » L'autre voie serait d'amender AD-11 explicitement, avec un type de ligne que le rejeu ignore.

### 3. MOYEN — l'enchaînement est possible, mais l'endroit du contrôle n'est pas fixé

Ce qu'on trouve dans le code :
- `compose_plan_with` fait au plus deux tentatives : `judge`, puis `retry_turns`, puis `judge` à nouveau. Elle rend ensuite `Ok(plan)`.
- Un contrôle placé juste après ce `Ok(plan)`, qui transforme un `Plan::Calls` en `Plan::Question`, ne touche ni au prompt, ni à `judge`, ni au tour de redemande. L'enchaînement décrit par la règle est donc réalisable.
- La forme de question de 5.5, `{"kind":"question","question":…}`, convient telle quelle.
- En revanche, si le contrôle est placé dans `serve/plans.rs`, la CLI et les mesures y échappent.

**Correctif proposé :** « Le contrôle vit dans la composition même, après le jugement et l'unique redemande, avant que le plan ne soit rendu ; tout chemin qui compose (canal, CLI, mesures) le traverse. »

### 4. MOYEN — l'origine du système de coordonnées n'est pas tranchée, et la branche « étape précédente » est théorique

Ce qu'on trouve :
- La phrase E inventait aussi l'EPSG (4326). Le texte demande que la coordonnée « figure dans la demande », mais ne dit pas si l'`epsg` est concerné.
- Une valeur qui « vient du résultat d'une étape précédente » ne peut s'écrire aujourd'hui que comme une référence de premier niveau, `step:k[.champ]` (`core/src/plan.rs`). Or aucun résultat de commande ne porte de point à ce jour.

**Correctif proposé :** « x et y figurent dans la demande, et le système aussi, sinon question (AD-19) ; à défaut, le paramètre entier est une référence `step:` ». Si le propriétaire tranche autrement pour le système, le texte doit le dire explicitement.

### 5. BAS — l'exclusion du serveur MCP est juste, mais pour une raison mal formulée

Ce qu'on trouve :
- L'exclusion est cohérente avec AD-13 et avec l'entrée 5.7 : `mcp` expose les commandes une à une, et c'est le client externe qui compose, pas le noyau.
- La vraie raison n'est donc pas l'absence de la phrase de l'utilisateur, mais l'absence de toute composition côté noyau.
- Le risque résiduel, à savoir une IA externe qui inventerait elle aussi un point, n'est pas dit.

**Correctif proposé :** « le serveur MCP non plus : le noyau n'y compose aucun plan (AD-13) ; un point inventé par une IA externe reste un risque accepté. »

## Autres décisions vérifiées, sans contradiction

- **AD-5** : le contrôle vit dans le noyau, et l'extension n'a rien à décider. Pas de contradiction.
- **AD-19** : la question demande le système, ce qui va dans le sens de la règle « toute coordonnée porte son EPSG ».
- **AD-21** : la sortie est une question explicite, pas un succès dégradé. Pas d'échec silencieux.
- **AD-22** : aucun vocabulaire QGIS n'est introduit.
- **AD-6, précision** : la liste fermée des méthodes du protocole n'est pas touchée.
- **Rejeu** : il ne compose rien. L'exclure est exact.
- **Remarque** : la section « Never » de 5.5 écartait toute comparaison de la demande côté Rust. Ce garde-fou en est une, assumée par la décision du 2026-10-23. Le plan de 5.16 doit le mentionner.
