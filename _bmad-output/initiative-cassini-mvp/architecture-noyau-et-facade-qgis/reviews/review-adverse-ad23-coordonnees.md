# Relecture adverse — AD-23, sous-règle « Coordonnées (2026-10-23) »

Date : 2026-10-23. Objet : l'amendement d'AD-23 (sous-règle *Coordonnées* et clause
*Prevents* ajoutée), préalable au ticket 5.16 (`epic-agent-et-mcp/tickets.toml`, id 16).
Lentille : construire deux unités qui obéissent à toutes les AD et s'assemblent mal ;
vérifier que la règle s'exprime dans le code existant.

## Verdict

Le fond est juste (refuser un point que le modèle a inventé, rendre une question, ne toucher
ni au prompt, ni à `judge`, ni à `retry_turns`), mais quatre phrases de la règle ne
s'appliquent pas telles quelles au code actuel : comment on reconnaît un point, ce qui compte
comme « résultat d'une étape », ce que veut dire « journalisé », et ce que veut dire
« figure dans la demande ». À corriger avant de construire 5.16.

## Ce que le code montre (état existant)

- `core/src/commands/analysis_isochrone.rs` : l'origine est une struct **locale à la
  commande**, `Origin { x: f64, y: f64, epsg: u32 }`, que `schema_for!` engendre
  (`core/src/registry.rs:129`). Il n'existe **aucun type point partagé** dans `core/`. Les
  autres structures qui contiennent des coordonnées sont des étendues :
  `InventoryExtent` et `DescribedExtent` (xmin, ymin, xmax, ymax), des types de sortie.
- `core/src/plan.rs` : `Plan::Question { question: String }`, sans aucun autre champ. Les
  références se limitent au **premier niveau** (`step:k` ou `step:k.champ`). `check_plan` ne
  valide une étape qui porte une référence qu'au moment de l'exécution.
- `ai/src/plan.rs` : `compose_plan_with` déroule `judge`, puis au besoin un seul
  `retry_turns`, et rend `Ok(Plan)`. Tous les chemins de composition y passent : le
  `Composer` de la CLI (`cli/src/main.rs:312`, utilisé par `plan.propose`), la commande
  `cassini plan`, et les mesures réelles (`ai/tests/*_reel.rs`).
- `core/src/serve/plans.rs::propose` : la question est rendue sous la forme
  `{"question"}`, **sans `plan_id` et hors journal** (« jamais gardée »). Le journal
  (AD-11, `Session::call`) n'écrit qu'une ligne par **étape exécutée**.
- Panneau (`agentview.py:75`) : une question est une réplique ; la réponse est une **nouvelle
  demande**, et la phrase d'origine n'est pas gardée.
- Générateur (`training/src/families.rs` ≈1407-1437, 2104-2170) : il écrit les points sous
  la forme `652 000`, `2.4580`, « X = …, Y = … », avec les systèmes « Lambert 93 »,
  « EPSG:2154 » et « WGS 84 » ; il apprend déjà au modèle à poser une question quand le lieu
  est nommé sans coordonnées. Le point inventé pour la phrase E (2,458 / 48,658) tombe
  exactement dans la plage EPSG:4326 du générateur (2,2 à 2,6 ; 48,6 à 48,9).

## Constats

### C1 — élevé — « Reconnu à son type dans le schéma engendré » : ce type n'existe pas

Deux unités qui respectent AD-6 et AD-19 à la lettre :
- 5.16 reconnaît un point à sa forme engendrée `{x, y, epsg}` (le nom `$defs/Origin` est
  exclu, puisque la règle interdit de reconnaître un point à son nom) ;
- un futur `terrain.viewshed` déclare `Observer { x, y, epsg, height }`, et un futur
  `terrain.line_of_sight` déclare `from` et `to`, ou bien un `Vec<…>` de points.

Les deux respectent AD-19 (chaque coordonnée porte son EPSG) et AD-6 (schéma engendré). Mais
la forme de `Observer` n'est pas exactement `{x, y, epsg}` : le garde-fou ne la voit pas, et
le modèle peut à nouveau inventer un point sans que rien ne le signale. Les étendues
(xmin…ymax) d'une future commande de mise en page ou d'atlas sont des coordonnées que la
règle ne voit pas non plus, alors que la première phrase dit « *une* coordonnée ».

**Correctif** (texte à ajouter, ici ou dans AD-19) : « Toute coordonnée d'un paramètre de
commande est portée par l'unique type `Point` du noyau (x, y, epsg). Une commande le réutilise
par composition (un observateur est `{ point: Point, height }`), sans jamais le recopier. Son
schéma engendré porte une marque fixe (une extension `schemars`, par exemple
`x-cassini-kind: point`), et c'est cette marque que suit le garde-fou, à travers les `$ref`,
les objets imbriqués et les tableaux. La recette refuse un paramètre de commande qui
contient un champ `x` ou `y` de type nombre ailleurs que dans `Point`. » Les étendues
utilisées comme paramètres sont soit marquées de la même façon (`Extent`), soit déclarées
hors du champ de la règle, en le disant explicitement. `analysis.isochrone` passe à `Point`
dans 5.16 ; la forme JSON de ses paramètres ne change pas.

### C2 — élevé — « Son déclenchement est journalisé » contredit AD-11 et le code

Le journal d'AD-11 a un seul auteur, n'accepte que des ajouts, compte une ligne par étape
exécutée, et c'est lui que le rejeu relit. Or une question n'est jamais gardée et n'a pas de
`plan_id` (`plans.rs`). Dès lors :
- soit l'auteur de 5.16 écrit une ligne de déclenchement au journal, et le rejeu (qui doit
  « échouer en la nommant » face à ce qu'il ne sait pas faire) tombe sur une ligne qui n'est
  pas une étape ;
