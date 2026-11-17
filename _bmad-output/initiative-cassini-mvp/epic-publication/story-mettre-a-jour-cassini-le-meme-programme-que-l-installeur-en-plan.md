---
title: 'Mettre a jour Cassini, le meme programme que l''installeur en ligne'
type: 'feature'
ticket: '5'
created: '2026-11-17'
status: 'built'
baseline_revision: 'd0eae55c3babf5b471a619dc2e246e561c160e1d'
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

**Problem:** Relancer l'installeur en ligne reinstalle tout sans comparer les versions, et un nouveau modele de meme nom ecrase l'ancien : `installer/modele.json` fixe `nom_installe = cassini-composition-4b-Q4_K_M.gguf` pour toutes les versions, si bien que le modele 0.2.0 publie par 7.15 remplacerait le 0.1.0, contre la regle « un ancien modele n'est jamais supprime ». Il n'y a ni lancement « Mettre a jour Cassini » ni test d'un installe a jour.

**Approach:** Le meme programme gagne un mode `/MISEAJOUR` et un raccourci « Mettre a jour Cassini » du menu Demarrer : il lit le manifeste, compare a la version installee, demande confirmation, installe le noyau et un nouveau modele sous son propre nom, a cote des anciens, et ne fait rien quand l'installe est a jour. Decision : le nom installe du modele porte sa version.

## Boundaries & Constraints

**Always:** une seule source d'installation (`cassini-en-ligne.iss`, meme AppId) ; aucune mise a jour automatique ni en arriere-plan, tout se lance a la main ; chaque fichier verifie par SHA-256 avant la pose, rien n'est pose si l'un echoue ; aucun fichier de `models` n'est jamais supprime ni ecrase ; `installation.json` garde ses deux cles (`version`, `installe_le`), `installe_le` devenant la date de la derniere mise a jour ; installe a jour : rien telecharge, rien reecrit, code 0 ; Cassini non installe : message clair, rien pose ; fichier du noyau en cours d'usage (QGIS ouvert) : message qui dit de fermer QGIS, rien pose ; `core`, `ai` et l'extension ne changent pas.

**Never:** pas de verification automatique des mises a jour ; pas de code reseau dans le noyau ou l'extension ; pas de nouveau modele sur un nom deja pris par un autre fichier ; pas de publication, d'etiquette ni de push depuis la session.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Mise a jour du noyau | installe 0.1.0, manifeste 0.1.1 | confirmation, noyau pose, `installation.json` a 0.1.1 avec la date du jour | — |
| Deja a jour | installe = manifeste, modele deja present | rien telecharge ni reecrit, message « Cassini est a jour », code 0 | — |
| Version installee plus recente | installe 0.2.0, manifeste 0.1.1 | traite comme a jour | — |
| Nouveau modele | meme version, modele du manifeste absent de `models` | modele pose sous son nom versionne, ancien gguf et sa declaration intacts, nouvelle declaration a son nom | — |
| Empreinte fausse | un fichier altere | rien pose, ancienne installation intacte | message qui nomme le fichier |
| Non installe | pas de `installation.json` ni de noyau | message « Cassini n'est pas installe », rien pose | code non nul en silencieux |
| Version illisible | `installation.json` absent ou invalide mais noyau present | mise a jour proposee | — |
| Silencieux | `/MISEAJOUR /VERYSILENT` | pas de dialogue, memes regles | — |

</frozen-after-approval>

## Code Map

- `installer/cassini-en-ligne.iss` -- `[Setup]` (AppId commun avec la porte 2), `[Files]` (noyau l.~42, modele l.~47), `[UninstallDelete]`, `InitializeWizard`, `PrepareToInstall` (lit le manifeste, `VersionInstallee` = version du manifeste, boucle des fichiers puis section `modele`), `CurStepChanged` (declaration, puis `installation.json` toujours reecrit), `ValeurApres`, `DateIsoUtc`. Ajouter : `/MISEAJOUR`, lecture de `{app}\bin\installation.json`, comparaison numerique, copie de l'installeur `{app}\mettre-a-jour-cassini.exe` (`Source: "{srcexe}"`) et `[Icons]` dans `{autoprograms}` avec `Parameters: "/MISEAJOUR"`.
- `installer/modele.json` -- `nom_installe` : devient `cassini-composition-4b-0.2.0-Q4_K_M.gguf` ; le test d'egalite avec le nom du fichier telecharge doit suivre.
- Noms a suivre : `.github/workflows/release.yml` (preuve en ligne et appel de `telecharger_modele.py --sortie`), `installer/cassini-hors-ligne.iss` (`NomModele`, `DestName`), `installer/tests/test_en_ligne.py`, `installer/tests/test_hors_ligne.py`, `docs/publier-une-version.md`. La declaration de la porte 2 garde son nom fixe `cassini-composition-4b.modele.json`, le nom du modele etant dans son contenu.
- `installer/manifeste.py` -- `construire(version, ...)` prend la version en parametre : la preuve d'un « ancien » manifeste passe par la, sans toucher a Cargo.
- `qgis-plugin/cassini_qgis/installation.py` (l.29-44) -- lit seulement `version` et `installe_le` ; ne pas changer.
- `qgis-plugin/cassini_qgis/models.py`, `core/src/commands/model_list.rs` -- le panneau liste tout GGUF du dossier `models` : aucun changement, a lire seulement.
- `installer/tests/test_manifeste.py`, `test_en_ligne.py` -- modeles de gardes textuelles ; la logique de comparaison de versions est reproduite en Python pour une table de cas testee sans Inno Setup.

