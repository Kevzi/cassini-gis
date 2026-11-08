---
title: 'Table de sémiologie et reconnaissance dans le noyau'
type: 'feature'
ticket: '20'
created: '2026-11-08'
status: 'done'
baseline_revision: '1ca5ed8778f040dbc649d4d4cc06cf1e2676eab2'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-styles-topographiques-officiels/change-styles-topographiques-officiels.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le contrat de la mise en forme topographique est figé (11.19). Le noyau ne sait pourtant ni reconnaître le thème d'une couche, ni lui composer un style IGN ou CNIG, ni ranger les couches. L'entrée 21, le geste QGIS, n'a donc rien à appliquer.

**Approach:**
- Le noyau reçoit :
  - une table de sémiologie, où chaque couleur cite sa source ;
  - une reconnaissance déterministe du thème d'une couche, par son nom, son fichier et ses champs ;
  - la composition du `StyleSpec` et du rangement ;
  - la commande `map.topographic_style`.
- Les déclarations de `layer.set_style` et `layer.arrange` entrent au registre. Elles sont `dictated_by_core`, et leur geste viendra en 21.
- La session :
  - lit l'inventaire hors du journal, avant l'exécution de `map.topographic_style` ;
  - le passe à la commande ;
  - dicte, après sa ligne, les `layer.set_style` puis le `layer.arrange` ;
  - fait de même au rejeu pour la lecture, sans rien dicter.
- La commande n'est offerte que si l'extension annonce les trois gestes.
- Tout est testé sans QGIS, avec le faux exécutant des tests.

**Décision du propriétaire (2026-11-08), option (a) :**
- Pour les zones du PLU, les codes exacts de la spécification de symbolisation du CNIG.
- Pour le reste, des couleurs choisies d'après la légende publique de l'IGN : eau bleue, bois verts, bâti gris, courbes orangées. Chacune porte sa source et la mention « d'après la légende IGN ».
- On n'attend pas de table officielle de l'IGN.

</frozen-after-approval>

## Boundaries & Constraints

**Always:**
- La table et les règles vivent dans le noyau (AD-5). Les paramètres envoyés à QGIS sont un `StyleSpec` neutre (AD-22), jamais un nom de classe ni une expression de QGIS.
- Chaque couleur porte en commentaire sa source publique : document IGN ou spécification du CNIG, avec l'adresse et la date de lecture. Aucun fichier de style tiers n'est recopié : le dépôt `cnigfr/DDU-SG5-SYMBOLISATION` n'a pas de licence. Rien d'Esri.
- La reconnaissance est déterministe : même inventaire, même résultat, même ordre.
- L'inventaire injecté n'entre jamais dans le journal : ni les paramètres journalisés, ni le résultat. Le résultat journalisé est exactement `styled`, `unchanged` et `groups`.
- Le rangement liste chaque couche du projet une fois. Les groupes vont de haut en bas : points, lignes, surfaces, « Autres couches » (rasters non reconnus), relief.
- `cargo fmt`, `clippy -D warnings`, la recette du catalogue verte, et les trois fiches passées en `faite` dans le même commit.

**Never:**
- Aucune dépendance nouvelle.
- Aucun geste dans l'extension (c'est 21) ; aucun changement de générateur ni d'entraînement.
- Pas de commande `map.topographic_style` offerte sans les trois gestes annoncés.
- Pas de seuil métier en dur hors de la table.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Villeneuve | Inventaire synthétique : parcelles, zonage PLU (`typezone` U/AU/A/N), gares (points), routes BD TOPO (`importance` 1-6), MNT | PLU en `categories` CNIG ; routes en `categories` par `importance` ; gares en `single` ; MNT en `elevation_ramp` ; groupes Transport, Urbanisme, Relief… ; MNT en dernier | — |
| Annecy | Cours d'eau, plans d'eau, bâti (`usage_1`), communes, deux MNT, ombrage | Eau en bleu (lignes et surfaces), bâti par usage, limites en trait, MNT en rampe, ombrage en `hillshade` au-dessus des rampes | — |
| Inconnue | Couche « export_2024 » sans champ connu | Dans `unchanged` (`reason`) et au groupe « Autres couches » | — |
| `layers` partiel | `layers: [routes]` | Seules les routes sont stylées ; le rangement porte sur tout le projet | — |
| Couche absente | `layers: [x]`, x hors de l'inventaire | Rien n'est dicté | `qgis.layer_missing` |
| Sans QGIS | CLI `run`, ou gestes non annoncés | Commande non offerte | `style.unavailable` |
| Ordre refusé | Le faux QGIS refuse le 2ᵉ `layer.set_style` | Suite arrêtée ; lignes 1 et 2 journalisées | Erreur de l'ordre (AD-21) |
| Dégradé | `map.graduated_style` sur une couche, défauts | Un `layer.set_style` `graduated` (jenks, 5 classes, `yellow_red`) dicté après la ligne | `style.invalid` pour `layer_id` vide ou `class_count` hors de 2 à 12 |
| Rejeu | Journal d'une mise en forme | Ligne mère relue (inventaire relu), lignes dictées rejouées une fois chacune ; `identical` | — |

