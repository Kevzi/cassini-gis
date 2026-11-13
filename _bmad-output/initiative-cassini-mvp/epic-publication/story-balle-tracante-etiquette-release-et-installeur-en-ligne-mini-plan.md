---
title: 'Balle traçante : étiquette, release et installeur en ligne minimal sous Windows'
type: 'feature'
ticket: '1'
created: '2026-11-13'
status: 'built'
baseline_revision: '15ccd9d2184af9497d1e1c4ab577b4f52a8c8a63'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** rien ne permet aujourd'hui à un inconnu d'obtenir le noyau : pas de release, pas de manifeste, pas d'installeur, pas de page où aller.

**Approach:** une étiquette `vX.Y.Z` fait construire le noyau Windows processeur par la recette, vérifier la garde hors ligne sur le binaire livré, et publier une release avec `manifeste.json` ; un installeur Inno Setup lit ce manifeste, télécharge le noyau, vérifie son SHA-256, le pose et écrit la version ; une page minimale explique l'avertissement de Windows.

## Boundaries & Constraints

**Always:** adresses fixes d'AD-17 (release, `manifeste.json`, `https://kevzi.github.io/cassini-gis`) ; l'installeur est un programme séparé, seul à télécharger, une fois ; une empreinte qui ne correspond pas fait refuser l'installation, rien n'est posé ; noyau et extension inchangés (aucune dépendance Rust ajoutée) ; actions GitHub officielles seulement, le commentaire d'en-tête de `recette.yml` est remis d'accord avec `cargo-deny-action` ; la licence d'Inno Setup est approuvée par le propriétaire avant adoption.

**Never:** modèle, porte 2, mise à jour, Linux, GPU et signature de code (7.2 à 7.7) ; construire sur le poste du propriétaire ; publier une étiquette ou passer le dépôt en public à la place du propriétaire ; contact personnel (le contact est `cassini-sig@proton.me`).

## I/O & Edge-Case Matrix

| Scénario | État | Comportement | Erreur |
|----------|------|--------------|--------|
| Installation | manifeste et noyau valides | `cassini.exe` dans `%LOCALAPPDATA%\Cassini\bin`, `installation.json` à côté | — |
| Empreinte altérée | manifeste servi avec un SHA-256 faux | installation refusée, aucun fichier posé | message en français |
| Hors réseau | manifeste injoignable | installation refusée | message en français |
| Étiquette ≠ version | `v0.2.0` alors que Cargo dit 0.1.0 | la release n'est pas publiée | job en échec |

</frozen-after-approval>

## Code Map

