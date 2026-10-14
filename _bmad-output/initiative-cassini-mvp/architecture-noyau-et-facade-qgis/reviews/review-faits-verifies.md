# Revue — tout ce qui est affirmé a-t-il été vérifié ?

Lentille configurée dans `finalize_reviewers` : traquer ce qui vient de la mémoire
du modèle plutôt que d'une source. Conduite le 2026-10-14.

## Verdict

Vérifié pour l'essentiel. Quatre affirmations porteuses n'ont PAS été confirmées
et doivent l'être ou être marquées ; une entrée de pile est superflue.

## Confirmé contre une source, ce jour

| Affirmation | Source |
| --- | --- |
| 3.44 dernière LTR 3.x, 4.2.3 courante, 4.4.0 première LTR 4.x le 2026-10-30 | qgis.org/resources/roadmap |
| `qgisMinimumVersion`/`qgisMaximumVersion`, `supportsQt6` abandonné, shim `qgis.PyQt` | plugins.qgis.org/docs/migrate-qgis4 |
| « Don't include binaries », 25 Mo, GPLv2+, dépendance externe en *About*, `QgsNetworkAccessManager` recommandé | plugins.qgis.org/docs/publish |
| Tuyaux et sockets = programmes séparés, avec la réserve de l'échange intime | gnu.org/licenses/gpl-faq |
| rmcp 3.5.1 Apache-2.0 ; llama-cpp-2 0.1.159 ; serde_json 1.0.154 ; schemars 1.2.3 ; wbgeotiff 0.1.2 ; wbraster 0.2.1 — toutes permissives | API crates.io |
| `whitebox_next_gen` absent de crates.io ; seules six crates de fondation publiées | crates.io + dépôt GitHub |

## Constats

**C1 — critique.** `requestAboutToBeCreated` de `QgsNetworkAccessManager` est la
pièce sur laquelle repose toute la preuve « zéro réseau » côté Python (AD-14). Le
dépôt recommande bien ce gestionnaire, mais l'existence et la signature du signal
n'ont pas été confirmées dans l'API PyQGIS. À vérifier avant de s'engager.

**C2 — critique.** AD-9 repose sur la persistance et la stabilité de
`QgsMapLayer.id()` à travers fermeture et réouverture du projet. Non confirmé
dans la documentation. Si l'identifiant n'était pas persisté, AD-9 et le rejeu
changeraient de forme.

**C3 — haute.** AD-3 affirme que le transport stdio de MCP emploie du JSON-RPC
2.0 délimité par ligne, et en tire que « le serveur MCP réemploie le même
cadrage ». Non confirmé contre la spécification MCP courante.

**C4 — moyenne.** « JSON Schema draft 2020-12, engendré par `schemars` » : le
dialecte émis par schemars 1.x n'a pas été vérifié.

**C5 — moyenne.** Python 3.9 minimum au tableau de pile : affirmé, non vérifié
sur QGIS 3.44 sous Windows. Déjà porté en *Deferred*, mais le tableau le présente
comme épinglé.

**C6 — moyenne, à corriger directement.** `jsonschema` 0.58.6 figure à la pile
sans emploi : la désérialisation `serde` valide déjà les paramètres entrants, et
les schémas engendrés servent à décrire les commandes à une IA, pas à valider.
Une dépendance inutile au noyau contredit la règle de licence de CLAUDE.md §8,
qui veut qu'on la demande avant de l'ajouter.

**C7 — moyenne.** `llama-cpp-2` compile llama.cpp : la recette a donc besoin de
cmake et d'un compilateur C++ sur les trois systèmes. Conséquence d'exploitation
réelle, absente de l'enveloppe.
