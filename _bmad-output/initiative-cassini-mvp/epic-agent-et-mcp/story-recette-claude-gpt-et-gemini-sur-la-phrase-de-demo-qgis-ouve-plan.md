---
title: 'Recette Claude, GPT et Gemini sur la phrase de démo, QGIS ouvert'
type: 'chore'
ticket: '27'
created: '2026-10-31'
status: 'done'
baseline_revision: 'bc171cd1569e0641d2c35d96a4eee2688ef771c6'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-ia-externe-pilote-la-session-qgis-ouverte/change-ia-externe-pilote-la-session-qgis-ouverte.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le relais (5.24-5.29) est construit et vérifié avec Claude Code seul ; rien ne montre que GPT et Gemini pilotent aussi la session QGIS, et le README ne dit pas comment brancher chaque client : le critère élargi de CAP-8 n'est pas livré.

**Approach:** Dans `README.md`, une fiche de branchement par client (Claude Desktop, ChatGPT pour ordinateur et Codex CLI, Gemini CLI). Une sonde `probe_external.py` sous le vrai QGIS ouvre une copie du projet Villeneuve-le-Roi, allume l'interrupteur, laisse un client externe jouer la phrase de démo, puis rejoue le journal dans un nouveau noyau. Chaque client est joué, les résultats consignés dans ce plan.

## Boundaries & Constraints

**Always:**
- Fiches tirées de la documentation officielle de chaque éditeur (sources citées) ; même binaire `cassini.exe mcp`, aucun argument propre à un client.
- Phrase exacte de `CLAUDE.md`, sur une copie `projet-sonde-5-27-<client>.qgz` posée à côté du projet de test (chemins relatifs conservés) ; le projet d'origine et ses données ne changent pas.
- Noyau `--features gpu` et extension installés à jour avant toute recette (mémoire « installer avant la recette »).
- Pour chaque client : appels reçus (commande, durée, issue), couche ajoutée au projet, PDF lisible, rejeu `identical: true` — ou l'échec tel qu'observé.
- Claude joue lui-même les clients en ligne de commande ; le propriétaire ne fait que les applications de bureau.

**Never:**
- Aucun code propre à un fournisseur d'IA (AD-29), aucune modification du noyau, du relais ou du panneau. Un client qui refuse un schéma ou bute sur le relais : on s'arrête, on consigne, on propose un ticket.
- Aucune dépendance de Cassini ajoutée ; les clients d'IA s'installent sur le poste seulement.
- Aucune donnée de test commitée.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Client joué | sonde lancée, interrupteur allumé, phrase envoyée au client | `external.call` reçus, couche(s) ajoutée(s), PDF `%PDF`, rejeu identique | — |
| Aucun appel | client sans outils Cassini (mauvaise config, mode autonome) | sonde : « aucun appel externe » au délai, code 1 | fiche corrigée, rejoué |
| Schéma refusé | client rejette un outil du catalogue | aucun appel ; message du client consigné | arrêt, ticket proposé |
| Appel en erreur | `isError` d'une commande | ligne d'erreur imprimée ; le client peut réessayer | consigné tel quel |

## Decisions (propriétaire, 2026-10-31)

- GPT : Claude installe Codex CLI sur le poste (npm, Apache-2.0, hors Cassini) ; le propriétaire fait une fois `codex login`, puis Claude joue la recette avec la sonde. ChatGPT pour ordinateur a sa fiche, non joué dans ce ticket.
- Claude : Claude écrit la configuration de Claude Desktop, le propriétaire y tape la phrase avec le vrai QGIS ouvert ; Claude joue en plus Claude Code en ligne de commande avec la sonde.

</frozen-after-approval>

## Code Map

