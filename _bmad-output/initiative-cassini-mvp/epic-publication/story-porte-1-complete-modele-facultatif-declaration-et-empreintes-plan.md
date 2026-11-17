---
title: 'Porte 1 complete : modele facultatif, declaration et empreintes'
type: 'feature'
ticket: '4'
created: '2026-11-17'
status: 'built'
baseline_revision: 'b6a2da0cc213d5ad15356064738ba68ce3a181e9'
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

**Problem:** L'installeur en ligne (`installer/cassini-en-ligne.iss`) ne pose que le noyau. Le modele n'est pas propose, la declaration `*.modele.json` avec empreinte n'est ecrite que par l'installeur complet, et le manifeste ne porte ni l'empreinte du modele ni celle du fichier de la porte 2, que la release ne depose pas non plus sur Hugging Face.

**Approach:** L'installeur en ligne propose le modele en option : il le telecharge depuis Hugging Face, verifie son SHA-256, le pose dans `{app}\bin\models` et ecrit la declaration avec `model_sha256`. Le manifeste gagne une section `modele` et une section `porte2` ; le workflow de release calcule l'empreinte du fichier de la porte 2 et le depose sur Hugging Face avec sa declaration, quand le jeton est present.

## Boundaries & Constraints

**Always:** les valeurs du modele viennent de `installer/modele.json` (seule source) ; la forme du manifeste lue par l'installeur reste valable (un champ par ligne, `nom`, `url`, `sha256`) et la boucle des fichiers n'avale ni `modele` ni `porte2` ; un modele absent, refuse ou altere ne pose rien du modele et un octet change fait refuser toute l'installation ; sans modele, l'installation du noyau reussit ; un ancien modele n'est jamais supprime (ni a la mise a jour, ni a la desinstallation) ; la declaration ecrite a la meme forme que celle de la porte 2 (`model`, `prompt`, `model_sha256`) ; `core` et `ai` ne changent pas.

**Never:** pas de telechargement par le noyau ni l'extension ; pas de changement du format de `*.modele.json` ; pas d'envoi reel sur Hugging Face, de jeton, d'etiquette ni de poussee depuis cette session ; pas de modification de l'installeur complet hors ligne sauf lecture de ses valeurs.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Avec modele | manifeste valide, case modele cochee (defaut), modele servi | noyau et modele poses, declaration avec `model_sha256` | — |
| Sans modele | `/MODELE=0` ou case decochee | noyau pose, aucun fichier dans `models`, code 0 | — |
| Octet change | modele servi altere | installation refusee, rien pose (noyau compris), code non nul | message qui nomme le fichier |
| Modele injoignable | URL du modele morte | installation refusee, rien pose | message reseau, rien installe |
| Manifeste sans section `modele` | ancien manifeste | noyau seul, modele non propose | pas d'erreur |
| Mise a jour | ancien modele deja pose | ancien modele et sa declaration intacts | — |

</frozen-after-approval>

## Code Map

