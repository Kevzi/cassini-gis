---
title: 'Le modèle publié sur Hugging Face, avec sa licence et sa fiche'
type: 'feature'
ticket: '3'
created: '2026-11-13'
status: 'in-progress'
baseline_revision: 'ede7d0a'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la recette de 7.2 et l'installeur en ligne de 7.4 téléchargent le modèle depuis Hugging Face, où il n'est pas encore.

**Approach:** le propriétaire crée le compte et le jeton (étape hitl) ; le dépôt public `KevziFar/cassini-composition-4b-GGUF` reçoit le modèle installé (`cassini-composition-4b-Q4_K_M.gguf`, SHA-256 `b687313f4a49c23e…`, session 5.54-3) sous le nom versionné `cassini-composition-4b-0.1.0-Q4_K_M.gguf`, avec la licence Apache-2.0, une NOTICE de Qwen, la mention des modifications et une fiche qui décrit un usage civil. La fiche ne dit rien d'un module défense (CLAUDE.md §2.9).

</frozen-after-approval>

## Implementation Notes

Oneshot : le travail est de la documentation et un script d'envoi (moins de 100 lignes de code). Sources de la fiche, de la NOTICE et du script d'envoi dans `docs/modele-hugging-face/`, versionnées ici ; le fichier de 2,65 Go n'entre jamais dans le dépôt. Base : `Qwen/Qwen3.5-4B`, Apache-2.0 sur Hugging Face (vérifié sur l'API et le fichier LICENSE du dépôt de Qwen).

## Verification

**Commands:**
- `curl -sIL https://huggingface.co/KevziFar/cassini-composition-4b-GGUF/resolve/main/cassini-composition-4b-0.1.0-Q4_K_M.gguf` -- expected: 200
- `sha256sum` du fichier téléchargé -- expected: égal à celui du modèle installé

**Manual checks (if no CLI):**
- La page du dépôt montre la licence Apache-2.0, la NOTICE de Qwen, la mention des modifications et la fiche.
