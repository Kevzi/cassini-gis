---
title: 'Demonstration de bout en bout dans QGIS'
type: 'feature'
ticket: '14'
created: '2026-11-16'
status: done
baseline_revision: '3b10bb0f991039ba734f86c789ff7c3d2e93ffda'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Aucune sonde ne montre, sous le Python de QGIS et avec le vrai modele, une seule demande en francais qui enchaine conversion MGRS, reprojection vers le systeme du MNT, champ de vision cumule et PDF A3 carroye, puis rejoue le journal.

**Approach:** Une sonde jetable derivee de `probe_demo.py` (jeu d'Annecy, vrai modele, une phrase), avec un verdict qui lit le PDF (A3, carroyage), un script PowerShell de lancement, puis la sonde est jouee sur le poste du proprietaire qui confirme la demonstration.

## Boundaries & Constraints

**Always:** une seule demande en francais ; vrai modele installe, sans reseau ; fichiers d'origine jamais modifies (copie du projet) ; rejeu apres fermeture et reouverture du projet, identique ; aucune logique metier dans l'extension.

**Decisions (2026-11-16, proprietaire) :** (1) la phrase proposee dans le plan est retenue (point reel choisi sur le MNT d'Annecy) ; (2) si le modele rate la chaine, le ticket finit en constatant l'echec avec le plan obtenu, et le proprietaire decide de la suite — pas de decoupage en deux phrases.

**Never:** pas de re-entrainement ni d'installation de modele dans ce ticket ; pas de nouvelle commande ; pas de donnees commitees (`donnees-test-cassini` reste hors depot).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Chaine reussie | une phrase, projet Annecy copie | plan valide, 4 familles de commandes executees, PDF A3 carroye, `VERDICT : OK` | — |
| Rejeu | projet ferme puis rouvert | journal rejoue identique, autant d'etapes que d'executees | `VERDICT : ECHEC — rejeu` |
| PDF sans carroyage ou hors A3 | PDF produit | verdict ECHEC nomme le defaut | code 1 |
| Modele rate la chaine | plan faux ou question | verdict ECHEC avec le plan obtenu ; rien n'est corrige en entrainant | code 1, plan consigne |

</frozen-after-approval>

## Phrase de la demonstration

« Convertis le point 32T LR 12345 67890 (MGRS) en WGS84, reprojette-le vers le systeme du MNT, calcule le champ de vision cumule depuis ce point et sors la carte en PDF A3 avec un carroyage UTM de 1 km. » Le point MGRS est remplace par un point reel du MNT d'Annecy.

## Code Map

- `qgis-plugin/tests/probe_demo.py`, `demo_verdict.py` (+ `test_demo_verdict.py`) -- modele : lancement, verdict, rejeu apres reouverture, hors ligne ; copier le patron.
- `qgis-plugin/tests/probe_recette.py`, `recette_verdict.py` -- phrases contre plan attendu, rejeu en session puis apres reouverture.
- `qgis-plugin/tests/probe_mgrs_layout.py`, `probe_layout_export.py` -- lecture de la mise en page : A3 (1191x842 pt) et grille ; a reprendre pour le verdict du PDF.
- `../outils-cassini/scripts/recette-5-54.ps1` -- lance `python-qgis-ltr.bat` avec `CASSINI_CORE` et `CASSINI_MODELS`.
- `../donnees-test-cassini/annecy/projet-test-annecy.qgz`, `annecy/mnt/` -- jeu de la demo (copie obligatoire).
- Commandes : `coord.convert` (noyau), `data.transform_point` / `data.reproject` (QGIS, le noyau complete la reprojection du point, 11.22), `terrain.viewshed_cumulative` (noyau, <= 32 observateurs), `layout.export_pdf` (`page_size` a3, `grid`).

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/demo_chaine_verdict.py` (+ test) -- verdict pur : plan contient les quatre familles, PDF A3, carroyage, rejeu identique -- testable hors QGIS
- [ ] `qgis-plugin/tests/probe_demo_chaine.py` -- sonde derivee de `probe_demo.py` : copie du projet Annecy, une phrase, vrai modele, rejeu apres reouverture
- [ ] `outils-cassini/scripts/demo-11-14.ps1` -- lancement sur le modele de `recette-5-54.ps1`
- [ ] jouer la sonde sur le poste, consigner plan, PDF et verdict ; le proprietaire confirme

**Acceptance Criteria:**
- Given le projet d'Annecy copie et le modele installe, when la sonde envoie la phrase, then elle rend `VERDICT : OK` et le plan contient conversion MGRS, reprojection, champ de vision cumule et export PDF.
- Given le PDF produit, when le verdict le lit, then il est en A3 et porte le carroyage demande.
- Given le projet ferme puis rouvert, when le journal est rejoue, then le resultat est identique.
- Given la sonde jouee, when le proprietaire a vu le resultat, then sa confirmation est consignee dans le plan.

## Implementation Notes

- Fait : `demo_chaine_verdict.py` + `test_demo_chaine_verdict.py` (24 tests verts), `probe_demo_chaine.py`, `outils-cassini/scripts/demo-11-14.ps1`. Point choisi sur le MNT 25 m d'Annecy : `32T KR 81280 88316` (Mont Veyrier, 797 m au MNT).
- Sonde jouee deux fois le 2026-11-16 (modele `ajuste-continu-5-58-1` installe, noyau installe, hors ligne) : `VERDICT : ECHEC — question`. Aucun plan : le garde-fou des coordonnees du noyau (`source: guard`) repond « Il manque a la demande un point en coordonnees : quels sont x, y et le systeme (code EPSG...) ? » (~38 s, identique aux deux essais). Le point MGRS n'est pas reconnu comme point en coordonnees. Releves : `%TEMP%\demo-11-14-20261116-141201.txt` et `-141250.txt`.
- Selon la decision (2), rien n'est corrige en entrainant ni en changeant la phrase : le proprietaire decide de la suite. Restent non verifies : plan valide, PDF A3 carroye, rejeu, reseau (jamais atteints). Confirmation du proprietaire : non consignee.
- **Rejouee le 2026-11-16 apres 5.61** (modele `continu-5-61b` installe) : plan juste en quatre etapes, PDF A3 carroye regarde, rejeu identique, `VERDICT : OK`. Le compteur de pages du verdict a ete corrige (Qt reecrit la page dans une mise a jour incrementale). **Confirmation du proprietaire (2026-11-16) : la demo est confirmee sur son poste.** Ticket clos.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 1 medium, 4 low, 2 maybe-false.

| Verdict | Route | Evidence |
|---|---|---|
| medium | patch (fait) | `judge_requested` refusait une grille de 1 km ecrite 1000 m, que `layout_export.py` accepte : faux ECHEC. Corrige, tests ajoutes (26 verts). |
| low | rejete | Grille : `kind` et `epsg` non verifies au niveau du plan ; `judge_pdf` les attrape apres execution. |
| low | rejete | Zone UTM quelconque acceptee dans `judge_pdf` ; le test `test_utm_31_is_accepted` le fige. Non corrige : n'empeche pas la demonstration. |
| low | rejete | `-Models` du script ps1 pointe tout le dossier des modeles ; un seul modele y est installe. |
| low | rejete | Chemins en dur du script ps1, hors depot. |
| maybe-false | defere | `pdf_media_box` / `pdf_page_count` lisent le PDF en octets : jamais essayes sur un PDF produit par Qt (aucun produit). A regler au premier PDF reel. |
| maybe-false | defere | Le plan n'est pas relie (observateur issu de la conversion, reprojection vers le CRS du MNT) : seuls les ids sont controles. |
| — | non-defaut | Criteres 1 a 4 non remplis : echec accepte par la decision 2 ; confirmation du proprietaire non consignee. |

## Verification

**Commands:**
- `python -m pytest qgis-plugin/tests/test_demo_chaine_verdict.py` -- expected: tests du verdict verts

**Manual checks (if no CLI):**
- `outils-cassini/scripts/demo-11-14.ps1` sur le poste : `VERDICT : OK`, PDF A3 carroye ouvert et regarde (pas seulement le verdict).
