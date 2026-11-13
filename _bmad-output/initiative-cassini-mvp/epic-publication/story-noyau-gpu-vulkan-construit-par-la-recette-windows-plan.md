---
title: 'Noyau gpu Vulkan construit par la recette Windows'
type: 'feature'
ticket: '6'
created: '2026-11-13'
status: 'built'
baseline_revision: '543721eae603ec3abf66c1be7c737ed677bab659'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
  - '{project-root}/docs/paquet-du-noyau.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau livré est construit pour le processeur seul : un poste avec une carte graphique n'en profite pas, alors que le noyau sait déjà faire du Vulkan (`--features gpu`) sur le poste du propriétaire.

**Approach:** le job Windows de la release construit le noyau gpu sur GitHub, avec un SDK Vulkan à version et empreinte fixées, assemble le paquet (`cassini.exe` et les DLL de llama.cpp) et le livre, sous le manifeste, comme noyau Windows des deux portes ; les deux installeurs posent tout le paquet.

## Boundaries & Constraints

**Always:** SDK Vulkan 1.4.363.0 (celui du poste du propriétaire), téléchargé à l'adresse officielle de LunarG `https://sdk.lunarg.com/sdk/download/1.4.363.0/windows/vulkansdk-windows-X64-1.4.363.0.exe`, empreinte SHA-256 `94a82d378f7a5e3e54c9db7d2fb7016af136e14ac0a18dbf0f2f67a36352d141` (publiée par LunarG, vérifiée), installé en silencieux (`install --accept-licenses --default-answer --confirm-command`) ; la construction échoue si l'empreinte diffère ; le SDK n'est jamais livré. Paquet par liste fermée : `cassini.exe`, `llama.dll`, `ggml.dll`, `ggml-base.dll`, tous les `ggml-cpu-*.dll`, `ggml-vulkan.dll` ; la construction échoue si l'un manque ou si un autre fichier est présent (jamais `llama-common.dll`, qui embarque un client HTTP). La garde hors ligne passe sur ce paquet (`cargo test --release -p cassini --features gpu --test hors_ligne`). Le noyau livré est unique, gpu : sans carte ni pilote Vulkan, il retombe sur le processeur à l'exécution (docs/paquet-du-noyau.md). Le manifeste liste chaque fichier du paquet avec son SHA-256 ; l'installeur en ligne les télécharge tous, vérifie chaque empreinte et ne pose rien si l'une diffère ; l'installeur complet les embarque. Rien n'est construit sur le poste du propriétaire. Les noyaux et DLL ne changent pas (aucun code Rust, aucune dépendance).

