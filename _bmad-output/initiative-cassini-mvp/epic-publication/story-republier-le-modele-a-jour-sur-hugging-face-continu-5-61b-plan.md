---
title: 'Republier le modele a jour sur Hugging Face (continu-5-61b)'
type: 'chore'
ticket: '15'
created: '2026-11-17'
status: 'done'
baseline_revision: '8cf2d47afad1134363c64b7da877d8a12805413c'
route: 'oneshot'
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

**Problem:** `installer/modele.json` decrit encore cassini-composition-4b-0.1.0 (SHA-256 b687313f…), plus ancien que le modele installe sur le poste du proprietaire, `continu-5-61b` (SHA-256 1c430872…) : il compose `data.reproject` au lieu de `data.transform_point`, donc la demo 11.14 echoue chez qui installe par la porte 1.

**Approach:** Versionner le fichier du poste en 0.2.0 (meme quantification, meme taille) et mettre a jour, depuis cette seule empreinte, `modele.json`, le script `publier.ps1`, la fiche Hugging Face, l'empreinte de l'installeur complet et son test. L'envoi sur Hugging Face est fait par le proprietaire avec son jeton ; rien n'est envoye depuis la session.

</frozen-after-approval>

## Implementation Notes

Oneshot : cinq fichiers, des valeurs (nom versionne 0.2.0, SHA-256 1c430872…, taille inchangee 2 783 446 944) et une phrase de fiche. `publier.ps1` verifie deja que le modele installe porte l'empreinte attendue : il refusera tout autre fichier. A faire par le proprietaire : `powershell -File docs\modele-hugging-face\publier.ps1`, puis rejouer la sonde de 11.14 avec le modele telecharge depuis Hugging Face. Fait le 2026-11-17 : le proprietaire a lance `publier.ps1` ; le fichier 0.2.0 est en ligne (taille 2783446944, SHA-256 1c430872…, verifie par l'API du depot). Telecharge par `installer/telecharger_modele.py` (empreinte verifiee) dans un dossier temporaire avec sa declaration, la sonde de 11.14 rend `VERDICT : OK` (rejeu identique, 0 connexion). Tant que le fichier 0.2.0 n'etait pas en ligne, la recette de release (qui le telecharge) echouerait : ne pas etiqueter avant l'envoi.

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 3 low patches, 2 rejetes, 1 verifie faux.

| Verdict | Route | Evidence |
|---|---|---|
| low | patch (fait) | La fiche disait que la 0.2.0 « poursuit la 0.1.0 » : lien non etabli dans le depot ; phrase retiree. |
| low | patch (fait) | Le test de la declaration ne verifiait pas le nom versionne de l'url : assertion ajoutee. |
| low | patch (fait) | `publier.ps1` ne controlait le code de sortie qu'apres le dernier envoi : controle ajoute apres l'envoi du modele. |
| false | rejete | Taille supposee : `stat` du fichier installe donne 2783446944, egale a `modele.json`. |
| low | rejete | Rien n'impose d'envoyer avant d'etiqueter : dit dans les notes ; la recette echoue fort sans le fichier, aucun etat faux. |

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: tout vert