- `README.md:369-421` -- section « Brancher un client MCP » : garder l'explication des deux modes ; remplacer le JSON générique par les fiches (Claude Desktop : `claude_desktop_config.json`, ouvert par Réglages › Développeur › Modifier la configuration — l'appli du Microsoft Store le range sous `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\` ; ChatGPT pour ordinateur : Réglages › MCP servers › Add server › STDIO, config partagée avec Codex dans `%USERPROFILE%\.codex\config.toml`, `[mcp_servers.cassini] command/args` ; Codex CLI : `codex mcp add cassini -- <exe> mcp` ; Gemini CLI : `gemini mcp add` ou `mcpServers` de `%USERPROFILE%\.gemini\settings.json`). Puis une recette en 4 gestes (ouvrir le projet, allumer l'interrupteur, taper la phrase, rejouer).
- `qgis-plugin/tests/probe_end_to_end.py:56-126,178-240` -- modèle : `find_core`, `GestureRegistry` complet (`layer.add`, `analysis.register_all`, `layout.export_pdf`), `guarded`, `read_journal`, phase de rejeu (`project.clear()`, `project.read`, `client.replay()`, `REPLAY_DONE.identical`).
- `qgis-plugin/tests/probe_demo.py:1-60` -- en-tête, `check_phrase()`, lancement sous `python-qgis-ltr.bat`, plateforme offscreen.
- `qgis-plugin/cassini_qgis/client.py:288-372` -- `set_external(enabled, ask)`, événements `EXTERNAL_STATE`, `EXTERNAL_FAILED`, `EXTERNAL_CALL_STARTED/FINISHED`.
- `../donnees-test-cassini/projet-test-villeneuve.qgz` -- couches `plu_zones` (U = constructible), `parcelles`, `gares` ; hors dépôt.
- Clients sur le poste (2026-10-31) : Gemini CLI 0.27.0 et Claude Code (Windows, `%APPDATA%\npm`), Claude Desktop (Store) ; ni ChatGPT pour ordinateur ni Codex CLI.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/tests/probe_external.py` -- sonde : copie le projet en `projet-sonde-5-27-<nom>.qgz`, lance le vrai noyau avec tous les gestes, `set_external(True, False)`, imprime chaque appel ; fin sur fichier d'arrêt (`<copie>.stop`) ou délai, puis contrôles (≥ 1 appel, couche ajoutée, PDF) et rejeu dans un nouveau noyau -- la preuve répétable.
- [x] `README.md` -- fiches des quatre clients et recette, sources en lien -- livrable du ticket.
- [x] Installation -- noyau `--features gpu` et extension à jour, `cassini.exe run model.list '{}'` -- prérequis.
- [x] Recette -- Gemini CLI (`gemini -p`), Codex CLI (`codex exec`, après `codex login` du propriétaire) et Claude Code (`claude -p`) joués par Claude avec la sonde ; Claude Desktop par le propriétaire, QGIS réel -- résultats dans Implementation Notes.

**Acceptance Criteria:**
- Given la sonde lancée et Gemini CLI branché par sa fiche, when la phrase de démo lui est donnée, then la sonde imprime les appels, une couche ajoutée, un PDF lisible et un rejeu identique.
- Given chacune des familles Claude, GPT et Gemini, when sa fiche est suivie mot à mot, then le client voit les outils de la session QGIS.
- Given la recette, when `python -I qgis-plugin/tests/check_extension.py` et les tests de `qgis-plugin/tests` tournent, then ils passent.

## Implementation Notes

