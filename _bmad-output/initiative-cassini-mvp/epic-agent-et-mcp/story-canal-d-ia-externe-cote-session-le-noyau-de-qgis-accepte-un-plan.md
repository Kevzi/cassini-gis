---
title: "Canal d'IA externe côté session : le noyau de QGIS accepte un relais"
type: 'feature'
ticket: '24'
created: '2026-10-30'
status: 'done'
baseline_revision: 'e484a232d66eb8fe3fa26a833351e6bc2d59480c'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Une IA externe ne peut pas joindre la session QGIS ouverte : `cassini serve` n'écoute que l'extension (stdio), et aucun canal local d'IA externe n'existe (AD-2, AD-14, AD-29).

**Approach:** Quand l'extension l'allume (`session.external`), le noyau de la session ouvre le canal d'IA externe — tube nommé Windows, socket Unix en dossier 0700 — le ferme à l'extinction ou à l'arrêt, y accepte des relais et répond à leur `external.attach` par le catalogue disponible ; le protocole passe à 2 des deux côtés. L'exécution des appels du relais est le ticket suivant (voir Decisions).

## Boundaries & Constraints

**Always:**
- Canal éteint par défaut, jamais rallumé seul ; fermé à `session.external {enabled:false}`, à la fin de stdin, à l'arrêt ; les relais branchés reçoivent alors la notification `external.session_lost` avant fermeture.
- Le noyau ne bloque jamais sur un relais (AD-4) : un fil lecteur et un fil écrivain par connexion, les lignes lues alimentent la file unique de la boucle avec leur source.
- Aucun chemin dans aucun message du canal d'IA externe (AD-9).
- Sans tokio, sur des fils bloquants (AD-14). Unix : `std::os::unix::net`, dossier propre à l'utilisateur en 0700 (mode et propriétaire vérifiés), jamais l'espace abstrait, socket orpheline supprimée avant d'ouvrir. Windows : `windows-sys`, `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` sur la première instance, descripteur réservé à l'utilisateur courant, client en `SECURITY_SQOS_PRESENT` ; AF_UNIX interdit.
- Erreurs à la forme AD-21 ; tous les tests existants de `serve`, `hors_ligne.rs`, `cargo deny check` et la recette pytest restent verts.

**Never:**
- Ni TCP, ni port, ni tokio, ni dépendance autre que `windows-sys`.
- Pas d'exécution de commande venue d'un relais (ticket suivant) ; rien dans `cli/src/mcp.rs` (5.25) ni dans le panneau (5.26) hors constante de protocole.
- Aucune méthode hors de la liste fermée de AD-6.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Défaut | session ouverte, rien d'allumé | aucun tube ni socket n'existe | — |
| Allumage | `session.external {enabled:true, ask:false}` | canal ouvert ; réponse `{enabled:true, ask:false}` ; `ask` gardé pour le ticket suivant | avant `session.hello` : `session.not_started` ; paramètres manquants : paramètres invalides |
| Extinction | `session.external {enabled:false}` | relais notifiés `external.session_lost`, connexions et canal fermés, socket supprimée ; réponse `{enabled:false}` | déjà éteint : même réponse |
| Seconde session | un autre noyau tient le canal | rien n'est ouvert | `external.channel_taken` |
| Poignée de main | `external.attach {protocol_version:2}` | `{protocol_version, core_version, commands}` : commandes disponibles (AD-25), sans chemin | version ≠ 2 : `session.protocol_mismatch` nommant le binaire à mettre à jour |
| Pendant un ordre | `external.attach` arrive pendant un ordre en vol | répondu sans attendre la fin de l'ordre | — |
| Autre méthode du relais | `command.run` etc. | rien n'est exécuté | méthode inconnue (jusqu'au ticket suivant) |
| Fin de stdin | QGIS fermé | relais notifiés, canal fermé, socket supprimée, sortie code 0 | — |
| Orpheline | socket Unix laissée par un noyau mort | supprimée, canal ouvert | — |
| Distant | connexion par `\\localhost\pipe\…` | refusée | — |

## Decisions (propriétaire, 2026-10-30)

