# Relecture adverse — AD-29 et précisions du 2026-10-30 (AD-2, AD-13, AD-14)

Date : 2026-10-30. Objet : l'amendement de la colonne d'architecture qui ouvre la session QGIS à une
IA externe par relais (`cassini mcp` → canal local → `cassini serve`), préalable au ticket 5.24
(`risk: high`). Les décisions du propriétaire (piste B, case « Me demander avant chaque appel »
éteinte par défaut, ChatGPT web hors périmètre) sont tenues pour acquises et ne sont pas rediscutées.

Lentille : construire, un niveau plus bas, deux unités qui obéissent chacune à la lettre de toutes
les AD et qui pourtant ne s'assemblent pas. Chaque paire trouvée est un trou ; chaque trou reçoit le
texte de règle minimal qui le ferme.

## Verdict

**À resserrer avant 5.24.** Le principe est sain — un seul exécutant, un seul auteur du journal, un
relais qui transporte — mais la règle s'appuie sur trois choses que le reste du contrat définit
autrement :

1. `session.hello` est, par AD-7 et dans le code, **l'annonce de l'exécutant** (version, commandes
   qu'il sait exécuter, chemin absolu du projet) et **l'ouverture du journal**. AD-29 la fait
   envoyer par le relais, qui n'est pas un exécutant et ne connaît pas le projet.
2. « Aucune méthode nouvelle » est incompatible avec la visibilité et la validation exigées par la
   même règle : rien, dans la liste fermée de AD-6, ne permet au noyau de montrer au panneau un
   appel externe qui n'emprunte pas `order.execute`, ni de lui demander une validation.
3. « Canal local » désigne désormais deux canaux (AD-2), alors que l'exception de AD-9 sur les
   chemins absolus vaut « sur le canal local, et là seulement ».

Douze trous au total ; les cinq premiers sont bloquants.

## Ce que le code montre (état existant)

- `core/src/serve.rs` : un `State` par connexion — `session`, `announced`, `project`, `pending`
  (plan en attente), `intermediates` (5.22). `hello` refuse un second `session.hello`
  (`session.already_started`), exige `protocol_version`, `commands` et `project` absolu, puis
  **ouvre le journal** (`Session::open`) et rend `catalogue(state)` (descriptions sérialisées,
  schémas compris, plus `available`).
- `request.cancel` compare l'`id` reçu à **une seule** cellule `link.run_id` ; hors ordre en vol,
  rien n'est annulable (`request.unknown`). `session.busy` n'est rendu que **pendant un ordre**
  (`answer_during_order`) ; une commande `runs_in: core` ou une composition de plan s'exécute dans
  la boucle principale, qui ne lit plus rien pendant ce temps.
- `cli/src/mcp.rs` (5.7) : le catalogue MCP est construit depuis **le registre compilé du binaire**
  (`Registry::standard()`), commandes `core` seules. Avec `--project <fichier>`, le serveur MCP
  **ouvre lui-même une session de journal** sur ce projet (`Session::open`). Outil inconnu → erreur
  JSON-RPC `invalid_params` ; erreur de commande → `isError` avec l'enveloppe AD-21 sérialisée en
  texte. L'appel est servi sur place, de façon bloquante, « un à la fois, dans l'ordre ».

## Trous bloquants

### T1 — `session.hello` a deux sens incompatibles

- **Unité A (noyau, AD-7 à la lettre)** : sur toute connexion, la première requête est
  `session.hello` avec `commands` et `project` ; elle ouvre la session et le journal.
- **Unité B (relais, AD-29 à la lettre)** : envoie `session.hello` sur le canal d'IA externe pour
  apprendre le catalogue. Il ne sait pas quoi mettre dans `commands` (il n'exécute rien) ni dans
  `project` (il ne connaît pas le projet).
- **Résultat** : soit le relais annonce `commands: []` et le noyau, s'il traite la connexion comme
  une nouvelle session, rend toutes les commandes `qgis` indisponibles ; soit il ouvre un second
  `Session::open` sur le même journal (lecture seule, ou conflit de verrou dans le même processus) ;
  soit le noyau répond `session.already_started`. Trois implémentations conformes, trois ruptures.
- **Règle proposée (AD-29, sous-règle *Annonce*)** :
  > Sur le canal d'IA externe, `session.hello` est une **requête du relais** qui porte sa seule
  > `protocol_version` et le rôle `relay` ; elle n'ouvre ni session ni journal, ne porte ni
  > `commands` ni `project`, et ne change pas la liste annoncée par l'extension. Le noyau répond
  > par le catalogue de la session en cours, sans aucun chemin. Elle est refusée
  > (`session.not_started`) tant que l'extension n'a pas elle-même ouvert la session (AD-7, AD-25).
  > Seul `session.hello` de l'extension ouvre la session et le journal.

### T2 — « Aucune méthode nouvelle » contre « chaque appel est montré, et validé si la case est cochée »

- **Unité A (noyau)** : n'utilise que la liste fermée de AD-6 ; un appel externe d'une commande
  `runs_in: core` (`terrain.slope`, `geocode.*`, `system.about`) ne produit aucun `order.execute`,
  donc **aucun message** vers l'extension.
- **Unité B (panneau)** : montre les appels externes en déduisant leur existence des
  `order.execute` qu'il reçoit hors de toute demande de sa part.
- **Résultat** : les appels `core` sont invisibles, la case « Me demander avant chaque appel » n'a
  aucun moyen d'être exprimée (le noyau ne peut rien demander à l'extension hors `order.execute`),
  et un développeur qui ajoute `session.external_call` se croit couvert par le joker `session.*` de
  AD-6 pendant qu'un autre la croit interdite par AD-29.
- **Règle proposée (AD-6, liste fermée, et AD-29)** :
  > La liste fermée gagne, sur le canal de l'extension et dans le sens noyau → extension, une
  > notification `external.call` (début, fin : commande, paramètres en clair, résultat ou erreur
  > AD-21, durée) et une requête `external.approve` (même contenu, réponse `{approved: bool}`),
  > envoyée seulement quand la case est cochée. Le joker `session.*` ne couvre aucune méthode non
  > nommée ici. Le noyau émet `external.call` pour **tout** appel externe, `core` ou `qgis`, et le
  > panneau n'infère jamais un appel externe d'un `order.execute`. `order.execute` porte en plus
  > `origin: "panel" | "external"`.
  >
  > (Corriger en conséquence la phrase de AD-29 : « aucune méthode nouvelle **sur le canal d'IA
  > externe** ».)

### T3 — Deux propriétaires de l'état « occupé », et une file qui existe quand même

- **Unité A (noyau, code actuel)** : `session.busy` n'est rendu que pendant un ordre en vol ;
  pendant une composition de plan (14 à 35 s) ou une commande `core`, la boucle ne lit rien.
- **Unité B (relais)** : écrit sa requête dans le tube et attend.
- **Résultat** : la requête externe attend dans le tampon du système, puis s'exécute après le plan —
  elle a été **mise en file**, ce que AD-29 interdit, sans qu'aucune unité n'enfreigne sa règle. Et
  « pendant un plan » n'est pas défini : un plan proposé qui attend la validation de l'utilisateur
  (`State.pending`) est-il « un plan » ? Inversement, la règle ne dit rien d'une demande du panneau
  (`plan.propose`, `command.run`) arrivée pendant un appel externe ; deux `State` par connexion
  donnent deux drapeaux occupé.
- **Règle proposée (AD-29, sous-règle *Un à la fois*, remplacée)** :
  > Le noyau tient **un seul** état d'occupation par processus, partagé par ses deux connexions ;
  > les connexions n'ont pas d'état de session propre. Est occupé : une composition de plan, un plan
  > proposé non encore validé ou rejeté, l'exécution d'un plan, un appel (du panneau ou externe) et
  > l'attente d'une validation `external.approve`. Toute requête qui exige l'exécutant, quelle que
  > soit la connexion, reçoit alors `session.busy` **immédiatement** : le noyau lit ses deux
  > connexions pendant tout travail long (AD-4), une composition de plan et une commande `core`
  > comprises. Une requête n'attend jamais dans un tampon le moment d'être servie.

### T4 — Deux auteurs du journal par le mode autonome

- **Unité A (`cassini mcp --project p`, 5.7, mode autonome)** : ouvre une session de journal sur
  `p`, comme `cassini run`.
- **Unité B (session QGIS sur `p`)** : prend le verrou exclusif à `session.hello` (AD-11).
- **Résultat** : si l'IA externe démarre avant QGIS, c'est **elle** qui tient le journal en
  écriture et la session QGIS passe en lecture seule ; l'utilisateur ne peut plus journaliser
  depuis QGIS. Et si `--project` désigne un autre projet que la session ouverte, AD-13 ne dit pas
  quel mode l'emporte.
- **Règle proposée (AD-13, précision)** :
  > Le mode se décide **à chaque appel** et non au démarrage : canal d'IA externe joignable → relais,
  > sinon autonome. En mode autonome, `cassini mcp` n'ouvre **aucun** journal et ne prend aucun
  > verrou ; `--project` est retiré, ou seulement accepté en lecture. En mode relais, aucun
  > argument du relais ne désigne un projet : c'est celui de la session.

### T5 — « Canal local » désigne deux canaux ; les chemins absolus fuient vers le fournisseur

- **Unité A (noyau)** : applique l'exception de AD-9 « sur le canal local » — désormais aussi le
  nom du canal d'IA externe dans AD-2 — et rend des chemins absolus dans les messages qu'il y écrit
  (résultat, erreur, ou `project` s'il recopie la réponse de hello de l'extension).
- **Unité B (relais)** : transmet tel quel au client MCP, qui l'envoie au réseau de son
  fournisseur.
- **Résultat** : le chemin réel du poste (nom d'utilisateur Windows compris) part chez un tiers ;
  aucune règle n'est enfreinte à la lettre.
- **Règle proposée (AD-9, exception, et AD-2)** :
  > On nomme **canal de l'extension** l'entrée/sortie standard de `cassini serve`, et **canal d'IA
  > externe** celui de AD-29 ; « canal local » n'est plus employé seul. L'exception de AD-9 vaut
  > sur le canal de l'extension seulement. Sur le canal d'IA externe, aucun chemin absolu ne
  > traverse, dans aucun sens : ni dans un résultat, ni dans une erreur, ni dans le catalogue ; un
  > paramètre absolu venu du relais est refusé (`params.absolute_path`).

## Trous importants

### T6 — Version de protocole : deux binaires, un message faux

- **Unités** : l'extension trouve son noyau par l'ordre de AD-17 ; l'application de l'IA lance
  `cassini mcp` depuis le chemin écrit dans **sa** configuration. Ce sont souvent deux binaires
  différents (paquet mis à jour d'un côté, copie ancienne de l'autre).
- **Résultat** : le noyau refuse le relais avec « mettez à jour l'extension Cassini » (message codé
  pour l'extension) ; ou, si la version de protocole est égale mais le registre différent, le relais
  bâtit ses schémas d'outils depuis **son** registre compilé (comme 5.7 le fait) et propose des
  paramètres que la session refusera.
- **Règle proposée (AD-29, *Annonce* et *Catalogue*)** :
  > Un écart de version entre relais et session rend `session.protocol_mismatch` avec un message
  > qui nomme **le binaire du relais** (son chemin tel que le client MCP l'a lancé n'est pas
  > transmis ; le message dit « le `cassini` configuré dans votre application d'IA »). Le relais ne
  > lit **jamais** son propre registre en mode relais : noms, descriptions, schémas d'entrée et de
  > sortie viennent tous de la réponse de la session.

### T7 — Le catalogue change sous le relais

- **Unités** : le relais met le catalogue en cache à sa première connexion ; la session, elle,
  s'arrête et repart (changement de projet, AD-7 ; réouverture de QGIS ; interrupteur éteint puis
  rallumé), avec une autre liste annoncée — ou disparaît, et le relais retombe en autonome.
- **Résultat** : le client MCP appelle `analysis.buffer` qui n'existe plus (« outil inconnu »), ou
  ne voit jamais les commandes QGIS parce que la session est apparue après son `tools/list`.
- **Règle proposée (AD-29, *Catalogue*)** :
  > Le relais déclare la capacité `tools.listChanged` et émet `notifications/tools/list_changed`
  > à chaque bascule de mode et à chaque nouvelle session (nouveau `session.hello` de relais) dont le
  > catalogue diffère. Un appel à un outil retiré rend une erreur d'outil `session.changed`, pas une
  > erreur de protocole.

### T8 — Cycle de vie : qui meurt au milieu d'un appel

- **QGIS se ferme pendant un appel externe** : AD-2 dit que le noyau s'arrête quand son entrée se
  ferme. Une unité s'arrête aussitôt (le relais voit le tube se fermer, sans réponse) ; une autre
  attend la fin de l'ordre, qui ne viendra jamais. Le journal ne dit pas ce que devient l'étape.
- **Le client MCP se déconnecte pendant un appel** : le relais meurt avec son entrée ; la session
  voit la connexion se fermer. Une unité annule l'étape, une autre la mène à terme et écrit le
  résultat dans un tube fermé (erreur d'écriture qui, dans le code actuel, fait échouer `serve`).
- **Règle proposée (AD-29, sous-règle *Cycle de vie*)** :
  > La session ouvre et ferme seule le canal d'IA externe ; le relais n'y crée rien. Fermeture de la
  > connexion du relais pendant un appel = `request.cancel` de cet appel (AD-12) ; une commande non
  > annulable va à son terme et se journalise normalement ; une écriture vers un relais disparu est
  > ignorée et ne termine jamais la session. Fermeture de l'entrée standard (QGIS fermé) : le noyau
  > répond au relais `session.closed` pour l'appel en vol, journalise l'étape comme interrompue, puis
  > s'arrête. Le relais traduit `session.closed` en erreur d'outil et repasse en mode autonome (T7).

### T9 — Annulation : espace des identifiants, propriétaire, délai du client

- **Identifiants** : le noyau compare `request.cancel` à **une** cellule `run_id`. Les `id` JSON-RPC
  ne sont uniques que par connexion : l'extension et le relais envoient tous deux `id: 1`. Un
  `request.cancel {id: 1}` du relais peut viser l'appel du panneau.
- **Délai du client** : les clients MCP ont un délai d'appel d'outil (de l'ordre de la minute chez
  certains) et envoient `notifications/cancelled` à son terme. Une commande non annulable (PDF,
  traitement de QGIS) se termine et se journalise, l'IA croit l'appel échoué et le rejoue : deux
  étapes, deux sorties.
- **Bouton Arrêter** : le panneau peut annuler un appel externe, mais par quel `id` ? Celui de sa
  propre connexion ne nomme pas la requête du relais.
- **Règle proposée (AD-12, précision, et AD-29)** :
  > Une requête est identifiée par **(connexion, `id`)** ; `request.cancel` ne vise qu'une requête de
  > sa propre connexion. Le relais traduit `notifications/cancelled` en `request.cancel`. Le panneau
  > arrête un appel externe par l'identifiant d'appel que porte `external.call` (T2), seul cas où une
  > connexion annule la requête d'une autre. Un appel externe annulé mais non annulable se
  > journalise comme exécuté, et `external.call` de fin le marque « terminé après abandon du
  > client » ; la description MCP de chaque commande non annulable dit qu'elle n'est pas
  > interrompue par un délai.

### T10 — La validation par l'utilisateur contre AD-4 et le délai du client

- **Unité A (noyau)** : la case cochée, envoie `external.approve` et attend la réponse — en
  bloquant la boucle, comme le fait aujourd'hui une commande `core`.
- **Unité B (panneau)** : montre la demande et attend un clic, sans limite.
- **Résultat** : l'utilisateur part déjeuner ; le client MCP abandonne ; la validation arrive après
  coup et exécute un appel que plus personne n'attend — ou le noyau gèle le panneau (AD-4).
- **Règle proposée (AD-29, *Visibilité*)** :
  > L'attente d'une validation n'est jamais bloquante (AD-4) et compte comme « occupé » (T3). Elle
  > prend fin au premier de : validation, refus (`external.refused`), annulation du client
  > (`request.cancel`, la demande disparaît alors du panneau), fermeture de l'une des deux
  > connexions, ou un délai fixe du noyau, constante unique (`external.approval_timeout`). Une
  > validation reçue après cette fin est ignorée. La case est lue par le noyau au moment de chaque
  > appel ; l'extension la transmet par un réglage de session, jamais en filtrant elle-même les
  > appels.

### T11 — Erreurs : deux traductions de AD-21 vers MCP

- **Unités** : 5.7 rend un outil inconnu en erreur JSON-RPC et une erreur de commande en
  `isError` avec l'enveloppe en texte. Un relais pourrait rendre `session.busy` en erreur JSON-RPC
  (« le serveur a échoué »), un autre en `isError` ; un troisième recopie l'erreur de protocole du
  noyau (`METHOD_NOT_FOUND`, `INVALID_PARAMS`) telle quelle.
- **Résultat** : selon le client, l'IA réessaie, abandonne, ou ne voit rien.
- **Règle proposée (AD-21, précision MCP)** :
  > Au bord MCP, toute erreur porteuse d'un code `domaine.cause` — `session.busy`,
  > `session.closed`, `external.refused`, `command.unavailable`, erreurs de commande — devient un
  > résultat d'outil `isError: true` dont `structuredContent` est l'enveloppe AD-21 (et le texte le
  > message français) ; seules les fautes du client MCP lui-même (outil hors catalogue, arguments non
  > objet) sont des erreurs JSON-RPC. Le relais ne reformule aucun message.

### T12 — Une seule session par utilisateur : nom réservé, nom périmé

- **Unités** : sous Unix, la session ouvre une socket à un chemin fixe ; après un plantage de QGIS,
  le fichier reste. La session suivante, conforme à « une seconde session est refusée », se refuse
  elle-même. Sous Windows, n'importe quel processus de l'utilisateur peut créer le tube en premier,
  et le relais parlera à lui.
- **Règle proposée (AD-29, *Une seule session*)** :
  > Une session qui trouve le nom pris tente de s'y connecter : si personne ne répond, le nom est
  > périmé, elle le supprime et l'ouvre ; sinon elle est refusée et le panneau le dit. Sous
  > Windows, le tube est créé en première instance exclusive. Le relais vérifie que le serveur du
  > canal appartient à l'utilisateur courant avant d'envoyer quoi que ce soit.

## Points mineurs

- **Origine au journal.** AD-11 fixe le contenu d'une ligne ; rien ne dit si un appel externe y est
  distingué. Proposer : champ `origin` (`panel`, `plan`, `external`), ignoré par le rejeu, pour que
  l'utilisateur sache après coup ce que l'IA a fait.
- **Couches intermédiaires (5.22).** `intermediates` vit dans le `State` de la connexion de
  l'extension ; un appel externe qui consomme une sortie de plan ne la rend pas intermédiaire, et
  l'inverse. Tranché par T3 (un seul état par processus), à mentionner dans 5.24.
- **Interrupteur et changement de projet.** « Ne se rallume pas seul à la réouverture » : préciser
  que le changement de projet dans la même fenêtre de QGIS, qui ouvre une session neuve (AD-7),
  compte comme une réouverture — l'interrupteur repart éteint.
- **Fichiers hors projet.** Par `layer.add`, l'IA externe peut ouvrir n'importe quel fichier du
  poste (AD-9 l'accepte, « non portable ») puis en lire la description et l'envoyer à son
  fournisseur. Le consentement de l'interrupteur couvre peut-être ce cas ; à dire explicitement
  dans le bandeau ou à restreindre aux chemins déjà connus de la session, comme AD-23 le fait pour
  le modèle local.
- **AD-14.** La preuve « refuse un client distant » ne dit rien de la socket Unix : préciser que le
  dossier est créé en `0700` et la socket en `0600`, et que la recette le vérifie.

## Résumé des textes à porter dans la colonne

| Trou | AD touchée | Nature |
| --- | --- | --- |
| T1 | AD-7, AD-29 | `session.hello` du relais : version seule, n'ouvre rien |
| T2 | AD-6, AD-29 | `external.call` (notification) et `external.approve` (requête) nommées |
| T3 | AD-4, AD-29 | un seul état occupé par processus, lu sur les deux connexions |
| T4 | AD-13, AD-11 | mode choisi par appel ; l'autonome n'ouvre aucun journal |
| T5 | AD-2, AD-9 | deux noms de canal ; aucun chemin absolu sur le canal d'IA externe |
| T6 | AD-7, AD-29 | message de version nommant le relais ; schémas venus de la session |
| T7 | AD-29 | `tools/list_changed` à chaque bascule |
| T8 | AD-2, AD-12, AD-29 | cycle de vie des deux déconnexions |
| T9 | AD-12, AD-29 | identifiants par connexion ; annulation par le panneau |
| T10 | AD-4, AD-29 | validation non bloquante, bornée, annulable |
| T11 | AD-21 | traduction unique vers MCP |
| T12 | AD-29 | nom périmé, première instance, propriétaire vérifié |
