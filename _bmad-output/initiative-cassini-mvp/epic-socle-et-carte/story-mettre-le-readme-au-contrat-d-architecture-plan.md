---
title: "Mettre le README au contrat d'architecture"
type: 'chore'
ticket: '10'
created: '2026-10-14'
status: 'built'
baseline_revision: 'eec793d548c400cb5a55abf7c193b6f9fa3f3d6c'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md', '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le `README.md` racine, remis au plan à l'entrée 1.9, précède le contrat d'architecture du 2026-10-14 : il dit encore que la forme du canal entre l'extension et le noyau n'est pas tranchée, ignore le dossier `cli/` et le binaire `cassini`, et son schéma de séquence fait enchaîner les étapes par le panneau alors que AD-5 donne la boucle au noyau. Le dépôt est public : la page d'accueil contredit le contrat.

**Approach:** corriger seulement les passages que le contrat rend faux ou incomplets — schéma d'architecture (canal stdin/stdout en JSON-RPC 2.0, binaire `cassini` et ses sous-commandes), paragraphe sur le canal, schéma de séquence (le noyau dicte chaque étape à l'extension), tableau et paragraphe des dossiers (`cli/`), licence (`cli/` en Apache-2.0), plus deux phrases sur le plancher QGIS 3.44 à 4.99, la distribution en deux artefacts et la lecture du MNT par le noyau, et un lien vers le contrat. Tout fait ajouté renvoie à un AD-n du contrat ; le reste du README, recette et démarrage rapide compris, n'est pas touché.

</frozen-after-approval>

## Implementation Notes

Route oneshot : une soixantaine de lignes de Markdown dans un seul fichier, sans code. Continuité de l'entrée 1.9 : trois schémas Mermaid courants, libellés entre guillemets droits, et ne pas surcorriger les deux tiers du README qui restent vrais.

## Verification

**Commands:**
- `grep -n "pas tranchée\|pas encore tranchée" README.md` -- expected: aucune ligne
- `grep -c '^```mermaid' README.md` -- expected: `3`
- `cargo test -p cassini-core` -- expected: 17 tests passent (rien du noyau ne change)

**Manual checks (if no CLI):**
- Chaque fait ajouté (canal, boucle, `cli/`, plancher QGIS, deux artefacts, MNT) se retrouve dans le contrat sous AD-2, AD-3, AD-5, AD-13, AD-17, AD-18 ou AD-20.

## Review Triage Log

Passe rapide (`quick`), 8 constats, tous réels : 0 high, 2 medium, 6 low ; tous corrigés (patch), aucun reporté.

- medium, patch — le schéma d'architecture gardait `registre --> ia`, flèche en sens inverse de la direction des dépendances du contrat (`cassini-core` ne dépend pas de `cassini-ai`) et contraire au texte du README. Remplacé par `binaire --> ia` et `ia --> registre`.
- medium, patch — la case IA portait encore « serveur MCP », alors que `mcp` est une sous-commande du binaire (AD-13), déjà dans la nouvelle case. Renommée « modèle local llama.cpp, lié au processus » (AD-15).
- low, patch — la règle « cassini-core ne dépend jamais de cassini-ai » citait AD-13 seul ; la citation nomme aussi la direction des dépendances du contrat.
- low, patch — plancher QGIS et distribution sans renvoi ; ajout de (AD-18) et (AD-17).
- low, patch — reprojection par QGIS sans renvoi ; ajout de (AD-19). Gardée : elle complète une phrase existante que le contrat rend incomplète.
- low, patch — ajouts sur la nature de ce qui circule (AD-8, AD-16), la validation du plan (AD-23), le journal au noyau seul et le modèle jamais servi par HTTP (AD-15) jugés hors de la liste de l'Approach. Gardés : ils corrigent des passages que le contrat rend incomplets, ce que l'Approach autorise ; le renvoi manquant (AD-11) est ajouté.
- low, patch — puce de licence en « et … et » et deux lignes au-delà de 80 colonnes ; reformulée et recoupée.
- low, patch — `Registre-->>Panneau: avancement affiché` employait la flèche de réponse pour un message à l'initiative du noyau ; passé en `->>`.
