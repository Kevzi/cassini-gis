---
title: 'Licences de tout ce qui est livré et frontière GPL'
type: 'feature'
ticket: '9'
created: '2026-11-17'
status: 'built'
baseline_revision: 'a60ea4b6fdc785b181426aa194ba0f2afc184925'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
  - '{project-root}/docs/publier-une-version.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la release et les installeurs ne disent pas sous quelles licences est livré chaque composant (crates du noyau, llama.cpp, modèle Qwen, outil d'installeur), ni pourquoi l'extension GPL et le noyau Apache-2.0 restent deux programmes séparés.

**Approach:** un script Python sans dépendance, `installer/licences.py`, lit `cargo metadata` et une liste fermée des composants hors Cargo, et écrit `LICENCES-TIERCES.txt` ; un texte `docs/frontiere-gpl.md` (copié en `FRONTIERE-GPL.txt`) explique la frontière. Les deux fichiers entrent dans les paquets du noyau, Windows et Linux, et sont joints à la release.

## Boundaries & Constraints

**Always:** aucun outil ni crate ajouté (stdlib Python et `cargo metadata` seulement) : sinon, licence soumise au propriétaire avant ajout ; liste du noyau seulement (`core`, `ai`, `cli`, sans `cassini-app`, comme `deny.toml`) ; sortie triée et déterministe ; échec si une crate livrée n'a pas de licence déclarée ; texte de frontière conforme à `CLAUDE.md` §2.3 et §2.4, sans rien du module défense (§2.9) ; fichiers en UTF-8, fins de ligne `\n`.

**Never:** réseau ; copier un texte de licence d'Esri ; changer le noyau, l'extension ou le catalogue ; signer ou renommer les installeurs ; casser les listes fermées des assembleurs.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Liste du noyau | `Cargo.lock` courant | une ligne par crate livrée : nom, version, SPDX ; plus llama.cpp (MIT), Qwen (Apache-2.0), Inno Setup | — |
| Crate ajoutée ou retirée | dépendance modifiée | la sortie suivante la montre ou ne la montre plus | — |
| Licence absente | crate sans champ `license` | échec nommant la crate | code de sortie non nul |
| Même entrée deux fois | deux exécutions | octets identiques | — |
| Paquets | assemblage Windows et Linux | `LICENCES-TIERCES.txt` et `FRONTIERE-GPL.txt` dans chaque paquet et joints à la release | assembleur en échec si absents |

</frozen-after-approval>

## Code Map

- `deny.toml` -- périmètre du noyau (exclut `cassini-app`), licences admises ; garde existante, à ne pas changer.
- `NOTICE`, `LICENSE` -- avis existants (ColorBrewer, WMM2025) ; la liste les complète, sans les remplacer.
- `installer/assembler_noyau.py` -- liste fermée du paquet Windows ; y ajouter les deux fichiers texte (installés sous `bin` par les deux portes).
- `installer/assembler_linux.py` (`attendus`, l. 217 ; ajouts de tar l. 146) -- liste fermée des tars Linux ; y ajouter les deux fichiers.
- `installer/preuve_linux.py`, `installer/manifeste.py` -- preuve et manifeste : lisent les paquets ; vérifier qu'ils acceptent les fichiers de plus.
- `.github/workflows/release.yml` (étapes aux l. 77, 173, 535-539) -- générer les fichiers avant l'assemblage et les joindre à `gh release create`.
- `docs/publier-une-version.md` (l. 9, licence d'Inno Setup) -- source de la ligne Inno Setup ; y ajouter une section sur ces fichiers.
- `docs/modele-hugging-face/` -- licence et base Qwen du modèle, pour la ligne Qwen.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/licences.py` + `installer/tests/test_licences.py` -- script et un test par ligne de la matrice -- génération déterministe.
- [ ] `docs/frontiere-gpl.md` -- texte de la frontière (processus séparés, canal JSON-RPC public, QGIS lie GDAL et GEOS de son côté, le noyau jamais).
- [ ] `installer/assembler_noyau.py`, `installer/assembler_linux.py` -- ajouter les deux fichiers aux listes fermées et aux contrôles.
- [ ] `.github/workflows/release.yml` -- générer, joindre à la release, exécuter le test.
- [ ] `docs/publier-une-version.md` -- section « Licences livrées ».

**Acceptance Criteria:**
- Given une release construite, when on ouvre chaque paquet, Windows et Linux, then `LICENCES-TIERCES.txt` et `FRONTIERE-GPL.txt` y figurent.
- Given une crate retirée ou ajoutée, when on relance le script, then la liste change en conséquence.
- Given une crate GPL ou LGPL parmi les dépendances du noyau, when on lance le script, then il échoue en la nommant.

## Implementation Notes

## Plan Change Log

## Review Triage Log

- medium, patch : `cargo metadata --offline` sur le job Linux sans les crates des autres plateformes ; `cargo fetch --locked` ajouté avant l'étape dans les deux jobs de `release.yml`.
- low, patch : docstring de `assembler_linux.py` (empreintes) corrigé ; les textes sont posés sans empreinte.
- low, rejetés : tolérance d'anciennes archives de `cassini.sh` (voulue), ordre strict du tar (produit par l'assembleur, testé), `test_depot_reel` ignoré en CI (l'étape de licences exécute le vrai script), `verifier_dossier` (false : `fichiers_attendus` mis à jour, tests verts), versions de `HORS_CARGO`, accord avec `deny.toml`, détection des racines, phrase de la doc.

## Verification

**Commands:**
- `python -I installer/licences.py --sortie <dossier temporaire>` -- expected: deux lectures donnent des octets identiques
- `python -I -m unittest discover installer/tests` -- expected: vert
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert
