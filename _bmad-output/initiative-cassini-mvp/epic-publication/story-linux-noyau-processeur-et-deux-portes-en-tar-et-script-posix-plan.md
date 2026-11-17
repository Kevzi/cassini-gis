---
title: 'Linux : noyau processeur et deux portes en tar et script POSIX'
type: 'feature'
ticket: '7'
created: '2026-11-17'
status: 'built'
baseline_revision: 'aa3f9f0f92ecf2fcbfaafd6e66440965f7d9dbc9'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La recette ne construit que pour Windows : aucun noyau Linux, aucune porte d'installation, aucune mise a jour sous Ubuntu. L'extension sait deja chercher `~/.local/share/cassini/bin/cassini` (`locator.py`), mais rien ne l'y pose.

**Approach:** Un job Linux de la recette (ubuntu-22.04) construit le noyau processeur et deux archives ; un script POSIX unique `installer/cassini.sh` installe sans droits root (porte 1 reseau, porte 2 a partir de l'archive complete) et se relance avec `--mise-a-jour`. Le manifeste gagne une section `linux`, que l'installeur Windows ne lit pas.

## Boundaries & Constraints

**Always:** un seul script pour les deux portes et la mise a jour (porte 2 = le script trouve `noyau/` et `modele/` a cote de lui, aucun reseau) ; installation dans `~/.local/share/cassini/bin` (noyau, `installation.json`, `models/`), jamais root ni `sudo` ; chaque fichier verifie par `sha256sum` avant la pose, rien pose si l'un echoue ; noyau pose par `mv` d'un fichier temporaire du meme dossier (QGIS ouvert) ; `installation.json` au format de 7.1 (`version`, `installe_le` en `%Y-%m-%dT%H:%M:%SZ`) ; regles de 7.5 : comparaison a trois nombres, a jour = rien telecharge ni reecrit, aucun fichier de `models` supprime ni ecrase, modele sous son nom versionne, declaration `*.modele.json` avec `model_sha256` comme l'installeur Windows ; noyau construit sans `gpu`, sans `target-cpu=native`, glibc au plus 2.35 verifiee dans la recette ; la section `linux` du manifeste vient apres `porte2` : la lecture de l'installeur Windows reste intacte ; `core`, `ai`, l'extension ne changent pas.

**Never:** pas de Vulkan sous Linux ; pas de `jq` ni d'autre dependance au-dela de `sh`, `tar`, `sha256sum`, `curl` ou `wget` ; pas de mise a jour automatique ; pas de publication, d'etiquette, de jeton ni de push depuis la session.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Porte 1 | manifeste servi, HOME vide | noyau et modele poses, declaration, `installation.json` | — |
| Porte 2 | archive depliee, reseau coupe | meme resultat sans aucun appel reseau | — |
| Octet change | noyau ou modele altere | rien pose, code non nul | message qui nomme le fichier |
| Mise a jour | installe 0.1.0, manifeste 0.2.0 | noyau remplace, ancien modele garde, nouveau a cote | — |
| Deja a jour | installe = manifeste, modele present | rien telecharge ni reecrit, code 0 | — |
| Non installe | `--mise-a-jour` sans installation | message clair, rien pose | code non nul |

</frozen-after-approval>

## Code Map

- `.github/workflows/release.yml` -- un seul job `release` (windows-latest) ; ajouter un job `linux` (ubuntu-22.04) en amont, le job Windows en `needs: linux` et telecharge ses artefacts avant `manifeste.py` ; `gh release create` reste le dernier pas et joint aussi le tar du noyau ; depot Hugging Face de la porte 2 Linux dans le pas existant.
- `installer/manifeste.py` (`construire`, `main`) -- section `linux` (`noyau` : nom, url, sha256 ; `porte2` : nom, url, sha256, taille), une valeur par ligne, apres `porte2` ; drapeau `--linux`.
- `installer/cassini.sh` -- a creer ; lit le manifeste par `sed`/`grep` (pas de JSON), meme regle de version que `cassini-en-ligne.iss` ; recopie de lui-meme en `~/.local/share/cassini/mettre-a-jour-cassini`.
- `installer/assembler_linux.py` -- a creer ; liste fermee (binaire `cassini`), tar.gz du noyau, tar complet, plafond glibc via `objdump -T`.
- `installer/modele.json`, `installer/telecharger_modele.py` -- source du modele, a reutiliser.
- `qgis-plugin/cassini_qgis/locator.py`, `installation.py` -- chemin Linux et lecture de `installation.json` deja la ; a lire seulement.
- `cli/tests/hors_ligne.rs` -- garde hors ligne par table d'imports PE : a verifier pour ELF ; ajouter une branche ELF seulement si elle manque.
- `installer/tests/test_manifeste.py`, `test_en_ligne.py` -- modeles de gardes ; creer `test_linux.py` (gardes textuelles de `cassini.sh`, table de versions, essai reel avec `sh` sur un HOME temporaire, ignore sans `sh`).
- `docs/publier-une-version.md` -- section « Linux (7.7) » et essai Ubuntu 22.04.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/manifeste.py`, `test_manifeste.py` -- section `linux` et `--linux`, absente quand non fournie, garde : la boucle Windows ne l'avale pas
- [ ] `installer/cassini.sh` -- installation porte 1 et porte 2, `--mise-a-jour`, empreintes, `mv` atomique, declaration, `installation.json`, regles de 7.5
- [ ] `installer/assembler_linux.py` -- archives et garde glibc
- [ ] `installer/tests/test_linux.py` -- gardes textuelles, table de versions, essai reel de `cassini.sh` (installation, octet change, mise a jour, a jour)
- [ ] `.github/workflows/release.yml` -- job `linux` : construction sans `gpu`, `cargo test --test hors_ligne`, archives, preuve (installation valide, altere refuse, mise a jour, a jour) sur un serveur local ; artefacts ; job Windows en aval
- [ ] `cli/tests/hors_ligne.rs` -- branche ELF si la garde ne s'applique pas
- [ ] `docs/publier-une-version.md` -- section Linux

**Acceptance Criteria:**
- Given `python -I -m unittest discover -s installer/tests`, then tout passe, essai de `cassini.sh` compris quand `sh` est present.
- Given la recette sur etiquette, when le job Linux finit, then le tar du noyau, le tar complet et `cassini.sh` existent et le manifeste porte la section `linux`.
- Given un HOME vide, when `cassini.sh` tourne contre un manifeste local, then le noyau repond a `run system.about` et `model.list` ne signale pas `models.tampered`.

## Implementation Notes

Fait le 2026-11-17 : `manifeste.py` (section `linux`, `--linux`), `cassini.sh` (porte 1, porte 2, `--mise-a-jour`, empreintes, `mv`, declaration, `installation.json`), `assembler_linux.py` (archives, garde glibc 2.35 et liste fermee de bibliotheques), `preuve_linux.py` (preuve de la recette), `test_linux.py`, job `linux` de `release.yml` (job Windows en `needs: linux`), branche ELF de `cli/tests/hors_ligne.rs`, doc. 97 tests verts, 1 ignore ; `cassini.sh` joue pour de vrai sous `sh` et `dash`. Jamais execute : le job Linux, le binaire ELF reel, la garde glibc, Ubuntu 22.04. La garde ELF n'interdit pas `socket`/`connect` (plus faible que la garde Windows `ws2_32`). Le tar complet de 2,8 Go passe en artefact vers le job Windows : environ 2,8 Go de disque en plus.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 2 medium, 4 low.

| Verdict | Route | Evidence |
|---|---|---|
| medium | patch (fait) | Disque du runner (~14 Go) : target, modele, tar complet, copie depliee et copie installee : `rm -rf target modele` apres l'assemblage, copies de la preuve supprimees des qu'elles ne servent plus. |
| medium | patch (fait) | Seul GLIBC_x.y etait verifie : lecture des `NEEDED` (`objdump -p`) contre une liste fermee, libgomp refusee ; test sur texte synthetique. Si le noyau reel demande libgomp, la recette echoue fort (a regler par CMAKE/OpenMP). |
| low | rejete | Modele pose avant le noyau : un echec de pose (disque plein) laisse un modele sans noyau ; la relance est idempotente et nettoie, cas rare. |
| low | rejete | Copie du script en `mettre-a-jour-cassini` qui echoue : le message de succes cite un chemin possiblement absent ; sans gravite, l'installation elle-meme est complete. |
| low | rejete | `assembler` ecrit les deux archives avant `verifier_archives` : la docstring promet trop, mais la recette echoue de toute facon et `sortie` est jetable. |
| low | rejete | Pas de test d'echec de pose : couvert pour l'essentiel par le rejet ci-dessus. |

## Design Notes

Sans `dynamic-backends`, le noyau processeur Linux vise la base x86-64 generique (plus lent, sans AVX2) ; acceptable pour une premiere livraison, a mesurer sur la VM Ubuntu. La porte 2 Linux depasse 2 Gio : meme depot Hugging Face que la porte 2 Windows, pas la release GitHub.

Non verifiable ici : Ubuntu 22.04 propre, QGIS Linux, la recette complete ; la VM du proprietaire (hitl) reste le juge.

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: tout vert

**Manual checks (if no CLI):**
- Ubuntu 22.04 propre : porte 1, puis porte 2 reseau coupe, demo, mise a jour qui garde l'ancien modele.