## Code Map

- `core/src/registry.rs:30-162` -- `CommandSpec` et ses constructeurs. Ajouter un marqueur de commande qui reçoit l'inventaire (sur le modèle de `leave_out_param`) et la liste des gestes requis pour qu'elle soit offerte.
- `core/src/commands/layer_open_vector.rs:30-58` -- modèle de déclaration déléguée et dictée, pour `layer_set_style.rs` et `layer_arrange.rs`.
- `core/src/commands/terrain_slope.rs:113-159` -- modèle de commande du noyau, pour `map_topographic_style.rs` (enregistrement par `inventory::submit!`, sans toucher `commands/mod.rs`).
- `core/src/style/` (nouveau) -- `spec.rs` (types `StyleSpec` serde + JsonSchema, validation `style.invalid`), `table.rs` (thèmes, couleurs sourcées, rampe d'altitudes), `recognize.rs` (thème d'une couche : nom, fichier, champs, valeurs), `arrange.rs` (groupes et ordre).
- `core/src/commands/project_inventory.rs:16-80` -- forme brute de l'inventaire. `values` n'existe que pour les champs texte : la reconnaissance par valeurs ne vaut que pour eux. `core/src/agent_inventory.rs:580-622` -- `geometry()`, `field_type()`, `is_text()`, à réutiliser.
- `core/src/journal/mod.rs:295-330` (`call_logged`, `opener_of`) -- ajouter la dictée des ordres de style après la ligne mère, sur le modèle des ouvertures. `:341-405` (`call_step`) -- lire l'inventaire par `self.router.call("project.inventory", {})` avant l'exécution, l'injecter après `resolve_call`, et retirer du résultat journalisé le champ caché des ordres. `:209-223` -- ensemble des gestes annoncés `with_gestures(...)` (session et rejeu).
- `core/src/journal/mod.rs:963-1030` (`Replay::run`) -- même injection de l'inventaire, à côté de celle de `leave_out_param` (`:1008`). Aucune dictée.
- `core/src/serve.rs:879-886` (drapeaux au `hello`), `:919` (`is_available`) -- l'offre exige les gestes requis. `QgisStub` rend `registry.unavailable`, à traduire en `style.unavailable`.
- `core/src/journal/tests.rs:525-600` (`SessionQgis`), `:439` (`InventoryQgis`) -- faux QGIS à étendre (`layer.set_style`, `layer.arrange`, refus à la demande). `core/src/serve.rs` tests `Client` `:1766`, `hello_with` `:1848`.
- `_bmad-output/.../catalogue-des-commandes.md` -- passer les trois fiches en `faite`, et la ligne de §4 « livrée par » à l'entrée 20 (geste : 21).

## Tasks & Acceptance

**Execution:**
- [x] `core/src/style/spec.rs` -- `StyleSpec` exactement comme son bloc du catalogue, avec validation -- la recette compare le schéma au bloc.
- [x] `core/src/style/table.rs`, `recognize.rs`, `arrange.rs` -- table sourcée, reconnaissance et rangement, avec tests unitaires (matrice ci-dessus, inventaires synthétiques de Villeneuve et d'Annecy dans les tests) -- cœur métier, testé sans QGIS.
- [x] `core/src/commands/map_topographic_style.rs`, `layer_set_style.rs`, `layer_arrange.rs` -- déclarations et commande, résultat public `styled`, `unchanged` et `groups`, plus le champ caché des ordres -- contrat 11.19.
- [x] `core/src/registry.rs`, `core/src/journal/mod.rs`, `core/src/serve.rs` -- injection de l'inventaire (session et rejeu), dictée, offre conditionnelle, `style.unavailable` -- amendement d'AD-30.
- [x] `core/src/journal/tests.rs`, tests de `serve.rs` -- dictée dans l'ordre, journal propre (pas d'inventaire), ordre refusé, rejeu identique sans double application, commande non offerte sans gestes.
- [x] catalogue -- fiches en `faite`, recette verte.

**Acceptance Criteria:**
- Given un projet Villeneuve ou Annecy synthétique, when `map.topographic_style` s'exécute avec un faux QGIS, then le journal contient la ligne mère puis un `layer.set_style` par couche reconnue puis un `layer.arrange`, sans aucun inventaire dans le journal.
- Given ce journal, when il est rejoué, then le rapport est `identical` et chaque style n'est appliqué qu'une fois.
- Given la table, when on la parcourt, then chaque couleur a une source écrite.

## Implementation Notes