**Never:** accélération Vulkan sous Linux (hors de l'epic) ; CUDA ; modèle, déclaration à empreintes, « Mettre à jour » (7.4, 7.5) ; signature.

## I/O & Edge-Case Matrix

| Scénario | État | Comportement | Erreur |
|----------|------|--------------|--------|
| Installation | manifeste et paquet valides | tous les fichiers posés dans `bin`, `cassini.exe` répond | — |
| Un fichier altéré | SHA-256 faux pour une DLL du manifeste | installation refusée, rien posé | message en français |
| Paquet incomplet ou en trop | `ggml-vulkan.dll` absente, ou `llama-common.dll` présente | la construction échoue | message en français |
| SDK altéré | SHA-256 du SDK téléchargé différent | la construction échoue avant toute compilation | message en français |

</frozen-after-approval>

## Code Map

- `.github/workflows/release.yml` -- job Windows : construction processeur actuelle à remplacer (installation du SDK, `--features gpu`, garde, assemblage), `manifeste.py` déjà multi-fichiers ; preuves d'installation de 7.1 et de 7.2 à adapter au paquet.
- `installer/manifeste.py` -- accepte plusieurs fichiers ; `installer/cassini-en-ligne.iss` -- ne lit que l'entrée `cassini.exe` (`PositionFichier`, `ValeurApres`) : à boucler sur toutes les entrées, téléchargement dans `{tmp}\noyau`, pose par `Source: "{tmp}\noyau\*"` externe.
- `installer/cassini-hors-ligne.iss` -- `Source: {#Noyau}` : à remplacer par le dossier du paquet ; `installer/modele.json`, `telecharger_modele.py` -- ne changent pas.
- `docs/paquet-du-noyau.md` -- liste des fichiers à copier et des DLL interdite ; `cli/tests/hors_ligne.rs` -- vérifie le dossier avec `--features gpu`.
- Licence du SDK : fichier `Licenses\LICENSE.txt` du SDK installé (« 100 % open-source, majoritairement MIT ou Apache 2.0 ; le SDK lui-même n'est pas sous licence »), à reprendre dans `docs/publier-une-version.md`.
- À créer : `installer/assembler_noyau.py` (+ tests) qui copie la liste fermée de `C:\ct\release` vers `dist\noyau` et refuse tout écart.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/assembler_noyau.py`, `installer/tests/` -- assemble le paquet par liste fermée, refuse un fichier manquant ou en trop (dont `llama-common.dll`)
- [ ] `.github/workflows/release.yml` -- installe le SDK (empreinte vérifiée), construit `--features gpu`, garde hors ligne, assemble le paquet, manifeste multi-fichiers, preuves adaptées
- [ ] `installer/cassini-en-ligne.iss` -- télécharge et vérifie chaque fichier du manifeste, pose le paquet
- [ ] `installer/cassini-hors-ligne.iss` -- embarque le paquet
- [ ] `docs/paquet-du-noyau.md`, `docs/publier-une-version.md` -- licence du SDK, version et empreinte fixées, essai sur GPU et sur processeur

**Acceptance Criteria:**
- Given un poste avec carte graphique, when on lance le noyau installé, then il utilise Vulkan ; sans carte ni pilote, il retombe sur le processeur.
- Given une DLL altérée dans le manifeste, when l'installeur en ligne s'exécute, then il refuse et ne pose rien.
- Given un paquet incomplet ou avec `llama-common.dll`, when la recette l'assemble, then elle échoue avant toute publication.

## Implementation Notes

Essai à blanc `release.yml` sur la branche (run 43427703726) : vert, paquet assemblé, installation prouvée (valide, altérée, sans manifeste, installeur complet). Trois défauts trouvés par l'essai : compilation Vulkan en échec avec le générateur Visual Studio (corrigé : `CMAKE_GENERATOR=Ninja`) ; boucle de l'installeur en ligne qui avançait après le `nom` au lieu du `sha256` (les DLL recevaient le contenu de `cassini.exe`, code 0xC000007B ; corrigé) ; sorties de la preuve rendues bavardes. Reste à faire par le propriétaire : l'essai sur sa carte graphique et sans carte.

## Plan Change Log

## Review Triage Log

- `IGNORES` morte et docstring trop large (assembler_noyau.py) -- low, patch : constante supprimée, docstring réécrite.
- Garde hors ligne avant l'assemblage -- low, rejeté : la liste fermée rend le paquet égal au dossier de construction pour les .dll/.exe ; corriger ajouterait une étape.
- Un seul SHA altéré dans la preuve, boucle Pascal sans test unitaire -- maybe-false/medium, defer : seul l'essai à blanc de la release peut la valider.
- Test-Path "bin\*" ne prouve que l'état final -- false : c'est le comportement voulu (la pose ne vient qu'après PrepareToInstall).
- Intro de publier-une-version.md « ce binaire » -- low, patch : corrigée.
- Tableau « facultatif » de ggml-vulkan.dll -- low, patch : précisé (facultatif à l'exécution, obligatoire dans le paquet).
- Critère 1 sans test automatisé -- false : l'étape manuelle du propriétaire est prévue par la section Verification.

## Design Notes

Le SDK n'est qu'un outil de construction : `glslc.exe` compile les shaders et les en-têtes décrivent l'API ; `vulkan-1.dll`, le chargeur, vient du pilote de l'utilisateur et n'est jamais livré. Le noyau gpu unique évite deux paquets : sans GPU, `ggml-vulkan.dll` est ignorée et le moteur processeur le plus adapté est pris seul.

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: OK
- essai à blanc de la release (`gh workflow run release.yml`) -- expected: vert, paquet assemblé, installation prouvée

**Manual checks (if no CLI):**
- Sur le poste du propriétaire : installer le paquet de l'artefact, `CASSINI_DEBUG_LLAMA=1`, une phrase : le périphérique Vulkan est choisi et les couches posées sur la carte ; puis sans la carte (`GGML_VK_VISIBLE_DEVICES=aucune`) : le processeur.