- `.github/workflows/recette.yml` -- en-tête à corriger ; job `hors-ligne-windows` (CARGO_TARGET_DIR `C:\ct`, LIBCLANG_PATH) à reprendre pour la construction.
- `cli/tests/hors_ligne.rs` -- la garde, à lancer sur le binaire livré ; `docs/paquet-du-noyau.md` -- noyau processeur = `cassini.exe` seul.
- `qgis-plugin/cassini_qgis/locator.py` -- cherche déjà `%LOCALAPPDATA%\Cassini\bin\cassini.exe` ; à ne pas modifier.
- `Cargo.toml` -- version 0.1.0 du workspace, source de vérité de l'étiquette.
- Docs d'Inno Setup vérifiées : `DownloadTemporaryFile(Url, BaseName, RequiredSHA256OfFile, …)` compare le SHA-256 et lève une exception ; HTTPS et HTTP ; licence permissive propre (usage commercial permis, copie modifiée marquée comme telle).
- À créer : `.github/workflows/release.yml`, `.github/workflows/site.yml`, `installer/manifeste.py` (+ `installer/tests/`), `installer/cassini-en-ligne.iss`, `site/index.html`, `site/avertissement-windows.html`, `docs/publier-une-version.md`.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/manifeste.py`, `installer/tests/test_manifeste.py` -- écrit `manifeste.json` (version, et par fichier nom, url, SHA-256) et refuse une étiquette différente de la version Cargo -- tests unittest stdlib
- [ ] `.github/workflows/release.yml` -- sur `v*` : construit le noyau (Windows), lance la garde hors ligne sur ce binaire, compile l'installeur, publie la release avec `cassini.exe`, l'installeur et `manifeste.json` (`gh release`, permissions `contents: write` limitées à ce workflow)
- [ ] `installer/cassini-en-ligne.iss` -- lit le manifeste (paramètre `/MANIFESTE=` pour tester en HTTP local), `DownloadTemporaryFile` avec le SHA-256 du manifeste, pose le noyau, écrit `installation.json` (`{"version","installe_le"}`, date ISO UTC), désinstallation du dossier
- [ ] `.github/workflows/release.yml` -- étape de preuve : installation silencieuse contre un manifeste local valide (noyau posé, `cassini.exe run system.about` répond) puis altéré (installation refusée)
- [ ] `site/*`, `.github/workflows/site.yml` -- deux phrases, lien de téléchargement, explication de l'avertissement de Windows ; déploiement GitHub Pages depuis `site/`
- [ ] `.github/workflows/recette.yml` -- commentaire d'en-tête ; étape des tests de `installer/tests`
- [ ] `docs/publier-une-version.md` -- licence d'Inno Setup, étapes du propriétaire (audit, passage en public, étiquette)

**Acceptance Criteria:**
- Given un manifeste valide, when l'installeur s'exécute, then le noyau est posé, l'extension le trouve et `installation.json` porte la version et la date.
- Given un SHA-256 altéré, when l'installeur s'exécute, then il refuse et ne pose rien.
- Given une étiquette qui diffère de la version Cargo, when la release est lancée, then rien n'est publié.
- Given la page minimale, when on l'ouvre, then elle explique l'avertissement de Windows.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1, `thorough` (quatre lentilles). Aucun intent_gap ni bad_plan : pas de reprise du plan.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | Étiquette de préversion (`v0.1.0-rc1`) publiée comme « dernière » release, vers laquelle pointent les adresses fixes. Filtre d'étiquette resserré à `vX.Y.Z`. |
| medium | patch | Preuve : serveur local attendu 3 s à l'aveugle. Remplacé par une attente active, 30 s au plus. |
| medium | patch | La désinstallation supprimait tout `{app}`, donc le futur dossier du modèle (7.4). Réduit à `{app}\bin`. |
| medium | patch | `verifier_etiquette.main` sans test. Trois tests ajoutés (égale, différente, absente) ; 9 tests verts. |
| low | patch | En-tête de `release.yml` disait « actions officielles seulement » alors qu'Inno vient de Chocolatey. Corrigé. |
| low | patch | Limite de confiance du SHA-256 non écrite. Section ajoutée à `docs/publier-une-version.md`. |
| medium | defer | La release ne rejoue pas fmt, clippy et les tests complets : une étiquette peut tomber sur un commit jamais vert. Contrôle manuel documenté. |
| medium | defer | L'installeur et le manifeste ne sont exercés que sur une étiquette (jamais sur `main`). Piste : déclencheur `workflow_dispatch` sans publication. |
| medium | defer | Empreinte de l'installeur lui-même non publiée : Q6 la place sur la page, donc 7.12. |
| maybe-false | defer | Preuves de refus : code de sortie non nul seulement. Il faudrait lire le journal (`/LOG=`) pour confirmer la cause ; non vérifiable sans exécution Windows. |
| low | defer | Pas de champ de schéma dans le manifeste ; l'ouvrira 7.5 (mise à jour). Fichier du noyau en cours d'usage lors d'une mise à jour : 7.5. |
| low | defer | Licences livrées (R7) et page complète : 7.7 et 7.12. |
| false | rejeté | « Le binaire livré peut différer du binaire gardé » : la copie vers `dist\` vient après la garde, et `hors_ligne` lance `CARGO_BIN_EXE_cassini`, le même fichier. |
| false | rejeté | « En-tête de `recette.yml` désaccordé » : le job `noyau` utilise bien `cargo-deny-action`. |
| low | rejeté | Port 8001 occupé, release déjà existante au relancement, JSON de version non échappé (produit par notre générateur), `SaveStringToFile` non vérifié, `tomllib`, doublons de noms : improbables, correction plus lourde que le défaut. |

## Design Notes

Le manifeste lui-même n'a pas d'empreinte connue d'avance : sa confiance vient de HTTPS sur GitHub (Q6 : empreintes publiées sur la page). Inno Setup n'a pas d'analyseur JSON ; le script extrait nom, url et SHA-256 par recherche de texte, sur un manifeste à mise en forme fixée par `manifeste.py`.

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: OK
- `cargo test --release -p cassini --test hors_ligne --locked` -- expected: garde verte

**Manual checks (if no CLI):**
- Étape du propriétaire : audit de l'historique, dépôt public, étiquette `v0.1.0` ; puis installation sur un poste Windows et page à son adresse fixe.