- Noyau et extension installés déjà à jour (2026-10-31) : `cassini.exe` installé identique à `C:\ct\release` (md5 `04c7df32…`), construit après le dernier commit de `core`/`cli` ; extension identique au dépôt ; `model.list` répond. Rien réinstallé.
- Le panneau n'a pas de bouton de rejeu (seul `client.replay()` existe, servi aux sondes) : la recette du README s'arrête à **Couper** et renvoie le rejeu à la sonde. Pour Claude Desktop, joué dans le vrai QGIS, le rejeu identique est prouvé pour la famille Claude par Claude Code avec la sonde.
- Gemini CLI 0.27.0 : `gemini mcp list` lancé depuis WSL reste bloqué (processus tué) ; Cassini branché à la main dans `%USERPROFILE%\.gemini\settings.json` (ancienne version gardée en `settings.json.avant-5-27`).
- Le mode automatique de Claude Code a refusé que Claude lance lui-même `gemini -p … --approval-mode yolo` (« Create Unsafe Agents ») : recette en ligne de commande suspendue, en attente d'une autorisation du propriétaire.
- `check_extension.py` propre, pytest 323 verts, pyflakes propre sur la sonde.
- Le propriétaire a ensuite autorisé le lancement des clients ; il a coupé l'interrupteur de son QGIS ouvert, qui tenait le canal (`external.channel_taken` au premier essai de la sonde).
- **Gemini CLI 0.27.0 (2026-10-31, 12 h 57)** : branché et relais vu (le catalogue de la session arrive : seuls des avertissements « unknown format uint64/uint32 ignored », sans effet), puis **refus de Google à l'authentification** : « This client is no longer supported for Gemini Code Assist for individuals. […] migrate to the Antigravity suite » (compte personnel `oauth-personal`, formule gratuite). Aucun appel. Ce n'est pas Cassini : il faut une autre façon de s'identifier (voir la suite).
- Sonde, premier essai Claude Code : la sonde n'enregistrait pas `project.inventory` ni `layer.describe` (catalogue réduit, l'IA a rouvert les fichiers de données par `layer.add`) ; elle enregistre maintenant les mêmes gestes que `plugin.py`, sauf la commande de test. Ce premier essai a tout de même donné 7 appels, un PDF et un rejeu identique (11 étapes).
- **Claude Code (`claude -p`, dossier neutre, `--allowedTools mcp__cassini__*`, 13 h 03)** : **réussite**. 7 appels, tous réussis : `project.inventory`, `analysis.select_by_attribute` (typezone = U, 137 zones), `analysis.buffer` (500 m, fusionné), `analysis.intersect` ×2, `layer.describe`, `layout.export_pdf` (1 page). La couche est ajoutée au projet, le PDF est lisible, le journal porte `origin: external`, et le rejeu dans un nouveau noyau rend `identical: true` (9 étapes). L'IA signale elle-même deux limites : la carte montre toutes les couches visibles, et aucun outil ne permet d'en masquer une.
- Codex CLI 0.166.0 installé (npm, Apache-2.0) ; `codex mcp add cassini -- …/cassini.exe mcp` a écrit `%USERPROFILE%\.codex\config.toml` comme le dit la fiche. En attente de `codex login`, à faire par le propriétaire.
- **Gemini CLI, avec la clé d'API du propriétaire (13 h 15-13 h 31)** :
  - La 0.27 demande des modèles retirés (404 sur `gemini-2.5-pro` et `gemini-2.5-flash-lite`). Mise à jour vers la 0.66.0 (deuxième essai).
  - Ensuite, `--approval-mode yolo` ne passe plus hors d'un dossier de confiance. Il est remplacé par `"trust": true` sur le seul serveur `cassini`, et par `--skip-trust` dans un dossier temporaire neutre. La clé est passée par l'environnement et n'est écrite dans aucun fichier.
  - Résultat : **relais vérifié avec Gemini**. Le catalogue de la session arrive, et `project.inventory` réussit par le relais (55 ms, journalisé `origin: external`, rejeu identique).
  - Puis l'API de Google échoue 9 fois de suite (`503 UNAVAILABLE, high demand`, puis `fetch failed`) et Gemini abandonne. Il n'y a ni couche ni PDF : la phrase n'est pas finie. Cause hors Cassini ; règle des deux essais atteinte, arrêt.
  - La fiche du README est complétée : version récente, clé d'API, `trust`.
