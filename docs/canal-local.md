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

## Erreurs

| Code | Quand |
|---|---|
| `registry.cancelled` | le `command.run` a été annulé, ou l'entrée s'est fermée pendant l'ordre |
| `session.busy` | un `command.run`, un `journal.replay` ou un `session.hello` arrive pendant un ordre en vol (y compris pendant un rejeu) ; refus immédiat |
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