- soit il n'écrit rien au journal, et la règle n'est pas tenue.

De plus, ce besoin vient des **mesures** : le ticket veut savoir si une question vient du
modèle ou du garde-fou. Or `question_reel` et `plan_reel` appellent `compose_plan` sans
session et sans journal : ils ne liraient jamais une ligne de journal.

**Correctif** : « Le déclenchement est marqué dans le résultat de la composition : la question
porte une provenance (`model` ou `guard`), que les mesures comptent et que `plan.propose`
peut transmettre. Rien n'est écrit au journal d'AD-11, qui ne porte que des étapes
exécutées. » Le champ de provenance n'entre pas dans la grammaire du modèle (valeur par
défaut `model`), ce qui garde la parité au jeton avec l'entraînement.

### C3 — élevé — « Vient du résultat d'une étape précédente » ne se vérifie pas avant l'exécution

Il n'existe qu'une seule façon pour un paramètre de venir d'une étape : une référence de
premier niveau, `origin: "step:k[.champ]"`, que `check_plan` ne valide qu'à l'exécution. Au
moment de montrer le plan, le garde-fou voit une chaîne de caractères. Pour l'adversaire, un
plan avec `"origin": "step:1.feature_count"` « vient d'une étape » : il passe le garde-fou,
puis échoue plus tard. Et aujourd'hui, aucune commande ne rend de point. Le géocodage
(AD-26, 10.16) rend un CSV, `crs`, `x_field` et `y_field`, c'est-à-dire des noms de colonnes,
pas un point. L'enchaînement « gare → isochrone » que cette clause semble promettre est donc
impossible, alors que la règle laisse croire qu'il marche.

**Correctif** : « Une référence `step:k[.champ]` n'est acceptée à la place d'un point que si le
schéma de sortie engendré de l'étape k désigne, à cet endroit, un `Point` (C1). Sinon, le
garde-fou rend la question. Tant qu'aucune commande ne rend de `Point`, seule la demande de
l'utilisateur fournit un point. » Ajouter aussi : « une coordonnée lue dans l'inventaire
(étendue, centre de couche) n'est pas une provenance admise ».

### C4 — moyen — « Figure dans la demande » se laisse contourner, et le système n'est pas tenu

- Les nombres sont pris un par un : « à 500 m … en 10 minutes » laisse passer
  `origin {x: 500, y: 10}`.
- Le système : l'utilisateur écrit « 652000 6850000 » sans système, et le modèle écrit
  `epsg: 4326`. Le texte n'oblige pas à vérifier l'EPSG, alors que la question demande
  « x, y et système ».
- Les écritures : le générateur produit `652 000`, `2.4580` et `2,458`, que ne retrouve pas
  une comparaison de textes. Si le garde-fou ne les reconnaît pas, `plan_reel` baisse (ce
  que le critère « verify » de 5.16 interdit).

**Correctif** : « Le x et le y du point figurent tous les deux dans la demande, comme valeurs
numériques (on compare des nombres, pas des textes), et ce sont deux nombres distincts. Le
système figure aussi dans la demande (un code EPSG, ou un nom que le noyau associe à un code
fermé : Lambert-93, WGS84) ; sinon, le garde-fou rend la question. » Les règles de lecture des
nombres restent au ticket, mais un test fait passer chaque exemple du générateur qui contient
un point par le garde-fou, et le plan doit en sortir inchangé.

### C5 — moyen — « Le noyau » : où le contrôle s'applique

Il existe trois chemins de composition. Si 5.16 place le garde-fou dans
`core/src/serve/plans.rs::propose` (le seul endroit de `core` qui reçoit `request`), la
commande `cassini plan` et les mesures réelles le contournent : ce que mesurent les tests
n'est plus ce que voit le panneau. S'il le place dans `ai`, le module privé, qui prend `core`
sans `ai`, ne l'a pas.

**Correctif** : « Le contrôle est une fonction pure de `core::plan` (plan, demande, registre),
appliquée une seule fois, à la fin de `compose_plan_with`, que tous les chemins de
composition empruntent. »

### C6 — faible — La réponse à la question perd la phrase d'origine

Une question est sans état : la réponse de l'utilisateur devient une nouvelle demande (5.5,
`agentview.py`). Un utilisateur qui ne répond que « 652000 6850000 Lambert-93 » perd « 10 min
à pied », et le modèle n'a plus rien à planifier.

**Correctif** : la question du garde-fou demande de **reformuler la demande avec le point**.
Une autre voie : le panneau remet dans la zone de saisie la phrase d'origine. Il faut écrire
ce choix dans la règle ou dans 5.16, sans changer la forme de la question de 5.5.

## Points sans objection

- Le rejeu est exclu à juste titre : le journal garde les valeurs résolues (`plans.rs`,
  en-tête).
- Le serveur MCP est exclu, et c'est cohérent : le noyau n'y voit pas la phrase. Il faudrait
  quand même noter qu'une IA externe peut inventer un point sans qu'aucune alerte ne se
  déclenche.
- La règle s'accorde avec AD-19 côté terrain. Un point donné en WGS84 en réponse à la
  question sera refusé par la commande de terrain s'il n'est pas dans le système du modèle
  d'élévation. C'est voulu, mais le message doit le dire.
- Le contrôle ne touche ni au prompt, ni à `judge`, ni à `retry_turns`. Le code l'exprime
  sans peine, en filtrant le `Ok(plan)` que rend `compose_plan_with`.
