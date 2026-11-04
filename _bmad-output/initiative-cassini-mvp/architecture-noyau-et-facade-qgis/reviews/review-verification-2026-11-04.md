# Relecture de vérification — amendement d'AD-24 (Genres), 2026-11-04

**Lentille :** chaque décision engagée est-elle vérifiée contre le web, le dépôt ou
le code existant, plutôt qu'affirmée de mémoire ?
**Objet :** la phrase ajoutée à AD-24 › Genres — « Un raster délégué se désigne par
son identifiant de couche (AD-9), ou par un chemin relatif au projet quand il sort
d'une étape du noyau : une référence `step:k` vers le champ `file` d'une commande
`runs_in: core` (AD-9, AD-23 ; amendé le 2026-11-04). »
**Cohérence examinée avec :** AD-9, AD-10, AD-11, AD-19, AD-20, AD-23.
**Contrat non modifié.**

## Verdict

**Réalisable, avec réserves.** Les faits sur lesquels l'amendement s'appuie sont
vrais dans le code : `terrain.slope` déclare bien un champ de sortie `file`, la
référence `step:k.file` passe le contrôle du plan, et QGIS accepte un chemin de
fichier là où il attend une couche raster (vérifié dans la documentation). En
revanche, l'amendement **ne dit pas comment cette référence est journalisée**.
Avec le code actuel, le journal y écrit le chemin du fichier produit à
l'exécution d'origine, et non un repère `out:N.n`. Au rejeu, l'étape suivante
relit donc ce fichier d'origine au lieu de celui qu'elle vient de recalculer, ce
qui contredit AD-11. Deux autres points restent ouverts : la forme du paramètre
« identifiant ou chemin », et le ticket 13 resté à trancher.

## Ce qui a été vérifié, et comment

| Affirmation de l'amendement | Vérifié contre | Résultat |
|---|---|---|
| `terrain.slope` est `runs_in: core` | `core/src/commands/terrain_slope.rs:147` (`.runs_in(RunsIn::Core)`) | Confirmé |
| Sa sortie a un champ `file` déclaré | `TerrainSlopeOutput { pub file: String, … }`, dérive `JsonSchema` (l. 45-47) | Confirmé : `file` est une propriété de premier niveau du schéma de sortie |
| `step:k` peut viser ce champ | `core/src/plan.rs` : `parse_reference` admet `step:<k>.<champ>` ; `check_plan` lit `properties.<champ>` du schéma de sortie de l'étape *k* et compare au schéma du paramètre visé par `normalize`, sans `description` ni `title` (l. 325-326) | Confirmé, à condition que le paramètre visé soit une `String` simple (sans `pattern`, `format`, `minLength`, que `normalize` garde) |
| Résolution à l'exécution | `core/src/serve/plans.rs:308` (`resolve_references`) puis `outputs.push(value)` (l. 331) | La valeur substituée est le résultat **brut** : chemin **absolu** `…/<projet>.cassini/<N>-terrain.slope.tif` (test `a_slope_in_a_plan_returns_its_absolute_existing_file`, `serve.rs:3168`) |
| « chemin relatif au projet » dans le journal | `journal/mod.rs`, `resolve_call` / `resolve_param` / `resolve_path` | Vrai **seulement si** le paramètre raster de la commande `raster.*` est déclaré dans `path_params` : le chemin absolu devient alors `<projet>.cassini/<N>-terrain.slope.tif`, en POSIX, marqué portable. S'il ne l'est pas, le chemin absolu entre tel quel au journal, ce que AD-9 interdit |
| QGIS lit un raster désigné par un fichier | Documentation de QGIS, *Processing › Using processing algorithms from the console* (via Context7, `/qgis/qgis-documentation`) : « Raster Layer, Vector Layer or Table … or a filename (if the corresponding layer is not opened, it will be opened but not added to the map canvas) » | Confirmé pour la branche principale de la documentation. La lecture n'a pas été refaite séparément pour 3.44 et pour 4.x : c'est à la recette sur les deux séries de le confirmer (AD-18, AD-25) |
| Pourquoi pas l'identifiant de couche | `qgis-plugin/cassini_qgis/raster_output.py` : le raster du noyau n'est ouvert dans le projet qu'à réception de `PLAN_DONE` ou `RUN_DONE`, donc **après** la fin du plan | Confirmé : au milieu d'un plan, aucun `layer_id` n'existe pour la pente. La voie par chemin est donc nécessaire, et le motif de l'amendement tient |

L'amendement ne nomme **aucune bibliothèque ni version**, si bien que rien n'y
risque d'être daté de ce côté-là. La seule affirmation technique extérieure au
dépôt est le comportement de QGIS face à un chemin de fichier : elle est vérifiée
ci-dessus.

## Constats

### V1 — Le rejeu ne se raccorde pas à la sortie recalculée (contredit AD-11) — **élevé**

AD-11 demande que les sorties soient désignées dans le journal par des **repères
locaux à l'étape** (`out:k.n`) et que « les étapes suivantes se résolvent par le
repère ». Or `resolve_references` substitue le chemin absolu avant que le journal
ne voie l'appel. Le journal convertit ensuite ce chemin en chemin relatif au
projet (`<projet>.cassini/3-terrain.slope.tif`), mais jamais en `out:3.1` : aucun
code ne fait cette conversion (`grep "out:"` ne trouve que la lecture du repère,
`journal/mod.rs:555` et `583`). Trois conséquences :