- `windows-sys` (MIT OR Apache-2.0) acceptée comme dépendance directe du noyau, compilée seulement sous Windows.
- Ticket coupé en deux : celui-ci garde transport, cycle de vie, `external.attach`, protocole 2. L'exécution des appels du relais (occupation, `external.call`, `external.approve`, chemins refusés, origine au journal, `session_lost` en plein appel) part au report (`deferred-work.md`), à créer en ticket avant 5.25.
- Pour cette suite : un plan proposé non tranché occupe la session pour le relais seulement ; l'extension garde le droit de le remplacer.

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` -- boucle unique : `Msg` (l.77) gagne la source (`Extension` / `External(conn)`) et la fermeture d'une connexion ; `handle_line`/`dispatch` (l.333-379) aiguillent par source ; `Channel::execute` (l.485) ne prend une `Reply` que de l'extension et répond aux messages des relais par `answer_during_order` (l.447) ; la fin de stdin (l.220) ferme le canal. `hello` (l.562) inchangé hors version.
- `core/src/serve/external.rs` (nouveau) -- `session.external`, `external.attach`, table des connexions (émetteur vers le fil écrivain), notification `external.session_lost`.
- `core/src/external/` (nouveau) -- transport : `Listener` (ouvrir, accepter dans un fil, fermer en se réveillant par une connexion à soi-même) et `Stream` lecture/écriture depuis deux fils ; `unix.rs`, `windows.rs` (E/S chevauchées : un `ReadFile` synchrone bloque le `WriteFile` du même handle) ; nom du canal et `connect()` client, que 5.25 réemploiera.
- `core/src/serve.rs:627` -- `catalogue(state)` : réemployer, filtré sur `available`.
- `core/src/registry.rs:16` -- `PROTOCOL_VERSION` 1 → 2 (journal, `system.about`, `hello` suivent).
- `qgis-plugin/cassini_qgis/client.py:20` -- `PROTOCOL_VERSION = 2` ; tests qui figent 1 : `test_tracer.py:125-136`, `test_plan_client.py:43,333`.
- `cli/tests/hors_ligne.rs` -- garde PE ; ne pas modifier.
- `core/Cargo.toml` -- `windows-sys = "0.61"` sous `[target.'cfg(windows)'.dependencies]` (0.61.2 déjà au lock).
- `docs/canal-local.md` -- table des erreurs (l.239) et section du canal d'IA externe.

## Tasks & Acceptance

**Execution:**
- [x] `core/Cargo.toml`, `Cargo.lock` -- `windows-sys` (features tubes, fichiers, sécurité, E/S, fils) avec la ligne de licence.
- [x] `core/src/external/{mod,unix,windows}.rs` -- transport, nom du canal, `connect()`, orpheline, `channel_taken`, tests unitaires du dossier 0700.
- [x] `core/src/registry.rs`, `qgis-plugin/cassini_qgis/client.py` et ses tests -- protocole 2.
- [x] `core/src/serve.rs`, `core/src/serve/external.rs` -- sources multiples, `session.external`, `external.attach`, cycle de vie.
- [x] `core/tests/canal_externe.rs` -- extension simulée (flux en mémoire, nom de canal propre au test) et client brut du canal : chaque ligne de la matrice ; sous Windows, `\\localhost\pipe` refusé.
- [x] `docs/canal-local.md` -- messages, erreurs, cycle de vie, nom du canal.

**Acceptance Criteria:**
- Given une session ouverte sans interrupteur, when un client cherche le canal, then il ne le trouve pas, et les tests existants de `serve` passent.
- Given le canal allumé puis éteint, when un client brut était attaché, then il a reçu le catalogue puis `external.session_lost`, et le canal n'existe plus.
- Given la recette sous Linux et Windows, when `cargo test -p cassini-core`, `hors_ligne.rs`, `cargo deny check` et pytest tournent, then ils passent.

## Implementation Notes

- `serve.rs` : `Msg::Line(Source, String)` et `Msg::Detached(conn)` ; `Link::side` sert les relais dans la boucle comme dans `Channel::execute`. Le hub tient une entrée de la file, qui ne se ferme donc plus avec stdin : drapeau `Link::input_closed`, posé quand la fin est lue pendant un ordre et regardé avant chaque attente. La fermeture du canal (`session_ended`) suit la boucle, quelle que soit l'issue, et `Drop` du hub la refait au besoin.
- `serve/external.rs` : table des connexions partagée avec le fil d'acceptation, verrouillée pendant l'inscription d'une connexion (une réponse à la première ligne du relais trouve toujours sa connexion). Fermeture : `external.session_lost` remis par le fil écrivain, au plus 500 ms (`FAREWELL`), puis coupure et attente des deux fils, qui rendent aussitôt la main ; le fil d'acceptation est réveillé par une connexion à soi-même et attendu, si bien que la socket est retirée et le tube libéré avant la réponse (le canal se rouvre aussitôt, `FILE_FLAG_FIRST_PIPE_INSTANCE` compris).
- `session.external` est servi aussi pendant un ordre en vol (il n'exige pas l'exécutant). Erreur d'ouverture autre que `external.channel_taken` : `external.io`, sans chemin.
- Windows : descripteur `O:<SID>D:P(A;;GA;;;<SID>)` (propriétaire posé en plus du DACL du plan) ; `connect()` vérifie ce propriétaire avant de parler, contre un nom de tube pris par un autre compte. Chaque lecture ou écriture chevauchée attend aussi l'événement d'arrêt de la connexion.
- Variable de test : `CASSINI_EXTERNAL_TAG` (suffixe du nom) ; les tests d'intégration passent le nom par `serve_with_channel` + `ChannelName::tagged`, une variable d'environnement n'isolant pas des tests parallèles du même processus.
- `test.qgis_probe` n'existe que sous `cfg(test)` du crate : `canal_externe.rs` prend `layer.describe` comme commande déléguée.

## Plan Change Log

## Review Triage Log

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), 2026-10-30.

| # | Lentille | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|---|
| 1 | BH, ECH, VG | `Hub::close` attend jusqu'à 500 ms par relais, l'un après l'autre : N relais muets bloquent la boucle N × 500 ms (doc : « au plus une demi-seconde ») ; aucun test n'a de relais qui ne lit plus | medium | `Conn::close` fait `recv_timeout(FAREWELL)` dans la boucle `for conn in conns` de `Hub::close` | patch |
| 2 | ECH | `wake()` en échec (dossier modifié, fichier sonde impossible en tmp, tube occupé) → `open.accept.join()` ne rend jamais la main : extinction ou fin de session figées | medium | `Hub::close` ignore l'erreur de `wake` puis joint le fil bloqué dans `accept` | patch |
| 3 | ECH, VG | `detached` attend aussi jusqu'à 500 ms sur l'écrivain d'un relais parti | low | `Conn::close(None)` passe par le même `recv_timeout` ; correction directe : ne pas attendre sans adieu | patch |
| 4 | BH | Branche `Msg::Detached` de la boucle principale qui termine la session | low | inatteignable aujourd'hui (`side` consomme `Detached`), mais un relais qui part finirait la session QGIS si le routage change ; correction directe | patch |
| 5 | VG | Les tests Windows du canal (tube, refus distant, réveil) ne tournent jamais en CI | medium | `recette.yml` : seul `hors-ligne-windows` est sous Windows et ne lance que `hors_ligne` | patch |
| 6 | VG, IA | Le chemin de production (`ChannelName::for_user`, `channel: None`) n'est exercé par aucun test | medium | tous les tests passent par `serve_with_channel` ; `cli/src/main.rs` appelle `serve_with_composer` | patch (test bout en bout du binaire, `CASSINI_EXTERNAL_TAG`) |
| 7 | BH, ECH | Course entre deux sessions sur une socket orpheline : l'une supprime la socket neuve de l'autre ; `Drop` supprime sans vérifier | low | réel mais demande deux allumages simultanés avec orpheline ; correctif = verrou de fichier | rejeté (peu probable, ajoute de la complexité) |
| 8 | BH, VG | La sonde « canal pris » d'Unix crée une connexion éphémère dans l'autre session | low | acceptée puis `Detached` aussitôt, sans effet | rejeté |
| 9 | BH, ECH | Lignes, files et connexions non bornées pour un relais | low | le canal n'admet que l'utilisateur courant (0700 / ACL), qui a déjà tous les droits ; correctif = gardes nouvelles | rejeté |
| 10 | BH | Pas d'état « attaché » par connexion | false | aucune autre méthode n'est servie dans ce ticket ; à concevoir avec l'exécution des appels (report) | rejeté |
| 11 | BH | `CASSINI_EXTERNAL_TAG` lue en production | low | variable prévue par le plan (Design Notes) ; un utilisateur ne la pose pas | rejeté |
| 12 | BH, ECH | Nom de tube Windows tiré de `USERNAME` ; un autre compte homonyme donne un `channel_taken` trompeur | low | nom fixé par le plan et AD-14 ; homonymes de domaine sur un même poste rares | rejeté |
| 13 | BH | Test du refus distant peut passer pour une autre raison | low | le test observé sous Windows donne « Accès refusé », compatible avec `PIPE_REJECT_REMOTE_CLIENTS` ; pas de moyen simple de distinguer | rejeté |
| 14 | BH | Adieu perdu sous Windows sans `FlushFileBuffers` | false | `switching_on_then_off…` et `closing_the_input…` lisent `external.session_lost` et passent sous Windows ; le handle est fermé sans `DisconnectNamedPipe` | rejeté |
| 15 | BH | Longueur de `sun_path` non vérifiée | low | chemin Linux sous 108 octets (tag ≤ 40) ; macOS hors cible | rejeté |
| 16 | BH | Trous de test : extinction pendant un ordre, panne de sortie, catalogue figé, test de `mod.rs` dans le vrai `XDG_RUNTIME_DIR` | low | comportements simples, chemin `break` → `close` commun ; le test de `mod.rs` utilise un nom marqué | rejeté |
| 17 | BH, IA | L'extension n'envoie pas `session.external` | false | l'intention fait agir le noyau ; « des deux côtés » ne vise que le protocole ; le panneau est le ticket 5.26 | rejeté |
| 18 | ECH | Zéro instance de tube si `create_instance` échoue après un `accept` | low | l'`accept` suivant recrée l'instance | rejeté |
| 19 | ECH | `connect_instance` en échec permanent fait tourner le fil toutes les 50 ms | low | panne système improbable, bornée en charge | rejeté |
| 20 | ECH | `XDG_RUNTIME_DIR` différent entre QGIS et le relais | low | les deux processus d'une même session ont le même environnement ; chemin fixé par le plan | rejeté |
| 21 | ECH | Noms de sonde d'uid prévisibles en `/tmp` | low | il faut deviner le pid ; l'échec ne fige plus rien après le n° 2 | rejeté |
| 22 | ECH | Rallumage alors que le fil d'acceptation est sorti | false | `accept_loop` ne sort que sur `stop`, posé seulement par `close`, qui retire `open` | rejeté |
| 23 | VG | `external.attach` attend pendant la composition d'un plan | false | la doc ne promet la réponse immédiate que pendant un ordre en vol, ce qui est vrai ; déjà signalé pour le ticket suivant | rejeté |

## Design Notes

Nom du canal : `\\.\pipe\cassini-<utilisateur>-external` ; Unix : `$XDG_RUNTIME_DIR/cassini/external.sock`, sinon `<tmp>/cassini-<uid>/external.sock` (uid lu sur un fichier qu'on vient de créer, sans libc). Une variable de test, lue seulement si elle est posée, isole le nom par test. Windows : descripteur `D:P(A;;GA;;;<SID du jeton>)` par `advapi32`. Unix : la connexion à une socket existante échoue (refusée) → orpheline supprimée ; elle aboutit → `external.channel_taken`.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` (Linux) puis sous Windows (PowerShell, `CARGO_TARGET_DIR=C:\ct`) -- expected: tout vert, `canal_externe` compris
- `cargo test -p cassini --test hors_ligne` sous Windows -- expected: vert
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo deny check` -- expected: ok
- `python -m pytest qgis-plugin/tests` -- expected: vert
