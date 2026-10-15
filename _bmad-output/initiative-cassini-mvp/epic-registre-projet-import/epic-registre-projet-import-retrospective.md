---
epic: epic-registre-projet-import
date: 2026-10-15
verdict: rejected
criteria: declared
headless: false
---

# Rétrospective de l'epic 2 — Registre de commandes et journal rejouable

## Epic summary

Interactif. Le propriétaire a accepté de rétrospecter l'epic alors que 2.11 est inachevé (réponse du 2026-10-15), sans point à peser en particulier.

| Ticket | Titre | Statut | Plan, baseline |
|---|---|---|---|
| 2.1 | Traceur : crate `cassini`, `system.about` | built | `6a04be4` |
| 2.2 | Une commande sans toucher un fichier partagé | built | `8083c60` |
| 2.3 | Contrat complet contrôlé par le registre | built | `c8906bf` |
| 2.4 | Validation, erreurs typées, exécutant abstrait | built | `b543b01` |
| 2.5 | Journal en écriture, ajout seul, verrou | built | `a180454` |
| 2.6 | Rejeu par `cassini replay` | built | `5cb8555` |
| 2.7 | Canal local `cassini serve` | built | `7f55dd5` |
| 2.8 | Parité `run` / `serve` | built | `bedbd71` |
| 2.9 | Passe de nettoyage | built | `629d811` |
| 2.10 | Suite de bout en bout | built | `e30262b` |
| **2.11** | **Ordres du noyau vers l'extension et annulation** | **planned, non construit** | aucun plan |