- **Construction :** faite dans un worktree à part, pendant qu'un entraînement tournait sur la branche principale ; commit par le workflow après la relecture.
- Variante `graduated` ajoutée à `StyleSpec` sur décision du propriétaire (2026-11-08) ; seuils calculés par QGIS.
- Commande `map.graduated_style` ajoutée sur décision du propriétaire (2026-11-08) ; même dictée que `map.topographic_style`.
- Écart de nom : la session reçoit l'ensemble des commandes annoncées par `with_gestures(...)` au lieu d'un drapeau `with_styling(bool)`, les deux commandes de mise en forme n'exigeant pas les mêmes gestes (`required_gestures()` sur `CommandSpec`).
- **Codes du CNIG :** à relever dans la spécification publique de symbolisation du PLU (site cnig.gouv.fr ; dépôt `cnigfr/DDU-SG5-SYMBOLISATION` pour la documentation, sans en copier aucun fichier). Si un code n'est pas trouvable, prendre une couleur d'après la légende publique et le noter dans la source, plutôt que d'inventer un code « officiel ».

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: vert, recette du catalogue comprise
- `cargo fmt --check && cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: propre
- `cargo deny check licenses` -- expected: aucune dépendance nouvelle

## Review Triage Log

Lentilles : blind-hunter (15), edge-case-hunter (9), verification-gap (1), intent-alignment (descriptif). Correctifs appliqués par l'agent de construction ; le noyau est vert (605 tests unitaires, plus les suites d'intégration ; `fmt` et `clippy` propres).

- medium, patch : les vecteurs non reconnus, souvent des résultats de l'utilisateur, finissaient sous les surfaces. « Autres couches » (vecteurs) est maintenant en haut, et « Autres rasters » juste au-dessus du relief. Essai ajouté.
- medium, patch : au rejeu, `layer.arrange` omettait les couches créées par un rejeu précédent, que QGIS aurait refusées. Une couche absente de la liste reste maintenant au-dessus des groupes ; `arrange.invalid` ne vaut plus que pour un groupe vide ou un doublon.
- medium, patch : la reconnaissance lisait tout le chemin `source`, dossiers compris. Elle ne lit plus que le nom du fichier et un `layername=` ; essai sur un chemin absolu.
- medium, patch : une pente ou une exposition pouvait être prise pour un MNT. Les mots des dérivés l'emportent ; essai avec `pente_mnt.tif`.
- low, patch : `importance` passait avant les noms de cours d'eau et de courbes ; `eau` reconnaissait `chateau_d_eau` ; les sous-zones du PLU (`1AU`, `Ah`, `Nh`) restaient sans couleur. Ordre corrigé, mots entiers seulement, préfixes du PLU. Essais ajoutés.
- low, patch : `layers: []`, un inventaire sans `layers` et une couche sans `layer_id` passaient en silence. Ils sont refusés (`style.invalid`, `style.unavailable`).
- low, patch : `map.graduated_style` acceptait un `layer_id` vide, et n'avait pas d'essai par le canal. Les deux sont réglés.
- low, patch : un ordre non déclaré était perdu sans bruit. Il donne maintenant `registry.invalid_order`.
- medium, patch : le rejeu ne vérifiait pas les gestes. Il rend maintenant `style.unavailable` à la bonne étape (`Replay::with_gestures`) ; deux essais.
- medium, patch : la vérification ne couvrait pas l'ensemble des couches posées par le rejeu. Il est vérifié, puis un second rejeu identique est fait.
- low, patch : essai ajouté pour des gestes annoncés en partie.
- low, patch : `pending_orders` est remis à zéro au début de chaque étape. Le défaut était défensif seulement : la fuite n'avait pas lieu, puisque chaque étape réussie le récrivait.
- medium, patch : attribution de ColorBrewer, demandée par sa licence Apache-2.0. Ajoutée dans `NOTICE` et dans le commentaire de `RAMPS`.
- low, patch : le commentaire des paliers d'altitude était faux pour une plaine. Il est corrigé.
- low, patch : `graduated` et `opacity` n'étaient pas précisés dans la fiche. C'est fait.
- false : exclusion du serveur MCP en mode relais. Le mode relais offrait déjà les commandes quand la session les marque disponibles ; la règle est écrite dans les fiches.
- defer : rampe étirée au min et au max réels du MNT ; milieu clair de `red_blue` ; transparence CNIG du PLU ; refus périmé du générateur ; dégradé inversé. Tous notés dans `deferred-work.md`.
- defer : couleur des bois (`#dfe8d5`, pâle, tirée de Plan IGN). À soumettre au propriétaire au PDF de 11.21.
- low, rejeté : couleurs IGN relevées dans le style public de Plan IGN plutôt qu'à l'œil. Elles sont plus fidèles que l'option (a) ne l'exigeait, et chacune garde sa source.
- low, rejeté : les essais côté noyau ne prouvent pas qu'une extension sait dessiner le `StyleSpec`. C'est l'objet de 11.21.
- low, patch (vérification finale) : deux essais de la CLI comptaient encore les commandes de mise en forme parmi celles du mode autonome (`core_names` de `cli/tests/mcp.rs`, essai relais) et dans la parité `run`/`serve`/MCP (`cli/tests/parity.rs`). Le filtre `required_gestures()` est ajouté. Vérifié sous Windows (`cargo test -p cassini -p cassini-ai`, `fmt`, `clippy`) : tout est vert. `cassini-training` : 86 essais verts.
