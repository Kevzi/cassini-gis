# Canal local : ordres du noyau, annulation et rejeu

Le canal de `cassini serve` est du JSON-RPC 2.0, un message par ligne, sur
l'entrée et la sortie standard, **dans les deux sens** (AD-3). Le protocole reste
à la version 1. Ce document fixe les messages que l'extension QGIS consomme.

Un fil du noyau lit l'entrée ; la boucle principale est seule à écrire. Une seule
étape s'exécute à la fois (AD-5).

## `order.execute` (noyau vers extension)

Quand un `command.run` vise une commande déléguée à QGIS et annoncée dans
`session.hello`, le noyau envoie un ordre. Les paramètres sont déjà résolus : les
chemins sont absolus et décidés par le noyau (AD-10). L'`id` est une chaîne
`core-<n>`.

```json
{"jsonrpc":"2.0","id":"core-1","method":"order.execute","params":{"command":"test.qgis_probe","params":{"layer":"l"}}}
```

L'extension répond sur le même flux, soit par la valeur de la commande :

```json
{"jsonrpc":"2.0","id":"core-1","result":{"ok":true}}
```

soit par une erreur dont `data` porte un `ErrorReport` `{code, message}`. Le
`command.run` d'origine rend alors cette erreur, et l'étape est journalisée en
erreur avec ce code :

```json
{"jsonrpc":"2.0","id":"core-1","error":{"code":-32000,"message":"Couche absente.","data":{"code":"qgis.layer_missing","message":"La couche est absente."}}}
```

Une réponse à un `id` inconnu ou déjà soldé est ignorée.

## `request.cancel` (dans les deux sens)

`params` : `{"id": <identifiant d'une requête en vol>}`. Reçu comme requête, il
répond `{"cancelled": true}` ; reçu comme notification, il ne répond pas.

Du client vers le noyau, il annule le `command.run` en vol, à condition que sa
commande soit déclarée `cancellable` :

```json
{"jsonrpc":"2.0","id":3,"method":"request.cancel","params":{"id":2}}
```

Le noyau envoie alors à l'extension la notification d'annulation de l'ordre :

```json
{"jsonrpc":"2.0","method":"request.cancel","params":{"id":"core-1"}}
```

puis attend la réponse à l'ordre au plus 5 secondes (`CANCEL_GRACE`), pour ne pas
supprimer un fichier que l'extension écrit encore. L'extension doit donc
interrompre son travail et répondre à l'ordre (par n'importe quelle réponse).
Ensuite le `command.run` répond l'erreur `registry.cancelled`, le journal reçoit
une ligne `cancelled: true` (sans `result` ni `error`), et les fichiers de sortie
de l'étape sont supprimés. Le rejeu saute une étape annulée.

Aucune commande du noyau (`core`) n'est annulable.

## `journal.replay` (extension vers noyau)

Le noyau rejoue le journal du projet par le canal (AD-11) : même `Router`, mêmes
résolutions de chemins que l'exécution d'origine, et pour les commandes QGIS les
mêmes ordres `order.execute` vers l'extension, que celle-ci exécute comme d'habitude.
Aucun paramètre :

```json
{"jsonrpc":"2.0","id":4,"method":"journal.replay"}
```

Réponse en cas de succès : `{"steps": 3, "identical": true}`. Le rejeu s'arrête au
premier écart et répond l'erreur `replay.diverged`, dont le `step` est l'étape fautive
et dont le message nomme la commande.

- Le rejeu **n'écrit pas le journal** ; ses sorties vont dans `replay/`, vidé au départ.
- Il n'est **pas annulable** : `request.cancel` répond `request.unknown`.
- Les `layer_id` changent à chaque session. Le journal garde celui que chaque étape a
  rendu (champ `layer_id` de la ligne) ; le rejeu le lie à celui qu'il obtient, le
  substitue dans les paramètres des étapes suivantes, et ne le compare pas. C'est un
  écart assumé à AD-11, qui voulait des repères `out:k.n` : `layer.add` n'a pas de
  sortie fichier.
- Les octets d'une sortie produite par QGIS (GeoPackage, PDF) portent dates et
  identifiants et ne sont pas comparés : le fichier doit exister et n'être pas vide, et
  le résultat typé doit être le même. Les sorties du noyau restent comparées octet par
  octet. Un chemin sous le dossier de sortie est comparé par son nom de fichier.
- Une commande QGIS que l'extension n'a pas annoncée à `session.hello` fait répondre
  `registry.unavailable`, avec le nom de la commande, à l'étape qui la porte.
- Le journal compte tout ce que le noyau a exécuté, `system.about` du démarrage compris :
  il est rejoué comme le reste.

## Plans de l'agent : `agent.models`, `plan.propose`, `plan.execute`, `plan.reject`

Le noyau compose, garde et exécute le plan (AD-5, AD-23). L'extension ne renvoie
jamais un plan, seulement son `plan_id` : le plan montré est le plan exécuté. Un seul
plan attend à la fois ; une nouvelle proposition efface l'ancienne (même si elle
échoue), `plan.execute` et `plan.reject` le consomment.

