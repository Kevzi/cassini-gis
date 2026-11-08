---
title: 'Contrat de la mise en forme topographique'
type: 'feature'
ticket: '19'
created: '2026-11-08'
status: 'done'
baseline_revision: 'e074bc1bf5099c6e2fca4220391550d1bd45f295'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le changement de cap du 2026-11-08 (`../change-styles-topographiques-officiels/change-styles-topographiques-officiels.md`) ajoute la mise en forme topographique des couches, mais le catalogue figé, l'architecture et la spec ne la connaissent pas encore. Les entrées 20 et 21 ne peuvent pas s'écrire contre un contrat absent.

**Approach:** Documents seulement. Le catalogue reçoit le type nommé `StyleSpec` (union `single`, `categories`, `elevation_ramp`, `hillshade`) et trois fiches planifiées : `map.topographic_style` (noyau), `layer.set_style` et `layer.arrange` (QGIS, dictées). Cette dernière vient de la décision du propriétaire, après l'approbation : ranger les couches en groupes par thème et dans l'ordre d'affichage. L'architecture reçoit les amendements d'AD-30 (plusieurs ordres dictés pour une demande) et d'AD-24 (genre de sortie `none`). La spec reçoit CAP-5 et la question ouverte des schémas français, tranchée pour la symbologie. La recette `core/tests/catalogue.rs` connaît `StyleSpec` et reste verte.

</frozen-after-approval>

## Implementation Notes

Route courte : trois documents et une ligne de la recette (`NAMED_TYPES`). Une fiche cassée exprès (`StyleSpek`) fait échouer la recette, qui lit donc bien les nouvelles fiches. Le changement de cap note la décision 5 (rangement) et la fiche `layer.arrange`.

## Verification

**Commands:**
- `cargo test -p cassini-core --test catalogue` -- expected: vert (27 essais)

Approuvé par le propriétaire le 2026-11-08 (« ok j'approuve »), après les corrections de la relecture ci-dessous.

## Review Triage Log

- medium, patch : l'amendement présentait `project.inventory` comme un ordre dicté. C'est une lecture hors du journal, comme pour `plan.propose`, et la fiche reste `dictated_by_core: false`.
- medium, patch : le rejeu n'était pas écrit, et on risquait d'appliquer les styles deux fois. Chaque ligne est rejouée seule : `map.topographic_style` relit l'inventaire et compare son résultat sans rien dicter, et les lignes dictées sont rejouées avec leurs paramètres. C'est écrit dans la fiche et dans AD-30.
- medium, patch : aucune conduite sans QGIS. La commande n'est pas offerte, et elle rend `style.unavailable` quand on l'appelle quand même.
- low, patch : deux codes pour une couche absente. `qgis.layer_missing` partout.
- low, patch : le paramètre de `layer.set_style` s'appelait `layer`. Il devient `layer_id`, comme dans le reste du catalogue.
- medium, patch : une couche absente de `layer.arrange` « gardait sa place, au-dessus des groupes », contradictoire et dangereux (un MNT non reconnu recouvrait tout). Le noyau range maintenant toutes les couches, une couche non reconnue va au groupe « Autres couches » placé selon sa sorte, et une liste incomplète est refusée.
- low, patch : les erreurs de rangement réutilisaient `style.invalid`. Elles ont leur code, `arrange.invalid`.
- low, patch : `StyleSpec` interdisait une surface sans contour, et `other` n'avait pas de `label`. `stroke` devient facultatif (au moins l'un des deux), `other` reçoit `label`, et le bloc du catalogue fait foi sur l'esquisse du changement de cap.
- low, patch : `hillshade` ne disait pas sur quoi il porte. Il s'applique à un raster d'ombrage déjà présent et ne calcule rien. Une rampe ou un ombrage va aux rasters, un symbole aux vecteurs.
- medium, patch : l'amendement d'AD-24 ne couvrait ni l'entrée ni la chaîne. Entrée `project`, sortie `none`, ligne vers un geste fixe de l'API de projet, et le même état au rejeu par les paramètres journalisés.
- low, patch : le changement de cap décrivait encore deux commandes. §3, §4.1, §4.3, §4.4, §5 et le frontmatter sont mis à jour.
- low, patch : les entrées 19 à 21 de `tickets.toml` ne parlaient pas du rangement. Elles sont mises à jour.
