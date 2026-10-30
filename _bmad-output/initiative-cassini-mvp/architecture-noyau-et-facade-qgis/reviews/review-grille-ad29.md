---
type: review
lens: grille (bon squelette d'architecture)
target: architecture-noyau-et-facade-qgis.md — amendement du 2026-10-30 (AD-2, AD-13, AD-14, AD-29, schéma d'ensemble, ligne CAP-8)
source: ../../change-ia-externe-pilote-la-session-qgis-ouverte/change-ia-externe-pilote-la-session-qgis-ouverte.md
date: 2026-10-30
verdict: à reprendre avant 5.24
---

# Relecture à la grille — amendement AD-29 (IA externe par relais)

## Verdict

**À reprendre avant de lancer 5.24.** La direction est juste et fidèle aux décisions du
propriétaire (relais, interrupteur éteint par défaut, case « Me demander avant chaque
appel », aucun fournisseur nommé). Mais l'amendement laisse ouverts trois points où 5.24
et 5.25 divergeraient à coup sûr : **la poignée de main du relais**, **les messages du
canal de l'extension** (interrupteur, fil d'appels, validation), et **le moment où `mcp`
choisit son mode**. Il contredit aussi deux textes du squelette (AD-6, liste fermée ;
Deferred, « socket locale reportée en V2 ») et laisse une incompatibilité entre AD-14 et
la façon naturelle d'écrire le canal.

Les décisions du propriétaire ne sont pas rediscutées ici. Le seul point qui demande une
décision nouvelle est signalé comme tel (C6, les chemins).

## Grille

| Critère | Tenu ? | Où |
| --- | --- | --- |
| Fixe les points de divergence de 5.24, 5.25, 5.26 | Partiellement | C1, C2, C3, C4 |
| Chaque règle est vérifiable et empêche sa divergence | Partiellement | C1 (« aucune méthode nouvelle » faux), C5 (preuve « refuse un client distant » floue) |
| Rien dans Deferred ne laisse diverger | Non | C7 (« socket locale » contredit) |
| Aucune contradiction avec une autre AD | Non | C1 (AD-7), C2 (AD-6), C7 (AD-2 Prevents, AD-3, AD-5, Deferred) |
| Couvre la proposition de changement | Presque | appels non bloquants (point 6 du changement) absents : C4 |
| Cycle de vie du canal | Partiellement | C3, C5 (socket orpheline) |
| Sécurité et consentement | Partiellement | C5, C6 |
| Concurrence | Partiellement | C4 |
| Erreurs (AD-21) | Silencieux | C8 |
| Annonce de version (AD-7) | Silencieux | C1, C2 |
| Recette et tests | Partiellement | C9 |

## Constats

### C1 — `session.hello` sur le canal du relais contredit AD-7 et le code (bloquant)

AD-29 dit que le relais emploie « les seules méthodes existantes (`session.hello`,
`command.run`, `request.cancel`) ». Or AD-7 définit `session.hello` comme **la requête de
l'extension**, qui porte la liste des gestes qu'elle sait exécuter et le chemin absolu du
projet, et qui **ouvre** la session (« ouvrir un autre projet termine la session »). Le
relais ne connaît ni l'un ni l'autre. Le code actuel (`core/src/serve.rs`, `hello`) exige
`commands` et `project`, ouvre `Session::open` et refuse un second appel en
`session.already_started`. 5.24 devra donc inventer en route ce que signifie un `hello`
venu du relais : c'est exactement la divergence que le squelette doit fermer.

**Correctif proposé.** Ajouter à AD-29 une puce « Poignée de main » :
> Sur le canal d'IA externe, le premier message est une requête du relais,
> `session.hello`, qui porte `protocol_version` et `role: "external"`, sans `project` ni
> `commands`. Le noyau n'ouvre ni session ni journal : il répond par sa version de
> protocole, sa version et le catalogue **disponible** de la session déjà ouverte, jamais
> le chemin du projet (AD-9). Version différente : `session.protocol_mismatch`, avec un
> message qui nomme le binaire à mettre à jour (celui que le client MCP lance, ou le paquet
> du noyau) — les deux peuvent différer quand l'utilisateur a deux installations.

(Variante équivalente : une méthode `session.attach` distincte, déclarée dans la liste de
AD-6. L'important est que l'un des deux soit écrit.)

### C2 — « Aucune méthode nouvelle » est faux du côté de l'extension (bloquant)

La règle ne vaut que pour le canal du relais. Pour tenir AD-29, le canal de l'extension
doit porter au moins trois échanges qui n'existent pas dans la liste fermée de AD-6
(précision du 2026-10-18 : « Ajouter une méthode demande d'amender ce contrat ») :

1. **l'interrupteur** : l'extension dit au noyau d'ouvrir ou de fermer le canal, et
   apprend s'il est refusé parce qu'une autre session le tient ;
2. **le fil d'appels** : le noyau dit à l'extension qu'un appel externe commence et ce
   qu'il rend (commande, paramètres, résultat ou erreur, durée) — sans quoi 5.25 n'a rien
   à afficher ;
3. **la validation** : avec la case cochée, le noyau demande à l'extension l'accord de
   l'utilisateur et attend la réponse.

Sans ces noms, 5.24 (noyau) et 5.25 (panneau) les inventeront chacun de leur côté.

**Correctif proposé.** Amender la précision de AD-6 et AD-29 :
> Liste fermée étendue (2026-10-30) : `session.external` (extension → noyau, `{enabled}` ;
> rend l'état, ou `external.channel_taken`), notification `external.call` (noyau →
> extension, `{call_id, command, params, phase: started|finished, result|error,
> duration_ms}`), requête `external.confirm` (noyau → extension, rend `{accepted}` ; un
> refus rend `external.refused` au relais). Aucune n'est offerte au modèle ni à un client
> MCP.

Et trancher la version (AD-7) : un noyau v1 répondrait `METHOD_NOT_FOUND` à
`session.external`. Le plus simple, conforme à AD-7, est **protocole 2**, refus net en cas
d'écart, et la ligne « Protocole Cassini » du tableau de pile mise à jour.

### C3 — Le moment où `mcp` choisit relais ou autonome n'est pas fixé (majeur)

AD-13 dit « relais quand une session a ouvert le canal, autonome sinon », sans dire
**quand** on regarde. Les clients MCP lancent `cassini mcp` à leur propre démarrage, le
plus souvent avant QGIS ; l'utilisateur allume l'interrupteur plus tard, ferme QGIS,
rouvre un autre projet. Une décision prise une fois au lancement rendrait le relais
inutilisable dans le cas courant ; une décision par appel sans règle ferait basculer un
appel vers l'autonome en silence.

**Correctif proposé.** Puce « Cycle de vie » dans AD-29 :
> Le relais cherche le canal à chaque `tools/list` et à chaque `tools/call`. Un changement
> de mode ou de catalogue (canal ouvert ou fermé, projet rouvert, donc session neuve)
> envoie `notifications/tools/list_changed` au client. Un appel commencé en relais ne
> finit jamais en autonome : canal perdu en cours d'appel, il rend `external.session_lost`.
> Le mode courant est dit au client (instructions de `initialize` et texte de
> `system.about`), comme la proposition le demande (« le dit à l'IA »).

### C4 — Concurrence : plusieurs clients, symétrie du « un à la fois », appels non bloquants (majeur)

- **Plusieurs relais.** 5.26 fait jouer Claude, GPT et Gemini ; rien ne dit si deux
  clients branchés en même temps sont admis. Proposer : plusieurs connexions admises,
  appels sérialisés par la session, `session.busy` pour le second.
- **Symétrie.** AD-29 refuse un appel externe pendant un plan, mais pas l'inverse : une
  demande tapée dans le panneau pendant un appel externe. Et « pendant un plan » est
  ambigu (plan proposé et en attente de validation, ou seulement en cours d'exécution ?).
  Proposer : « De `plan.propose` à la fin de `plan.execute` ou `plan.reject`, et pendant un
  appel externe, toute autre demande, d'où qu'elle vienne, reçoit `session.busy`. »
- **Une seule boucle.** Écrire que les lignes du canal externe entrent dans la même file
  que celles de l'entrée standard, et que la boucle principale reste seule à écrire et à
  journaliser (c'est la structure actuelle de `serve.rs`). Sinon 5.24 peut être tenté
  d'ouvrir un second fil d'exécution sur la session.
- **Appels non bloquants** (point 6 de la proposition, report de 5.7) : absent du
  squelette. Ajouter : « AD-4 vaut pour le relais : pendant un appel, il continue de
  servir `ping` et l'annulation du client ; `notifications/cancelled` du client devient
  `request.cancel` sur le canal. » Préciser aussi que l'attente de validation (case
  cochée) peut dépasser le délai d'un client MCP, et ce que rend alors le noyau.

### C5 — AD-14 interdit la voie naturelle du canal, et la preuve de localité est floue (majeur)

- `tokio` est admis sans `net` ; or `tokio::net::UnixListener` et
  `tokio::net::windows::named_pipe` demandent précisément `net`. 5.24 se heurtera à
  `cargo-deny`, puis sera tenté d'assouplir la règle. Écrire la voie : « le canal est
  écrit avec `std::os::unix::net` et l'API Win32 des tubes nommés, sur des fils
  bloquants ; `net` de tokio reste interdite ». La crate d'accès Win32 (`windows-sys`,
  MIT OR Apache-2.0) est à valider avec sa licence au ticket (CLAUDE.md §8).
- Interdire explicitement **AF_UNIX sous Windows** : il passe par `ws2_32.dll`, que
  `cli/tests/hors_ligne.rs` refuse — mieux vaut le dire avant que quelqu'un le choisisse
  pour avoir un seul code.
- « Un test montre qu'il refuse un client distant » n'est pas vérifiable tel quel sur une
  machine de recette. Proposer des critères précis : Windows — tube créé avec
  `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` et une liste d'accès
  restreinte au SID de l'utilisateur, vérifiés par le test, plus une connexion par
  `\\localhost\pipe\…` (chemin distant) qui échoue ; Unix — dossier en 0700, socket en
  0600, vérifiés.
- **Socket orpheline.** Sous Unix, un noyau tué laisse le fichier de socket : la règle
  « une seconde session est refusée » refuserait alors à tort. Écrire : « un canal dont la
  connexion échoue est orphelin ; il est supprimé et recréé ». (Noter aussi la limite de
  104 octets du chemin sous macOS pour le choix du dossier.)

### C6 — Chemins et données lisibles par l'IA externe (moyen ; décision du propriétaire à demander)

L'exception de AD-9 (chemins absolus sur le canal local) a été écrite pour l'extension.
Rien ne dit qu'elle ne vaut pas pour le canal externe, ni quels fichiers une IA externe
peut faire ouvrir : `layer.add` sur un chemin quelconque du poste, dont le contenu
repart ensuite vers le fournisseur par le réseau du client. AD-23 n'excepte le serveur
MCP que pour les points inventés.

**Correctif proposé.** Deux phrases dans AD-29 :
> L'exception de AD-9 ne vaut pas pour le canal externe : il ne porte que des chemins
> relatifs au projet, résultats et erreurs compris. *(À décider par le propriétaire :)*
> un appel externe ne nomme que des fichiers de l'arborescence du projet ou déjà connus de
> la session ; sinon `external.path_refused`.

### C7 — Contradictions de texte avec le reste du squelette (moyen)

- **Deferred, « Socket locale au lieu de l'entrée/sortie standard. Reportée en V2 ».** Il
  existe désormais un canal local de ce genre. Reformuler : « le canal de l'extension reste
  l'entrée/sortie standard ; seul le canal d'IA externe (AD-29) est local ».
- **AD-2, Prevents « deux transports concurrents ».** Il y en a maintenant deux. Préciser :
  « deux transports pour le même interlocuteur ».
- **AD-3.** Dire le sens du canal externe : le relais demande, le noyau répond, aucun ordre
  du noyau vers le relais (même posture que le serveur MCP).
- **AD-5** (« recueille la validation de l'utilisateur ») : un appel externe s'exécute
  sans validation par défaut. C'est la décision du propriétaire ; il faut l'écrire comme
  écart dans AD-5 ou AD-29 (« un appel externe est une étape unique, sans plan ; la
  validation n'est demandée que si la case est cochée »), sinon un relecteur de 5.24
  pourra la tenir pour une infraction.
- **AD-25 / catalogue.** « Celui que la session annonce » : préciser que le relais
  n'expose que les commandes **disponibles** (le catalogue de `hello` porte aussi les
  indisponibles, champ `available`). Et AD-12 : une commande non annulable est montrée
  comme telle dans le fil, sans bouton Arrêter actif.

AD-1, AD-11, AD-13 (règle), AD-15, AD-16 et AD-23 restent tenues : le relais est dans le
binaire Apache-2.0, il ne lie rien de QGIS, la session reste seul auteur du journal, et la
règle « une seule session » s'accorde avec le verrou exclusif de AD-11.

### C8 — Erreurs : forme côté MCP et codes nouveaux non fixés (moyen)

AD-21 fixe l'enveloppe sur le canal, pas sa traduction côté MCP. 5.7 rend une erreur de
commande en résultat d'outil `isError` avec un texte (`cli/src/mcp.rs`). Le relais doit
faire **pareil**, sinon un même échec se lit autrement selon le mode. Proposer : « une
erreur de commande ou de session devient un résultat d'outil `isError` dont le contenu
structuré porte `code` et `message` de AD-21 ; seule une panne du relais lui-même est une
erreur JSON-RPC ». Lister les codes nouveaux : `external.session_lost`,
`external.refused`, `external.channel_taken`, `external.path_refused` (si C6 retenu),
`request.cancelled` pour l'arrêt depuis le panneau.

### C9 — Recette (mineur)

AD-14 ne prévoit que la preuve de localité. Ajouter à la ligne « Tests » des conventions,
pour 5.24, des tests automatiques sans QGIS : relais + extension simulée (harnais de
`cli/tests/end_to_end.rs`) ; interrupteur éteint → aucun canal (connexion refusée) ;
réouverture → éteint ; seconde session refusée ; appel externe pendant un plan →
`session.busy` ; arrêt depuis le panneau → `request.cancelled` au client ; journal d'un
appel externe rejoué à l'identique. 5.26 garde la recette humaine à trois familles, en
phrases enchaînées dans une même session.

Optionnel : dire si une ligne de journal porte la provenance d'un appel (externe ou
panneau). Si oui, sous la clé réservée de AD-11 et hors comparaison au rejeu ; si non,
l'écrire, pour que 5.24 ne l'ajoute pas de lui-même.

## Ce qui est bien tenu

- La règle centrale — la session est seule à valider, dicter et journaliser — ferme
  bien le risque du second noyau et du journal à deux auteurs (AD-5, AD-11).
- Consentement : éteint par défaut, pas de rallumage seul, canal fermé avec la session.
- Aucun fournisseur nommé dans le code ; MCP comme seule langue du relais (CLAUDE.md §2.8).
- Ligne CAP-8 et schéma d'ensemble à jour ; source ajoutée dans l'en-tête.

## Ordre conseillé des correctifs

C1 et C2 d'abord (ils changent le protocole et sa version), puis C3, C4 et C5 (ils fixent
le travail de 5.24), C6 à soumettre au propriétaire, puis C7, C8 et C9 (texte et recette).