- `installer/cassini-en-ligne.iss` -- `PrepareToInstall` (l.~118-210) lit le manifeste par recherche de texte et boucle sur `nom`/`url`/`sha256` ; borner la boucle avant `"modele"`, lire la section a part, `DownloadTemporaryFile(url, nom, sha)` pour l'empreinte ; `[Files]` pose `{tmp}\noyau\*` ; `UninstallDelete` (l.~47) reserve le dossier modele : ne jamais le supprimer. Pas de `[Tasks]`/`[Components]` : ajouter l'option, parametre silencieux `/MODELE=0|1`.
- `installer/manifeste.py` -- `construire()` : ajouter `modele` et `porte2` apres `fichiers`, taille comprise ; lecture de `modele.json`.
- `installer/modele.json` -- url, sha256, taille, `nom_installe` du modele (a ne pas changer).
- `installer/cassini-hors-ligne.iss` (l.~98-101) -- forme de la declaration a reproduire, `EmpreinteModele`.
- `installer/tests/test_manifeste.py`, `test_hors_ligne.py` -- modeles de tests (tmp dir, `sys.path.insert`, garde textuelle de l'iss).
- `.github/workflows/release.yml` -- l.132-205 preuve de l'installeur en ligne (serveur local, manifeste altere) a etendre ; l.213-284 installeur complet ; `gh release create` en fin : le manifeste doit etre fini avant.
- `core/src/commands/model_list.rs` (`ModelDeclaration`, l.~49), `ai/src/lib.rs` l.198-220 (`verify_fingerprint`, `models.tampered`) -- contrat deja teste, a relire seulement.
- `docs/publier-une-version.md` -- documenter l'option, le jeton `HF_TOKEN` et l'essai sur VM propre.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/manifeste.py` et `installer/tests/test_manifeste.py` -- sections `modele` (de `modele.json`) et `porte2` (nom versionne, url Hugging Face, sha256, taille de l'installeur complet fourni) ; forme fixe testee, sections absentes quand non fournies
- [ ] `installer/cassini-en-ligne.iss` -- option modele (case cochee par defaut, `/MODELE=0|1`), telechargement verifie, pose dans `{app}\bin\models`, declaration avec `model_sha256`, rien pose en cas d'echec, modele jamais supprime
- [ ] `installer/tests/test_en_ligne.py` -- garde textuelle de l'iss (declaration egale a celle de la porte 2, nom et empreinte venant de `modele.json`, `/MODELE`, `models` absent de `UninstallDelete`) ; compilation et essai local contre un serveur local si Inno Setup est installe sur le poste
- [ ] `.github/workflows/release.yml` -- empreinte et taille de l'installeur complet, declaration d'empreintes de la porte 2 deposee a cote sur Hugging Face (etape a jeton `HF_TOKEN`, sautee avec un avis si absent), manifeste complete avant `gh release create`, preuve en ligne : avec modele, sans modele, octet change
- [ ] `docs/publier-une-version.md` -- option, jeton, essai VM

**Acceptance Criteria:**
- Given un manifeste avec `modele` et un modele servi localement, when l'installeur tourne en silencieux, then le modele est pose, sa declaration porte `model_sha256` et `cassini.exe run model.list` ne signale pas `models.tampered`.
- Given `/MODELE=0`, when l'installeur tourne, then le noyau est pose et `models` ne contient aucun modele.
- Given un octet change dans le modele servi, when l'installeur tourne, then il sort en erreur et rien n'est pose.
- Given `python -I -m unittest discover -s installer/tests`, then tout passe.

## Implementation Notes

Fait le 2026-11-17 : `manifeste.py` (sections `modele` et `porte2`, flags `--modele`, `--porte2`), `cassini-en-ligne.iss` (case modele cochee par defaut, `/MODELE=0|1`, telechargement verifie, declaration avec `model_sha256`, `UninstallDelete` limite aux fichiers de `bin` pour ne jamais toucher `models`), `release.yml` (preuve avec modele, sans modele, octet change ; manifeste complet avant `gh release create` ; depot Hugging Face si `HF_TOKEN`), doc, tests (53 verts, 1 ignore : Inno Setup absent du poste). Le Pascal n'a jamais ete compile ni lance : premiere execution reelle = recette de release. Le noyau (`core`, `ai`) n'est pas touche.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 1 medium, 2 low patches, 2 rejetes.

| Verdict | Route | Evidence |
|---|---|---|
| medium | patch (fait) | Une mise a jour avec un modele d'un autre nom ecrasait la declaration de l'ancien : elle prend maintenant le nom du modele quand une autre existe deja ; garde de test ajoutee. |
| low | patch (fait) | Test d'`UninstallDelete` avec `` (jamais faux) : motif corrige, verifie sur `Name: "{app}in"`. |
| low | patch (fait) | Modele tenu deux fois sur le disque pendant l'installation : doc, environ 6 Go libres. |
| low | rejete | Case modele affichee avec un ancien manifeste : le manifeste n'est lu qu'apres les pages ; le texte de la page le dit et le noyau s'installe seul, sans erreur. Corriger demanderait de telecharger avant la page. |
| low | rejete | Etape porte 2 sans preuve propre : elle echoue fort (`Stop`, controle des sections) ; une preuve reelle exige un jeton et un depot reel, hors de cette session. |

## Design Notes

Manifeste attendu (extrait, apres `fichiers`) :
```
"modele": {"nom": "cassini-composition-4b-Q4_K_M.gguf", "url": "...", "sha256": "...", "taille": 2783446944},
"porte2": {"nom": "cassini-installeur-complet-0.1.1.exe", "url": "...", "sha256": "...", "taille": 0}
```
(un champ par ligne en vrai, dans l'ordre `nom`, `url`, `sha256`, `taille`). L'installeur coupe le texte au premier `"modele"` pour sa boucle de fichiers.

Non verifiable ici : la VM Windows propre, le noyau gpu qui charge le vrai modele, l'envoi reel sur Hugging Face. Ils vont dans une verification du proprietaire, comme pour 7.2.

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: tout vert
- `cargo test -p cassini-ai plan` -- expected: contrat de declaration inchange

**Manual checks (if no CLI):**
- Sur une VM Windows propre, installer avec puis sans modele, changer un octet du modele servi, lancer la demo.
