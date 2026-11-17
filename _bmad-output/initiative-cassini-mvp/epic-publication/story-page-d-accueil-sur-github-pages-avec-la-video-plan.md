---
title: "Page d'accueil sur GitHub Pages, avec la vidéo"
type: 'feature'
ticket: '12'
created: '2026-11-17'
status: 'built'
baseline_revision: '753d2f5cfb0f372b69af390a35bf5a3fdcaf120e'
route: 'full'
route_source: 'auto'
risk: 'low'
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

**Problem:** `site/index.html` n'a que deux phrases et un lien : ni vidéo, ni installation en trois étapes, ni empreintes SHA-256 des installeurs, ni FAQ ; le README ne renvoie pas à la page.

**Approach:** un générateur `installer/page_accueil.py` écrit `site/index.html` (et la FAQ) à partir du `manifeste.json` d'une release et de l'empreinte de l'installeur en ligne ; la vidéo est servie par la page elle-même, sans service tiers ; le README renvoie à la page pour l'installation.

## Boundaries & Constraints

**Always:** HTML et CSS en ligne, aucun script ni police ni ressource externe ; thème clair et sombre comme la page actuelle ; empreintes lues dans le manifeste (porte 2 et Linux) et calculées sur le fichier (installeur en ligne), jamais écrites à la main ; liens vers les adresses fixes des releases et de Hugging Face ; vidéo `site/cassini-presentation.mp4` sous 25 Mo, en balise `<video controls>` ; rien du module défense ; contact `cassini-sig@proton.me`.

**Never:** réencoder la vidéo dans le dépôt (outil hors poste du dépôt) ; publier la page sans la relecture du propriétaire ; changer le noyau, l'extension ou les installeurs ; ajouter un traqueur ou une dépendance.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Page d'une release | manifeste avec `porte2` et `linux`, installeur en ligne | page avec les trois étapes, un lien et une empreinte par installeur | — |
| Empreinte | fichier téléchargé | son SHA-256 égale celui affiché | — |
| Manifeste sans `linux` | section absente | la ligne Linux est omise | — |
| Vidéo absente | pas de `site/cassini-presentation.mp4` | page générée, sans balise vidéo | message qui le dit |
| Vidéo trop lourde | fichier de 25 Mo ou plus | échec | message nommant la taille |
| Texte | page générée | deux phrases, FAQ, lien vers la page SmartScreen, contact | — |

</frozen-after-approval>

## Code Map

- `site/index.html`, `site/avertissement-windows.html` -- page actuelle et style (variables `--fond`, `--texte`, `--lien`) à reprendre.
- `.github/workflows/site.yml` -- publie `site/` à chaque poussée de `site/**` ; ne change pas.
- `installer/manifeste.py` (`section_porte2`, `section_linux`) -- forme du manifeste : `porte2` et `linux.{noyau,porte2}` portent `nom`, `url`, `sha256`, `taille`.
- `docs/publier-une-version.md` (l. 69-75) -- annonce que les empreintes des installeurs paraîtront sur la page (7.12) ; y ajouter la commande de génération.
- `README.md` -- section « Installer » : renvoyer à la page.
- Vidéo source : `C:\Users\kevin\Videos\cassini-presentation.mp4` (30,1 Mo, hors dépôt) ; la recette du montage est dans `cassini-presentation.filter.txt` à côté. Pas de ffmpeg sur le poste Windows ni dans WSL : réencodage par le propriétaire.
- Release `v0.1.1` : `manifeste.json` et `cassini-installeur-en-ligne.exe` pour l'essai du générateur.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/page_accueil.py` + `installer/tests/test_page_accueil.py` -- générateur et un test par ligne de la matrice.
- [ ] `site/index.html`, `site/faq.html` -- pages générées depuis le manifeste de `v0.1.1`.
- [ ] `README.md`, `docs/publier-une-version.md` -- renvoi vers la page ; commande de génération et réencodage de la vidéo sous 25 Mo.
- [ ] Étape hitl (propriétaire) -- réencoder la vidéo, la poser dans `site/`, relire la page, pousser ; vérifier l'adresse fixe, la lecture et une empreinte.

**Acceptance Criteria:**
- Given un manifeste de release, when on génère la page, then chaque lien d'installeur est accompagné de son empreinte, identique à celle du manifeste.
- Given la page générée, when on la lit, then elle ne charge rien hors de son dossier.

## Implementation Notes

## Plan Change Log

## Review Triage Log

- medium, patch : lien `latest` avec l'empreinte d'un fichier précis ; lien désormais vers `releases/download/v<version>/`, page régénérée depuis `v0.1.1`, doc : régénérer à chaque release.
- low, rejetés : erreur de manifeste mal formé (le propriétaire le produit lui-même), mention Linux dans les étapes (aucune release Linux publiée), `STYLE` partagé, test des ressources externes partiel (suffisant pour ces gabarits), vidéo (étape hitl du propriétaire).

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: vert
- `python -I installer/page_accueil.py --manifeste <manifeste.json> --installeur-en-ligne <exe> --sortie site` -- expected: pages écrites

**Manual checks (hitl):**
- Adresse fixe `https://kevzi.github.io/cassini-gis` : la vidéo joue, un téléchargement a l'empreinte affichée.