### `agent.models` (extension vers noyau)

Le noyau seul découvre les modèles (dossier désigné par `CASSINI_MODELS`, sinon `models/` à
côté de l'exécutable) et rend ceux que l'utilisateur peut choisir. Aucun paramètre :

```json
{"jsonrpc":"2.0","id":4,"method":"agent.models"}
```

Réponse, triée par nom ; chaque entrée porte le **nom de fichier seul** (un GGUF, ou une
déclaration `*.modele.json`) et sa taille en octets, jamais un dossier ni un chemin (AD-9) :

```json
{"jsonrpc":"2.0","id":4,"result":{"models":[
  {"file":"Qwen3.5-9B-Q4_K_M.gguf","size_bytes":5680000000},
  {"file":"ajuste.modele.json","size_bytes":2700000000},
  {"file":"casse.modele.json","size_bytes":0,"problem":"la déclaration de modèle n'est pas lisible (expected value at line 1 column 1)"},
  {"file":"manque.modele.json","size_bytes":0,"problem":"fichier manquant : base.gguf"}]}}
```

`problem` (facultatif) dit pourquoi un modèle ne peut pas servir : déclaration illisible,
ou GGUF nommé absent du dossier. Une telle entrée reste listée pour que l'utilisateur voie
la raison ; elle n'est jamais prise par défaut. Aucun GGUF n'est ouvert pour lister.

- C'est un **écart assumé à AD-15** (décision du 2026-10-18) : la liste ne passe pas par
  `command.run model.list` et **n'est pas journalisée**. `model.list` reste inchangé.
- Il faut une session ouverte (`session.not_started` sinon) ; sans compositeur, l'erreur est
  `agent.unavailable` ; pendant un ordre en vol, `session.busy`.
- Une erreur de découverte (`models.no_folder`, `models.unreadable`) remplace le dossier par
  « (dossier des modèles) ».

### `plan.propose` (extension vers noyau)

```json
{"jsonrpc":"2.0","id":5,"method":"plan.propose","params":{"request":"Tampon de 500 m autour des gares.","model":"ajuste.modele.json"}}
```

`model` est facultatif : un nom de la liste d'`agent.models`, comparé tel quel. Absent (ou
`null`), le noyau prend le **premier modèle sans problème**. Un nom absent de la liste répond
`models.unknown`, un modèle à problème `models.unusable` avec la raison ; dans les deux cas
avant tout ordre d'inventaire. Un `model` qui n'est pas un texte répond `-32602`. Changer de
modèle d'une proposition à l'autre recharge le modèle (le noyau garde un seul modèle à la fois).

Le noyau cherche son modèle, puis demande l'inventaire à l'extension par un ordre
`order.execute` de `project.inventory` (non journalisé), compose le plan et le vérifie :
`check_plan`, puis la disponibilité de chaque commande (AD-7). Un plan non conforme est
refusé, jamais réparé. Réponse :

```json
{"jsonrpc":"2.0","id":5,"result":{"plan_id":"plan-1","plan":{"kind":"calls","calls":[
  {"command":"analysis.buffer","params":{"layer_id":"gares_1","distance":{"value":500,"unit":"m"}}},
  {"command":"layout.export_pdf","params":{"title":"Gares"}}]}}}
```

Une étape peut citer la sortie d'une étape précédente : `step:<k>` (sortie entière) ou
`step:<k>.<champ>` (un champ de premier niveau). L'extension affiche le plan tel quel.

Quand la commande existe mais qu'un mot de la demande ne désigne rien de l'inventaire
(« les pharmacies » sans couche de pharmacies, « les parcelles chères » sans champ de
prix), le modèle pose une **question** au lieu de deviner. Elle est rendue comme un
résultat, pas comme une erreur, et **sans `plan_id`** :

```json
{"jsonrpc":"2.0","id":5,"result":{"question":"Aucune couche de pharmacies dans le projet : laquelle faut-il utiliser ?"}}
```

Rien n'est gardé en attente : une question ne s'exécute jamais, et un `plan.execute`
qui suit répond `plan.unknown`. Comme toute nouvelle proposition, une question efface aussi le plan qui
attendait : son `plan_id` ne s'exécute plus. La réponse de l'utilisateur est une nouvelle demande,
envoyée par un nouveau `plan.propose`. Un refus (aucune commande ne fait la chose)
reste l'erreur `agent.refused`.

La composition bloque le noyau jusqu'à sa fin et **n'est pas annulable** :
`request.cancel` sur l'`id` de `plan.propose` répond `request.unknown`. Pendant
l'ordre d'inventaire, les autres requêtes reçoivent `session.busy`.

### `plan.execute` (extension vers noyau) — la validation de l'utilisateur

```json
{"jsonrpc":"2.0","id":6,"method":"plan.execute","params":{"plan_id":"plan-1"}}
```

Le noyau revérifie le plan gardé, puis l'exécute **une étape à la fois**, chacune par
le même chemin que `command.run` (journal, chemins de sortie, annulation). Les
références `step:k` sont remplacées par les valeurs déjà obtenues ; le journal garde
ces valeurs résolues. Avant chaque étape, le noyau envoie à l'extension une
**notification** `plan.step` (noyau vers extension, sans `id`, donc sans réponse
attendue) :

