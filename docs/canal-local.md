# Canal local : ordres du noyau, annulation et rejeu

Le canal de `cassini serve` est du JSON-RPC 2.0, un message par ligne, sur
l'entrée et la sortie standard, **dans les deux sens** (AD-3). Le protocole est
à la version **2** depuis le canal d'IA externe (AD-7, AD-29, 2026-10-30). Ce document fixe
les messages que l'extension QGIS consomme, et ceux du canal d'IA externe.

Un fil du noyau lit l'entrée ; la boucle principale est seule à écrire sur la sortie. Les
relais du canal d'IA externe ont chacun un fil lecteur et un fil écrivain : le fil lecteur
répond lui-même à ce qui n'exige pas l'exécutant, et ne verse dans la file de la boucle que
les `command.run` qu'il a pu réserver et les `request.cancel`. Une seule étape s'exécute à la
fois (AD-5), sous **un seul état d'occupation** pour l'extension et les relais (AD-29, voir
« Occupation »).

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
commande soit déclarée `cancellable`. L'extension vise le sien par son `id`, et l'appel d'un
relais par son `call_id` (voir `external.call`) ; un relais ne vise que l'appel de sa propre
connexion, par l'`id` de son `command.run`. Tout autre identifiant répond `request.unknown`.

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
{"jsonrpc":"2.0","id":5,"result":{"question":"Aucune couche de pharmacies dans le projet : laquelle faut-il utiliser ?","source":"model"}}
```

Le champ `source`, toujours présent, dit d'où vient la question : `model` (le modèle l'a
posée) ou `guard` (le garde-fou du noyau, AD-23). Le garde-fou s'applique à tout plan composé
par le modèle local : une coordonnée du plan (un `Point` : x, y et code EPSG) doit venir de
la demande — x, y et le système y figurent tous trois, comme nombres, ou le système par son
nom (Lambert-93, WGS84) — ou d'une étape précédente (`step:<k>.<champ>` vers un champ déclaré
`Point`). Sinon le plan n'est pas montré et le noyau rend sa question :

```json
{"jsonrpc":"2.0","id":5,"result":{"question":"Il manque à la demande un point en coordonnées : quels sont x, y et le système (code EPSG, par exemple 2154 pour le Lambert-93) ?","source":"guard"}}
```

Sur une question `guard`, l'extension remet la phrase d'origine dans la zone de saisie,
pour que l'utilisateur la complète par le point. La provenance n'est jamais écrite au journal.

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

## Canal d'IA externe : `session.external` et `external.attach`

Le canal d'IA externe (AD-2, AD-14, AD-29) est un second canal local, réservé à `cassini mcp`
en mode relais. Il n'existe que tant que l'utilisateur l'autorise dans le panneau : **éteint par
défaut**, jamais rallumé seul par le noyau. Même grammaire que le canal de l'extension
(JSON-RPC 2.0, un message par ligne), mais le noyau n'y envoie aucune requête : il répond, et
notifie la perte de la session. **Aucun chemin ne passe** sur ce canal, dans aucun sens (AD-9).

**Nom du canal**, propre à l'utilisateur :

- Windows : tube nommé `\\.\pipe\cassini-<utilisateur>-external`, qui refuse les clients
  distants (`PIPE_REJECT_REMOTE_CLIENTS` : une connexion par `\\localhost\pipe\…` échoue), n'admet
  que l'utilisateur courant (descripteur `O:<SID>D:P(A;;GA;;;<SID>)`), et dont la première
  instance est exclusive (`FILE_FLAG_FIRST_PIPE_INSTANCE`). Le client vérifie que le tube
  appartient à l'utilisateur courant et se branche en `SECURITY_SQOS_PRESENT`.
- Unix : socket de domaine `$XDG_RUNTIME_DIR/cassini/external.sock`, sinon
  `<dossier temporaire>/cassini-<uid>/external.sock`. Le dossier est en 0700 et appartient à
  l'utilisateur (vérifié, jamais corrigé : un dossier ouvert aux autres fait refuser le canal).
  Jamais l'espace abstrait de Linux. Une socket orpheline laissée par un noyau mort (personne ne
  répond) est supprimée avant d'ouvrir.
- La variable `CASSINI_EXTERNAL_TAG`, lue seulement si elle est posée, isole le nom par un
  suffixe : elle sert aux tests, jamais à l'utilisateur.

### `session.external` (extension vers noyau)

```json
{"jsonrpc":"2.0","id":8,"method":"session.external","params":{"enabled":true,"ask":false}}
```

Allumé, le noyau ouvre le canal et répond `{"enabled": true, "ask": false}`. `ask` est la case
« Me demander avant chaque appel ». La validation de chaque appel (`external.approve`) est un
ticket à part : d'ici là, **la case allumée refuse tout appel d'un relais** par
`external.not_approved`, sans rien exécuter. Rallumer un canal déjà ouvert ne change que `ask`.

```json
{"jsonrpc":"2.0","id":9,"method":"session.external","params":{"enabled":false}}
```

Éteint, le noyau envoie à chaque relais branché la notification `external.session_lost`, coupe
les connexions, ferme le canal (la socket Unix est supprimée) et répond `{"enabled": false}` ;
même réponse si le canal était déjà éteint.

- Il faut une session ouverte (`session.not_started` sinon). `enabled` est requis, et `ask`
  pour allumer : sinon `-32602`.
- Une autre session QGIS du même utilisateur tient déjà le canal : `external.channel_taken`, et
  rien n'est ouvert. Une autre panne d'ouverture : `external.io`.
- `session.external` est servi aussi pendant un ordre en vol : il n'exige pas l'exécutant.

### `external.attach` (relais vers noyau, sur le canal d'IA externe)

La poignée de main du relais (AD-7) :

```json
{"jsonrpc":"2.0","id":1,"method":"external.attach","params":{"protocol_version":2}}
```

Réponse : la version du protocole, celle du noyau, et le **catalogue disponible** de la session
(AD-25) — les seules commandes disponibles, sous la forme du catalogue de `session.hello`, sans
aucun chemin :

```json
{"jsonrpc":"2.0","id":1,"result":{"protocol_version":2,"core_version":"0.1.0","commands":[{"id":"system.about","available":true}]}}
```

Une version différente répond `session.protocol_mismatch`, dont le message nomme le binaire à
mettre à jour : le relais (`cassini mcp`) s'il est en retard, le paquet du noyau de la session
QGIS sinon. Une `protocol_version` absente ou qui n'est pas un entier répond `-32602`.

`external.attach` est répondu à toute heure, **pendant un ordre en vol et pendant une
composition de plan compris**, sans attendre leur fin : le fil lecteur du relais y répond
lui-même. Un relais n'emploie sinon que `command.run` et `request.cancel` (AD-6) ; toute autre
méthode répond « méthode inconnue » (`-32601`) et rien n'est exécuté. Une notification ne
reçoit rien (un `command.run` sans `id` n'est pas exécuté), une ligne illisible reçoit une
erreur à identifiant nul.

### `command.run` (relais vers noyau)

Même forme que pour l'extension, sur une commande du **catalogue disponible** :

```json
{"jsonrpc":"2.0","id":7,"method":"command.run","params":{"command":"layer.describe","params":{"layer_id":"l1"}}}
```

Le noyau de la session l'exécute par le même chemin que celui de l'extension (`Session::call_from`, qui note l'origine) :
validation, chemins de sortie, journal, et pour une commande QGIS l'ordre `order.execute` dicté à
l'extension (AD-5), que celle-ci exécute comme d'habitude. Il répond au relais par la valeur de la
commande ou par une erreur AD-21.

- **Avant tout** : session ouverte (`session.not_started` sinon), exécutant libre (`session.busy`
  sinon, aussitôt, rien n'est mis en file), case `ask` éteinte (`external.not_approved` sinon).
- Une commande inconnue répond `registry.unknown_command`, une commande QGIS non annoncée
  `registry.unavailable` ; rien ne part vers l'extension.
- **Chemins (AD-9)** : un paramètre de chemin de la commande (`path_params`, par exemple `path`
  de `layer.add`) doit être relatif au projet et ne pas en sortir. Absolu (`/x`, `C:\x`,
  `\\serveur\x`) ou sortant par `..` : `external.path_refused`, rien n'est exécuté ni
  journalisé, et le message ne cite que le nom du paramètre.
- **Aucun chemin absolu ne repart** : le résultat est rendu relatif au dossier de travail ; un
  texte qui est en entier un chemin du projet devient relatif au projet, un autre chemin absolu
  entier son nom de fichier. Dans un message (d'erreur ou non), les dossiers du projet et de
  travail deviennent `.`, et tout autre chemin absolu cité est réduit à son nom de fichier. Même
  règle pour `external.call`.
- **Journal** : la ligne de l'étape porte `"origin": "external"`. Le rejeu l'ignore et rejoue
  l'étape à l'identique ; il n'écrit rien, les lignes gardent leur origine.

### `external.call` (noyau vers extension, notification)

Chaque appel d'un relais qui part à l'exécution est annoncé à l'extension, commandes `core`
comprises, au début puis à la fin. Le `call_id`, `external-<n>`, est donné par le noyau ; le
panneau arrête l'appel en le renvoyant dans `request.cancel`.

```json
{"jsonrpc":"2.0","method":"external.call","params":{"call_id":"external-3","event":"started","command":"layer.describe","params":{"layer_id":"l1"}}}
{"jsonrpc":"2.0","method":"external.call","params":{"call_id":"external-3","event":"finished","command":"layer.describe","result":{},"duration_ms":41}}
```

`finished` porte `result`, ou `error {code, message}`. Un appel refusé avant l'exécution
(occupation, case `ask`, chemin, commande inconnue ou indisponible, paramètres mal formés)
n'est pas annoncé. L'extension, qui ne connaît pas encore cette notification, l'ignore.

### Occupation

Un seul état d'occupation par processus, valable dans les deux sens (AD-29). L'occupent : une
composition ou une exécution de plan, un `command.run` ou un `journal.replay` de l'extension, un
appel d'un relais. Un **plan proposé non tranché** occupe pour les relais seulement : l'extension
garde son `command.run` et peut le remplacer par un nouveau `plan.propose`.

- Côté relais : pendant une occupation, `command.run` répond `session.busy` aussitôt ; quand
  seul un plan proposé attend, le message dit qu'il attend la validation de l'utilisateur.
- Côté extension : pendant un appel d'un relais, `command.run`, `plan.*` et `journal.replay`
  répondent `session.busy` (`session.external` reste servi). Un appel de relais déjà réservé
  mais encore en file passe avant l'extension.
- Plusieurs relais peuvent se brancher ; leurs appels passent un à la fois.

### `request.cancel` (relais vers noyau)

`{"id": <id du command.run du relais>}`. Mêmes règles que pour l'extension : annulable seulement
si la commande l'est (`request.not_cancellable` sinon), l'ordre est annulé (AD-12), l'étape est
journalisée `cancelled` et le `command.run` répond `registry.cancelled`. Un autre `id`, ou celui
d'un autre relais : `request.unknown`.

### Perte de la session ou du relais en plein appel

- **Interrupteur coupé** pendant l'ordre d'un appel de relais : l'ordre est annulé s'il est
  annulable, sinon il finit et l'étape est journalisée. Le relais reçoit
  `external.session_lost` et **aucune réponse**.
- **Entrée fermée** (QGIS fermé) : l'ordre est annulé comme tout ordre (plus rien ne peut y
  répondre), l'étape est journalisée annulée ; le relais reçoit `external.session_lost`
  (`session_ended`) et aucune réponse.
- **Relais débranché** en plein appel : même traitement, la réponse est abandonnée, la session
  est libérée.

### `external.session_lost` (noyau vers relais, notification)

```json
{"jsonrpc":"2.0","method":"external.session_lost","params":{"reason":"switched_off","message":"L'utilisateur a fermé le canal d'IA externe dans QGIS."}}
```

`reason` vaut `switched_off` (interrupteur éteint) ou `session_ended` (entrée fermée : QGIS
s'arrête). La connexion est fermée juste après. Le noyau confie d'abord la notification à tous
les relais, puis leur laisse **une demi-seconde en tout**, quel que soit leur nombre, pour la
recevoir : un relais qui ne lit plus ne retient pas la session au-delà (AD-4).

### Le relais vu de `cassini mcp` (ticket 5.25)

`cassini mcp` a deux modes (AD-13), choisis à chaque `tools/list` et `tools/call` de son client,
jamais par sondage :

- **Relais** : une connexion vivante au canal est gardée ; sinon `cassini mcp` se branche
  (`ChannelName::for_user`) et envoie `external.attach`. Une réponse fait le mode relais : les
  outils sont les entrées `available` du catalogue rendu (nom `id`, description en français puis
  en anglais, schémas de la session), commandes QGIS comprises ; le registre propre du relais
  n'est jamais lu. Chaque `tools/call` devient un `command.run`. `--project` est ignoré, et aucun
  chemin ne part (AD-9).
- **Autonome** : aucun canal, ou une session qui ne répond pas. Les outils sont les commandes
  `runs_in: core` (ticket 5.7) ; avec `--project`, le projet est ouvert puis refermé à chaque
  appel, comme `cassini run` : le verrou du journal n'est jamais tenu entre deux appels. Un projet
  introuvable reste refusé au démarrage (code 1).
- **Versions différentes** : `external.attach` répond `session.protocol_mismatch`. Les outils
  restent ceux du mode autonome, et chaque `tools/call` rend un résultat `isError` portant cette
  erreur, dont le message nomme le binaire à mettre à jour ; rien n'est exécuté.

Le serveur annonce `tools.listChanged` ; dès que le catalogue servi diffère du dernier servi
(changement de mode ou de catalogue), il envoie `notifications/tools/list_changed`. Il l'envoie
aussi dès la réception d'`external.session_lost` (ou la fin du flux du canal), et la requête
suivante est servie en autonome.

Traduction (AD-29) :

| MCP | Canal d'IA externe |
|---|---|
| `tools/call` d'un outil du catalogue | `command.run {command, params}`, `id` propre à la connexion |
| `notifications/cancelled` de cet appel | notification `request.cancel {id}`, puis attente de la réponse du `command.run` (le service MCP l'abandonne, le client ayant annulé) |
| réponse `result` | résultat structuré |
| erreur portant un `ErrorReport` (AD-21) | résultat `isError`, texte = `ErrorReport` en JSON (comme en 5.7) |
| `external.session_lost` ou fin du flux pendant l'appel | résultat `isError` `external.session_lost` ; l'appel n'est **jamais** réexécuté en autonome |
| erreur sans `ErrorReport` (erreur de protocole de la session, par exemple `-32602`) | erreur de protocole MCP de même code et même message |
| outil hors catalogue du mode courant | erreur de protocole `-32602` |

Appels simultanés : en relais, la session n'exécute qu'un appel à la fois ; un second `tools/call`
pendant qu'un premier est en cours reçoit aussitôt le résultat `isError` `session.busy`. En
autonome, le fil d'exécution les met en file et les sert dans l'ordre.

Le runtime de `cassini mcp` n'attend jamais un appel de façon bloquante : chaque requête est une
tâche, et `ping` est servi pendant une commande longue, dans les deux modes. L'annulation du
client devient `request.cancel` en relais ; en autonome, la réponse de l'appel annulé est
abandonnée, mais la commande du noyau va jusqu'au bout (et, avec `--project`, est journalisée).
Seuls transports : l'entrée et la sortie standard côté client, le canal d'IA externe côté session
(AD-14).

## Erreurs

| Code | Quand |
|---|---|
| `registry.cancelled` | le `command.run` ou l'étape en vol d'un `plan.execute` a été annulé, ou l'entrée s'est fermée pendant l'ordre |
| `session.busy` | un `command.run`, un `journal.replay`, un `session.hello`, un `agent.models` ou une méthode `plan.*` arrive pendant un ordre en vol (y compris pendant un rejeu, une étape de plan ou l'inventaire d'une proposition) ; un `command.run`, un `journal.replay` ou une méthode `plan.*` de l'extension pendant un appel de relais ; un `command.run` de relais pendant toute occupation (composition et plan non tranché compris) ; refus immédiat |
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
| `request.unknown` | `request.cancel` sur un `id` inconnu ou déjà soldé, ou sur l'appel d'une autre connexion ; rien ne change |
| `external.channel_taken` | `session.external` : une autre session QGIS de l'utilisateur tient déjà le canal d'IA externe ; rien n'est ouvert |
| `external.io` | `session.external` : le canal d'IA externe n'a pas pu s'ouvrir (dossier qui n'est pas privé, panne du système) ; le message ne porte aucun chemin |
| `session.protocol_mismatch` | `session.hello` ou `external.attach` : la version de protocole diffère ; le message nomme ce qu'il faut mettre à jour |
| `session.not_started` | toute requête de l'extension avant `session.hello`, `session.external` compris ; `external.attach` ou `command.run` d'un relais sans session ouverte |
| `external.not_approved` | `command.run` d'un relais alors que la case « Me demander avant chaque appel » est cochée ; rien n'est exécuté |
| `external.path_refused` | `command.run` d'un relais dont un paramètre de chemin est absolu ou sort du projet ; rien n'est exécuté ni journalisé |
| `external.session_lost` | résultat `isError` d'un `tools/call` de `cassini mcp` en relais : la session a été perdue pendant l'appel (interrupteur coupé, QGIS fermé) ; rien n'est réexécuté en autonome |

Toute autre requête reçue pendant un ordre reçoit sa réponse normale, sans
attendre la fin de l'ordre.

## Fermeture de l'entrée

Si l'entrée se ferme pendant un ordre, l'étape est annulée comme ci-dessus (sans
attente, puisque plus rien ne peut arriver), le serveur se termine proprement et
le verrou du journal est libéré. Aucun côté ne tue l'autre.

Quelle que soit la cause de la fin (entrée fermée, sortie en panne), les relais branchés au
canal d'IA externe reçoivent `external.session_lost` (`session_ended`), puis le canal est fermé
et la socket Unix supprimée, avant que le serveur ne rende la main.