- **Tickets au statut `built`** (finis par le build, pas encore `done`) : 2.1 à 2.10.
- **Tickets inachevés (`pending_tickets`)** : 2.11. Le propriétaire a accepté de poursuivre malgré cela.
- **Plage** : les baselines se suivent sans chevauchement, chaque plan va jusqu'à la baseline suivante ; le dernier (2.10) va jusqu'à `5cc2361`, le HEAD au moment de la rétro (plage déduite, non enregistrée). Plage totale `6a04be4..HEAD` : 16 commits dont 5 fusions (toutes mesurées), 38 fichiers.
- **Inventaire** : fichier de l'epic (Done when présent), plans 2.1 à 2.10, `tickets.toml`, spec et contrat d'architecture, `git_evidence.py`. Aucun fichier d'histoire (tous les tickets sont des entrées sans `story_file`). **Journaux de session absents** : l'analyse des leçons de processus tirées des sessions n'a pas été faite.
- **Hors périmètre de cette rétro** : le plan orphelin `plan-8-2-phrases-et-validateur.md` (signalé par `status`, appartient à l'epic 8).

## Findings

Méthode : trois sous-agents en lecture seule (vues agrégées, réconciliation à la spec, relecture du diff, lentilles adversariale, cas limites et trous de vérification), plus une vérification de comportement faite ici. Les constats marqués **vérifié** ont été relus par moi dans le code ; les autres sont des rapports de sous-agent non revérifiés (voir « Open questions »).

### A. Défauts qui touchent une exigence de l'epic

| # | Constat | Source | Disposition |
|---|---|---|---|
| F1 | **Ligne de journal tronquée** : l'ajout la tolère (ajoute un `\n`), mais `Replay::load` échoue en `journal.io: InvalidData` sur toute ligne illisible, sans nommer l'étape ; l'historique entier devient non rejouable (R5). Le rejet du triage 2.6 supposait un cas improbable que `append` prévoit pourtant. **Vérifié.** | `core/src/journal.rs:244-252`, `:480-483` ; triage 2.6 | À corriger |
| F2 | **Rejeu aveugle au contenu des gros résultats** : un résultat de plus de 200 octets est résumé en `{"truncated":true,"bytes":n}` ; le rejeu compare ce résumé, donc deux contenus de même longueur sont « identiques ». R5 est vide pour les futures commandes à gros résultat. **Vérifié.** | `journal.rs:602-609`, `:562-566` | À corriger avant l'epic 9 |
| F3 | **Entrées non contrôlées au rejeu** : `size` et `modified` sont écrits (`input_entry`) et jamais relus ; « entrée modifiée → écart détecté » (matrice 2.6) n'est vrai que si le résultat dépend de la taille. Le rejet du triage 2.6 contredit sa propre matrice. | `journal.rs:538-599` ; plan 2.6 l.49 | À corriger (non revérifié) |
| F4 | **Commande QGIS annoncée mais jamais exécutable** : `session.hello` met `available:true`, `command.run` passe par `QgisStub` → `registry.unavailable`, et cette erreur est **journalisée** comme une étape, que le rejeu refuse ensuite de rejouer. Même appel non annoncé : rien n'est écrit. **Vérifié** (stub, absence de `with_qgis` dans `serve`). Dépend de 2.11. | `core/src/serve.rs:237-239`, `:264-272` ; `journal.rs:226-229`, `:558-560` | À corriger avec 2.11 |
| F5 | **Rien dans le noyau pour l'annulation ni les ordres sortants** (AD-3, AD-4, AD-12) : `serve` ne lit que des requêtes, une réponse reçue donne `INVALID_REQUEST`. Promis par la note de l'epic (« livrés ici, entrée 11 »). | `serve.rs:129-143` ; epic, Notes | Livrer 2.11 |
| F6 | **`path_params` / `output_params` non contrôlés par le contrat** : aucune mention dans `contract.rs`. Une faute de frappe laisse un chemin absolu dans le journal, contre AD-9 ; le rejet du triage 2.5 tombe. **Vérifié** (grep négatif). | `contract.rs` ; `journal.rs:375-401` | À corriger |

### B. Aggregate views

| # | Vue | Constat | Source | Disposition |
|---|---|---|---|---|
| F7 | Duplication | Utilitaire de projet de test copié 5 fois (6 avec `basemap.rs`, antérieur à l'epic) ; `canonical` recodée dans `parity.rs` alors que `journal.rs:612` est publique | `journal.rs:713`, `serve.rs:282`, `cli/src/main.rs:249`, `cli/tests/parity.rs:17,36`, `cli/tests/end_to_end.rs:16` | À reporter (nettoyage) |
| F8 | Duplication | Déréférencement de `$ref` codé deux fois, gardes de profondeur 8 et 16 ; `ErrorReport` construit en littéral à deux endroits | `contract.rs:201`, `plan.rs:151` ; `serve.rs:45-51`, `cli/src/main.rs:123-127` | À reporter |
| F9 | Taille | `journal.rs` : 1121 lignes dont 702 de code, sept responsabilités ; couplage `Session`/`Replay` voulu (AD-11) ; chemins, `canonical`, `registry_version` séparables. Les autres fichiers sont de taille légitime. | `journal.rs` | À reporter ; le couper avant d'y ajouter 2.11 |
| F10 | Architecture | Cycle de modules `error` ↔ `journal` (`error.rs` importe `JournalError`, `journal.rs` importe `ErrorReport`). **Vérifié.** Aucune dépendance GPL, aucun lien vers QGIS ou `/ai`, sens `cli → core` respecté. | `error.rs:5-7`, `journal.rs:22`, `core/Cargo.toml` | Accepter ou reporter |
| F11 | Motifs | Quatre formes d'erreur (`RegistryError`, `JournalError`, `PlanError` en struct, `RpcError`) ; styles de noms de tests mêlés | `registry.rs:132`, `journal.rs:44`, `plan.rs:30`, `serve.rs:22` | À reporter |

### C. Diff-scope review (jointures)

| # | Constat (sous-agent, non revérifié sauf mention) | Source | Disposition |
|---|---|---|---|
| F12 | Une montée de version du noyau casse tout rejeu contenant `system.about` (`core_version` dans le résultat) sans le dire ; `versions` n'est jamais lu au rejeu | `journal.rs:230-237`, `cli/src/main.rs:213` | À reporter |
| F13 | `p.qgz` et `p.qgs` côte à côte partagent le même `p.cassini` (journal et verrou) ; `p.cassini` en lien symbolique écrit hors dossier de travail (AD-10) | `journal.rs:286-291` | À reporter |
| F14 | Rejeu : deux rejeux simultanés écrasent `replay/` ; lecture d'une ligne à moitié écrite par une session active | `journal.rs:520,539,473-483` | À reporter |
| F15 | Un journal livré avec un projet partagé est exécuté tel quel au rejeu (chemins absolus ou `..` acceptés, simple avertissement) ; sans effet avant le branchement de QGIS | `journal.rs:431-444,512-516,160` | À reporter, à traiter avant l'epic 9 |
| F16 | `serve` : ligne sans borne, octets invalides remplacés par U+FFFD (un chemin peut être altéré en silence) ; notifications sans `id` exécutées mais jamais acquittées ; `id:null` traité comme une requête | `serve.rs:76-80,120,144-150` | À reporter |
| F17 | `session.hello` sur un projet verrouillé rend un succès sans `mode` ; l'extension annonce « prêt » puis échoue à chaque commande | `serve.rs:226-234` | À reporter (AD-11 : « le dit explicitement ») |
| F18 | Windows : comparaison de chemins sensible à la casse, préfixe `\\?\` non géré | `journal.rs:669-688` | À reporter |
| F19 | Les messages d'erreur serde recopient la valeur fautive, donc possiblement un chemin absolu (AD-9) | `registry.rs:157-159,224-226` | À reporter |

### D. Trous de vérification et écarts de spec

| # | Constat | Source | Disposition |
|---|---|---|---|
| F20 | **Parité et bout en bout peu probants** : le binaire ne contient que `system.about` (sans paramètres) ; aucun chemin, sortie ou repère `out:` ne traverse `run`/`serve` ; la ligne de journal écrite par `run` et par `serve` n'est jamais comparée ; aucun chemin Windows à antislash. La couverture de 2.10 est connue et déclarée dans son plan. | `cli/tests/parity.rs:94-111`, `end_to_end.rs` ; plans 2.8, 2.10 | Accepter pour l'epic ; ajouter une commande `core` de production avec chemins dès l'epic 4 ou 9 |
| F21 | **« Toute exécution est journalisée »** (R4, CLAUDE.md §4) contre `cassini run` sans `--project`, qui n'écrit rien : l'énoncé est plus fort que le livré | `cli/src/main.rs:83-85` | Spec à réconcilier |
| F22 | **Deux formes de référence entre étapes sans pont** : `step:k` (2.4, `plan.rs`) n'est résolue par aucun chemin d'exécution ; `out:<k>.<n>` (2.6) ne vaut que pour les sorties de fichier ; la règle 9 de `docs/regle-de-redaction-des-commandes.md` ne décrit que `step:k` | `plan.rs` ; `journal.rs:~415` ; docs règle 9 | À trancher avant l'epic 5 ou 9 |
| F23 | `WriteOutputParams.out` figure au schéma des paramètres alors que la règle 6 interdit un nom de sortie dans les paramètres ; le catalogue le montrerait au modèle | `test_commands.rs:~58` ; docs règle 6 | À reporter (aujourd'hui sous `cfg(test)`) |
| F24 | **Ordre des tickets** : `tickets.toml` met 2.11 avant 2.9 (`9.after` contient 11), mais 2.9 et 2.10 ont été commités sans 2.11 | `tickets.toml` ; commits `e30262b`, `5cc2361` | Leçon de processus |

Ajouts non spécifiés mais sans violation d'AD : dossier `replay/` jamais vidé, `journal.lock` séparé du journal, refus de `session.hello` pour un chemin de projet relatif.

## Behavior verification

Exercé de bout en bout sur le dépôt courant :

- `cargo test -p cassini-core -p cassini --locked` : **93 tests passent**, 0 échec.
- Binaire `cassini` : deux `run system.about --project` (résultats identiques), puis `replay` → `{"steps":2,"identical":true}` ; journal `journal.jsonl` texte, une ligne JSON par étape, sans chemin absolu, avec `journal.lock` et `replay/` à côté.
- `serve` : un `session.hello` avec un chemin non absolu est refusé par `-32602` (comportement documenté, F-ajout).
- Non exercé : toute commande déléguée à QGIS (aucun exécutant réel), un journal tronqué, un rejeu concurrent, Windows en conditions réelles de casse de chemin.

## Previous-retro follow-through

Rétrospective précédente trouvée : `epic-epreuve-du-modele-local/epic-epreuve-du-modele-local-retrospective.md` (epic 8).

| Action | Propriétaire | Landed ? |
|---|---|---|
| A1 : mesure courte câble débranché | Propriétaire | Écartée par le propriétaire (le fichier le dit) |
| A2 : ranger les plans 8.3 à 8.6 dans le dossier de l'epic | Propriétaire, `bmad-ticket` | Appliquée pour 8.3 à 8.6 (`status` les voit dans l'arbre). Reste ouverte pour la partie 2 de 8.2 : `status` signale encore `plan-8-2-phrases-et-validateur.md` à la racine |
| A3 : corriger le README de l'épreuve | Dev | Le fichier dit « appliqué » ; je n'ai pas revérifié le README, donc *aucune preuve trouvée* de mon côté |
| A4 : aligner `run_forms.py` | Dev | Idem : annoncé appliqué, non revérifié |
| A5 : rappeler à l'epic 5 AD-15 et la mesure sur poste modeste | Propriétaire | *Aucune preuve trouvée* (l'epic 5 n'a pas démarré) |
| A6 : leçon de processus (recalcul des chiffres, plans dans le dossier de l'epic) | Propriétaire | Plans de l'epic 2 écrits dans le dossier de l'epic dès la création : **oui**. Recalcul des chiffres par script : *sans objet*, aucun ticket de l'epic 2 ne produit de chiffres |

## Action items

Proposés, non appliqués. Propriétaire de chaque action : le propriétaire décide de l'exécution.

| # | Action | Constats | Propriétaire |
|---|---|---|---|
| R1 | **Construire 2.11** (ordres noyau→extension, annulation, état « annulée » au journal) ; y inclure F4 (une commande annoncée doit s'exécuter ou être refusée avant écriture au journal) et F17 (`mode` dans la réponse de `session.hello`) | F4, F5, F17 | Dev, `bmad-build 2.11` |
| R2 | Rendre le rejeu robuste : nommer l'étape d'une ligne illisible, comparer les gros résultats par empreinte, relire `size`/`modified` des entrées | F1, F2, F3 | Dev (ticket de correction, avant l'epic 9) |
| R3 | Ajouter au contrat (`contract.rs`) un contrôle de `path_params` et `output_params` contre le schéma des paramètres | F6 | Dev |
| R4 | Découper `journal.rs` (chemins et `canonical` à part) avant d'y toucher pour 2.11 ; mutualiser le projet de test et `canonical` ; éventuellement sortir `error` du cycle | F7, F8, F9, F10 | Dev (passe de nettoyage) |
| R5 | Réconcilier la spec : énoncé de R4 (« toute exécution » contre `--project` facultatif), forme unique de référence entre étapes (`step:k` contre `out:`), et la règle 9 de la documentation | F21, F22 | Propriétaire, avec `bmad-architecture` |
| R6 | Avant l'epic 9 : trancher la confiance accordée à un journal reçu d'ailleurs (F15) et la borne d'une ligne sur `serve` (F16) | F15, F16 | Propriétaire |
| R7 | Leçon de processus : tenir l'ordre `after` des tickets ; ne pas construire 2.9 (nettoyage) et 2.10 (suite finale) avant 2.11 qu'ils attendent | F24 | Propriétaire |
| R8 | Reporter F12, F13, F14, F18, F19, F23 dans `deferred-work.md` | F12-14, F18, F19, F23 | Dev |

## Acceptance verdict

**Rejeté** (verdict machine), critères **déclarés** (Done when de l'epic).

- **Raison déterminante** : le ticket **2.11** est inachevé, ce qui force le verdict machine à « rejeté » (règle 3). Il porte les ordres du noyau vers l'extension et l'annulation (AD-3, AD-4, AD-12) que la note de l'epic promet de livrer ici.
- **Done when**, sur ce qui est construit :
  1. Contrat complet : **tenu** (`contract.rs`, `the_real_registry_respects_the_contract`).
  2. Aucun fichier partagé : **tenu** (`build.rs`, `inventory`, diff `d1e89cc`).
  3. Même résultat par CLI et canal : **tenu pour les commandes `core`**, non démontrable pour les commandes `qgis` sans 2.11 (F4, F20).
  4. Rejeu identique après fermeture et réouverture : **tenu mais faible** (F1, F2, F3 ; scénario limité à `system.about`).
  5. Journal lisible à côté du projet : **tenu**.
- Aucun défaut ne remet en cause R1, R2 ni le périmètre civil ou la licence du noyau (aucune dépendance GPL, noyau sans QGIS).
- Une décision humaine l'emporte : le propriétaire peut passer ce verdict à « accepté avec points ouverts » une fois 2.11 construit et R2 appliqué.

## Open questions

1. Faut-il construire 2.11 avant d'ouvrir l'epic 9, ou accepter que l'epic 9 invente les ordres sortants (la note de l'epic dit non) ?
2. Le rejeu doit-il refuser, sauter ou réparer une ligne tronquée (F1) ? Choix de politique, pas de technique.
3. Les constats F3 et F12 à F19 viennent d'un sous-agent et n'ont pas été revérifiés par moi ; à confirmer avant de les transformer en tickets.
4. Sessions de build : aucun journal de session disponible, donc aucune leçon sur ce qui a été essayé et abandonné.