```json
{"jsonrpc":"2.0","method":"plan.step","params":{"plan_id":"plan-1","step":1,"command":"analysis.buffer"}}
```

suivie, pour une commande QGIS, de l'ordre `order.execute` de l'étape. L'ordre suivant
ne part qu'après la réponse au précédent. Réponse finale :

```json
{"jsonrpc":"2.0","id":6,"result":{"plan_id":"plan-1","steps":[
  {"step":1,"command":"analysis.buffer","result":{"layer_id":"tampon_2","name":"Tampon","feature_count":18,"crs":"EPSG:2154"}},
  {"step":2,"command":"layout.export_pdf","result":{"file":"2-layout.export_pdf.pdf","page_count":1,"layer_count":2,"sources":[],"truncated":false}}]}}
```

Un chemin sous le dossier de travail est rendu relatif à lui : aucun chemin absolu.

- **Première erreur** : les étapes suivantes ne partent pas ; l'erreur garde le code
  de l'étape (celui de l'extension pour une commande QGIS) et porte `step`, le numéro
  de l'étape **dans le plan** (à partir de 1).
- **Annulation** : `request.cancel` avec l'`id` du `plan.execute` annule l'étape en vol,
  selon les règles ci-dessus (une étape non annulable répond `request.not_cancellable`
  et continue). L'erreur est `registry.cancelled` avec `step`, l'étape est journalisée
  `cancelled`, ses sorties partielles sont supprimées, la suite ne part pas.
- **Une seule exécution** : rappeler `plan.execute` avec le même `plan_id` répond
  `plan.unknown` et n'envoie rien.

### `plan.reject` (extension vers noyau)

```json
{"jsonrpc":"2.0","id":7,"method":"plan.reject","params":{"plan_id":"plan-1"}}
```

Réponse `{"plan_id":"plan-1","rejected":true}` ; le plan est oublié. Un `plan_id` qui
n'est pas celui du plan en attente répond `plan.unknown` et laisse ce plan en attente.

## Erreurs

| Code | Quand |
|---|---|
| `registry.cancelled` | le `command.run` ou l'étape en vol d'un `plan.execute` a été annulé, ou l'entrée s'est fermée pendant l'ordre |
| `session.busy` | un `command.run`, un `journal.replay`, un `session.hello`, un `agent.models` ou une méthode `plan.*` arrive pendant un ordre en vol (y compris pendant un rejeu, une étape de plan ou l'inventaire d'une proposition) ; refus immédiat |
| `plan.unknown` | `plan.execute` ou `plan.reject` sur un `plan_id` inconnu, déjà exécuté, rejeté ou remplacé ; rien n'est exécuté |
| `agent.unavailable` | `plan.propose` ou `agent.models` : le noyau n'a pas de compositeur de plans |
| `models.unknown` | `plan.propose` : `model` n'est pas dans la liste d'`agent.models` ; rien n'est composé, aucun ordre d'inventaire |
| `models.unusable` | `plan.propose` : `model` désigne un modèle à problème ; le message porte la raison ; aucun ordre d'inventaire |
| `ai.empty_prompt` | `plan.propose` : la demande est vide ou blanche ; refusée avant toute recherche de modèle et sans ordre d'inventaire |
| `registry.unavailable` | `plan.propose` : l'extension n'a pas annoncé `project.inventory` ; rien n'est composé |
| `session.io` | `plan.execute` : la notification `plan.step` n'a pas pu être écrite ; l'étape `step` n'est pas lancée |
| `ai.no_model`, `models.*`, `ai.*` | `plan.propose` (et `agent.models` pour `models.*`) : aucun modèle utilisable, dossier des modèles absent, ou échec du modèle ; le message ne porte aucun chemin absolu |
| `agent.refused` | `plan.propose` : le modèle refuse la demande, avec sa raison |
| `agent.plan_invalid` | `plan.propose` (ou `plan.execute` à la revérification) : plan non conforme ou nommant une commande non annoncée ; `step` désigne l'étape fautive |
| `plan.missing_output`, `plan.bad_reference` | `plan.execute` : une référence `step:k.champ` ne se résout pas sur la sortie obtenue ; `step` désigne l'étape |
| `replay.diverged` | `journal.replay` : l'étape `step` ne redonne pas le résultat, l'erreur ou la sortie du journal |
| `registry.unavailable` | `journal.replay` : une commande QGIS du journal n'est pas annoncée par l'extension |
| `request.not_cancellable` | `request.cancel` sur une commande non annulable ; rien ne change |
| `request.unknown` | `request.cancel` sur un `id` inconnu ou déjà soldé ; rien ne change |

Toute autre requête reçue pendant un ordre reçoit sa réponse normale, sans
attendre la fin de l'ordre.

## Fermeture de l'entrée

Si l'entrée se ferme pendant un ordre, l'étape est annulée comme ci-dessus (sans
attente, puisque plus rien ne peut arriver), le serveur se termine proprement et
le verrou du journal est libéré. Aucun côté ne tue l'autre.