- **Gemini CLI, nouvel essai (13 h 51)** : encore des `503 high demand`, puis `429` : le quota gratuit du jour est épuisé (20 requêtes par jour sur `gemini-3.8-flash`, les tentatives répétées l'ont consommé ; nouvel essai possible dans 18 h environ). Aucun appel. Gemini reste à finir : relais prouvé, phrase non finie.
- **Claude Desktop (propriétaire, vrai QGIS, 2026-10-31)** : « c'est bon tout fonctionne ». Les appels défilent dans le panneau, la couche apparaît sur la carte, et le PDF sort.
- Décision du propriétaire (2026-10-31) : 5.27 se clôt avec Gemini « relais prouvé, phrase à finir ». La recette complète de Gemini et le champ `instructions` du serveur MCP sont au report (`deferred-work.md`).
- **Codex CLI 0.166.0 (`codex exec --sandbox read-only`, dossier neutre, 13 h 49-13 h 50)** : `default_tools_approval_mode = "approve"` posé sur le seul serveur `cassini`.
  - Premier essai, avec la phrase seule : Codex voit le serveur (`codex mcp list` : enabled), mais il **ne l'emploie pas**. Il tente le terminal (refusé par le bac à sable), puis cherche le PLU sur le web, et conclut qu'il ne peut rien faire. Aucun appel.
  - Second essai : la phrase est précédée de « Dans mon projet QGIS ouvert, avec les outils Cassini : ». **Réussite** : 8 appels, tous réussis (`project.inventory`, sélections, tampon de 500 m autour de la gare « Villeneuve-le-Roi », deux intersections, `layout.export_pdf`). La couche est ajoutée, le PDF est lisible, `origin: external`, et le rejeu rend `identical: true` (10 étapes).
  - Codex donne le chemin du PDF comme s'il était dans son propre dossier. Le noyau ne rend qu'un nom relatif (AD-9) ; Codex le rattache à tort à son dossier de travail.
  - Constat : Claude Code prend Cassini de lui-même ; Codex ne le fait que si on nomme Cassini ou QGIS dans la demande. Le serveur MCP n'envoie pas de champ `instructions` (Codex le lit, d'après sa doc) : piste de ticket, hors périmètre ici.
- Claude Desktop (Store) : `cassini` ajouté à `…\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\claude_desktop_config.json` (ancienne version gardée en `.avant-5-27`). Recette à faire par le propriétaire.

## Plan Change Log

## Review Triage Log

Relecture `quick`, 2026-10-31. Comptes : high 0, medium 1, low 4, false 0, rejetés 5 (dont 4 dont le correctif serait d'éditer le plan).

| # | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|
| 1 | Critère 1 (Gemini : couche, PDF, rejeu) non tenu ; « chaque client est joué » faux pour Gemini | low | réel, mais clôture décidée par le propriétaire (Implementation Notes) et recette Gemini au report ; correctif = éditer le plan figé | rejeté |
| 2 | Contrôle du journal faux dès qu'un appel échoue à l'exécution | medium | `Session::call_from` (`core/src/journal/mod.rs`) journalise aussi les étapes en `error` ou `cancelled` avec `origin` ; la sonde comparait à `succeeded` | patch (ne compter que les lignes sans `error` ni `cancelled`) |
| 3 | `EXTERNAL_STATE` avec `enabled: false` ignoré : attente de 30 min | low | aucune branche pour ce cas dans `on_first` | patch (échec aussitôt) |
| 4 | Fichier d'arrêt sans effet si un `finished` ne vient jamais | low | le noyau émet `finished` pour tout appel commencé, même relais parti (5.28) ; garde nouvelle | rejeté |
| 5 | Fiche Claude Code sans lien vers une source | low | les trois autres fiches en ont un | patch (lien vers code.claude.com/docs/en/mcp, vérifié) |
| 6 | Fiche Gemini : constats de la recette présentés comme faits généraux | low | refus du compte et modèles retirés observés ici, pas dans la doc de Google | patch (« constaté lors de la recette du 31 octobre 2026 ») |
| 7 | Étape « rejouer » de la recette remplacée par « Couper » sans entrée au Plan Change Log | low | écart justifié dans Implementation Notes ; correctif = éditer le plan | rejeté |
| 8 | Design Notes encore en `--approval-mode yolo` | low | correctif = éditer le plan ; la marche réelle est dans Implementation Notes et la fiche | rejeté |
| 9 | Tâches non cochées | low | tenue du plan, cochées à la clôture de l'implémentation | rejeté (fait hors relecture) |

Vérification après correctifs : pyflakes propre, `check_extension.py` propre, pytest 323 verts.

## Design Notes

La sonde ne pilote pas le client : elle tient la session comme le ferait QGIS ouvert. Claude lance la sonde en tâche de fond, puis le client (`gemini -p "<phrase>" --approval-mode yolo`), puis crée le fichier d'arrêt. Le propriétaire, lui, joue dans le vrai QGIS et lit le panneau ; la sonde n'est pas nécessaire pour lui.

## Verification

**Commands:**
- `python -I qgis-plugin/tests/check_extension.py` -- expected: ok (sonde hors paquet de l'extension)
- `uv run --with pytest python -m pytest qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_external.py <projet> gemini` -- expected: code 0, rejeu identique