## Tasks & Acceptance

**Execution:**
- [ ] `installer/modele.json` et ses suiveurs (`cassini-hors-ligne.iss`, `release.yml`, tests, doc) -- nom installe versionne, un seul nom partout
- [ ] `installer/cassini-en-ligne.iss` -- mode `/MISEAJOUR`, version installee, comparaison, confirmation, a jour sans effet, non installe, copie `mettre-a-jour-cassini.exe` et raccourci, `installe_le` mis a jour, modele versionne a cote de l'ancien
- [ ] `installer/tests/test_en_ligne.py` -- gardes textuelles (parametre, `[Icons]`, copie de l'installeur, aucun `DeleteFile` ni `DelTree` sur `models`) et table de comparaison des versions reproduite en Python
- [ ] `.github/workflows/release.yml` -- preuve : installer une version A, `/MISEAJOUR` contre un manifeste B (version plus haute, modele versionne) puis contre le meme manifeste ; verifier noyau, `installation.json`, deux modeles, deux declarations, `model.list` sans `models.tampered`, et l'absence d'effet au second passage
- [ ] `docs/publier-une-version.md` -- raccourci, `/MISEAJOUR`, nom versionne du modele, essai en VM

**Acceptance Criteria:**
- Given une version installee plus ancienne que le manifeste, when `/MISEAJOUR /VERYSILENT` tourne, then le noyau est remplace, `installation.json` porte la nouvelle version et le modele precedent reste, avec sa declaration.
- Given un installe a jour, when `/MISEAJOUR` tourne, then rien n'est telecharge et `installation.json` n'a pas change.
- Given `python -I -m unittest discover -s installer/tests`, then tout passe.

## Implementation Notes

Fait le 2026-11-17 : nom installe du modele versionne (`modele.json`, installeur complet, `telecharger_modele.py`, recette, doc) ; `cassini-en-ligne.iss` : `/MISEAJOUR`, version installee lue dans `installation.json`, comparaison, confirmation, « a jour » sans effet (sortie code 0 avant tout telechargement), non installe, detection de QGIS ouvert, modele pose avec `onlyifdoesntexist`, copie de l'installeur et raccourci du menu Demarrer ; preuve dans `release.yml` ; tests (69 verts, 1 ignore : Inno Setup absent). Le script Pascal n'a jamais ete compile ni lance : premiere execution reelle = recette ou VM. Ecart a la note de conception : la copie de l'installeur n'est pas remplacee quand la mise a jour tourne depuis elle (un executable en cours ne s'ecrase pas), elle reste celle de la derniere installation par l'installeur d'origine.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 3 medium, 7 low.

| Verdict | Route | Evidence |
|---|---|---|
| medium | patch (fait) | L'installeur complet ecrasait la declaration d'un ancien modele de nom different : meme regle que l'installeur en ligne (nom de la declaration tire du modele si une autre existe) ; garde de test. |
| medium | patch (fait) | Renommage de `bin` en `bin-verif` non remis en cas d'echec : refus avec message pour renommer a la main, rien n'est pose ; message normal elargi (« toute fenetre ouverte sur le dossier ») ; garde de test. |
| medium | patch (fait) | Script sans BOM UTF-8 avec accents (nom du raccourci) : BOM ajoute, tests en `utf-8-sig`, garde de test. |
| low | rejete | Copie de l'installeur jamais rafraichie depuis le raccourci : un executable en cours ne s'ecrase pas ; consigne dans les notes. |
| low | rejete | `ExitProcess(0)` laisse le dossier temporaire d'Inno : sans gravite, le code 0 est prouve par la recette. |
| low | defer | En interactif, « a jour » ou « non installe » n'apparait qu'apres les pages d'accueil : ergonomie a juger sur la VM du proprietaire. |
| low | rejete | Miroir Python de la comparaison (hex de `StrToIntDef`, pre-versions) : les versions viennent de Cargo, trois nombres ; une pre-version illisible est proposee, comme prevu. |
| low | rejete | Modele deja present non retelecharge meme hors `/MISEAJOUR` : c'est la regle du plan (nom versionne = deja la). |
| low | rejete | Preuve de la recette : version illisible non jouee, absence de telechargement deduite des empreintes ; couverture suffisante pour une recette, le reste est a la VM. |
| low | rejete | `publier.ps1` et `entrainement-du-modele.md` citent le nom non versionne du modele installe sur le poste : c'est le nom reel de ce fichier chez le proprietaire. |

## Design Notes

Compare : trois nombres separes par des points, parties manquantes a 0 ; une version illisible vaut « mise a jour proposee ». « A jour » vaut : version du manifeste inferieure ou egale a l'installee, et fichier du modele du manifeste deja present (ou pas de section `modele`). Le nom versionne du modele suffit a savoir si le fichier est deja la : pas de calcul d'empreinte de 2,8 Go. La copie de l'installeur dans `{app}` est celle de la derniere installation ; chaque mise a jour la remet a niveau.

Non verifiable ici : raccourci du menu Demarrer, texte francais des dialogues, noyau verrouille par QGIS ouvert, panneau qui montre les deux modeles ; tout cela revient a l'essai du proprietaire en VM.

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: tout vert

**Manual checks (if no CLI):**
- VM : installer, lancer « Mettre a jour Cassini » contre un manifeste plus recent, relancer le panneau et choisir l'ancien modele ; relancer la mise a jour : « Cassini est a jour ».