1. Au rejeu, la pente est recalculée dans `replay/`, mais l'étape `raster.*`
   suivante lit la pente de l'exécution d'origine, restée dans le dossier de
   travail. Le rejeu ne vérifie donc plus la chaîne entière.
2. L'entrée est journalisée avec sa taille et sa date (`input_entry`). Si le
   dossier de travail est nettoyé, ou si le projet est déplacé sans son dossier
   `.cassini`, le rejeu s'arrête sur « l'entrée a changé ou disparu », alors même
   qu'il vient de régénérer ce fichier.
3. Le numéro *k* de `step:k` est la place de l'étape dans le **plan** ; le *N* du
   nom de fichier et du repère `out:N.n` est le numéro dans le **journal**, qui
   continue d'une session à l'autre. Les deux numérotations diffèrent dès qu'une
   commande a été journalisée avant le plan. La clé `plan_step` (5.34) permet de
   passer de l'une à l'autre, mais rien ne le fait aujourd'hui.

**À ajouter à l'amendement, sans re-décider AD-11 :** le noyau journalise un
paramètre résolu depuis `step:k.file` sous la forme du repère `out:N.n` de l'étape
qui l'a produit. Ce n'est pas un chemin. Le test correspondant : rejouer, sans le
fichier d'origine, un plan `terrain.slope` → `raster.*`.

### V2 — « identifiant de couche, ou chemin » : la forme du paramètre n'est pas fixée — **moyen**

Le résolveur du journal traite chaque paramètre de `path_params` comme un chemin
(`resolve_path`). Un paramètre unique qui accepterait l'un ou l'autre ferait
passer un identifiant de couche pour un chemin relatif au projet, avec une
fausse entrée `inputs`. À l'inverse, un paramètre absent de `path_params`
laisserait le chemin absolu entrer au journal, ce qu'interdit AD-9. Ni AD-24 ni
AD-22 ne disent laquelle des formes suivantes retenir :

- deux paramètres distincts et exclusifs, un identifiant de couche et un fichier,
  ce dernier seul déclaré dans `path_params` ;
- ou un objet typé avec un discriminant.

C'est une décision de contrat de paramètres. Elle appartient au contrat, pas au
premier ticket qui en aura besoin. Elle conditionne aussi le schéma visé par
`step:k.file` : le contrôle de type de `check_plan` exige une `String` nue.

### V3 — Le ticket 13 (`raster.reclassify`) porte encore l'inconnue que l'amendement tranche — **faible**

`epic-catalogue-etendu-qgis/tickets.toml`, entrée 13, `unknown = "Enchainer pente
puis reclassement, a trancher par le proprietaire…"`. L'amendement d'AD-24 retient
l'option « accepte aussi un chemin relatif ». Pourtant, le diff de travail de
`tickets.toml` ne touche que l'entrée 23. L'`unknown` devrait devenir une `notes`
« Decision: 2026-11-04 », comme pour l'entrée 23, faute de quoi la porte d'entrée
du ticket reste fermée sur une question déjà réglée.

### V4 — Le modèle n'a jamais vu `step:k.file` — **faible, à prévoir**

Le prompt et les exemples de `ai/src/plan.rs` (l. 143, 145, 409) ne connaissent que
`step:k.layer_id`. Pour composer « pente puis reclassement », le modèle devra
apprendre la forme `step:k.file`. Cela passe par des exemples d'entraînement
(règle « modèle entraîné, pas de pansements »), au ré-entraînement groupé 5.41 ou
au suivant. AD-23 n'en est pas touché : une référence `step:` est un chemin déjà
connu de la session.

### Cohérence avec les autres décisions

- **AD-9** : cohérent dans l'esprit. L'exception du 2026-11-04 (résultat brut avec
  chemins absolus vers l'extension) est bien ce qui fait passer un chemin absolu
  dans `order.execute`, ce que l'exception du 2026-10-15 autorise déjà. Le journal
  reste relatif **si** V2 est réglé dans le sens de `path_params`.
- **AD-10** : respecté. Le fichier visé est nommé par le noyau dans
  `<projet>.cassini/` (`locate`, `journal/mod.rs:386`), et l'extension n'invente
  rien.
- **AD-11** : en tension, voir V1.
- **AD-19** : sans objet direct. Le raster de pente garde le système du MNT, et
  aucune coordonnée ne traverse le canal. À noter : `raster.*` étant délégué,
  QGIS pourrait reprojeter à la volée, ce qui reste conforme puisque la
  reprojection appartient à QGIS.
- **AD-20** : respecté. Le noyau n'ouvre pas le raster pour le compte de
  `raster.*` ; seul le chemin passe à QGIS. La règle « aucune commande `raster.*`
  ne refait un calcul de `terrain.*` » est conservée.
- **AD-23** : respecté. `check_plan` valide la référence et le type avant
  l'affichage, et le modèle n'écrit aucun chemin.

## Recommandation

Compléter la phrase d'AD-24 de deux précisions, par un amendement daté : (a) au
journal, la référence devient le repère `out:N.n` d'AD-11, jamais un chemin ;
(b) la forme du paramètre « identifiant ou fichier » (proposition : deux
paramètres exclusifs, le fichier dans `path_params`). Lever ensuite l'`unknown` de
l'entrée 13. Le reste de l'amendement est vérifié et peut rester tel quel.
