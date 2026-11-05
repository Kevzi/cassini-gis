---
title: 'Refuser les MNT en pseudo-mètres et rendre la pente en degrés portable'
type: 'bugfix'
ticket: '11'
created: '2026-11-05'
status: 'built'
baseline_revision: '147695444eedc014b2fc017ec55849ad84529394'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un MNT en EPSG:3857 (ou tout code projeté hors de la liste fermée des systèmes métriques connus) est accepté dès que sa clé d'unité vaut le mètre : pente, distances, courbure et rayon du champ de vision sont faussés sans message. Et la pente en degrés passe par `f64::atan` de la libm du système, non garantie identique entre Windows et Linux, ce qui peut faire diverger un rejeu.

**Approach:** Dans `check_crs` (`core/src/dem.rs`), n'accepter un code projeté que s'il est dans `is_known_metric`, clé d'unité au mètre ou absente ; tout autre code rend `dem.not_projected`, message en français qui nomme `EPSG:<code>` et invite à reprojeter dans QGIS. La pente en degrés utilise `libm::atan` (crate `libm`, MIT OR Apache-2.0, sans dépendance, accord redemandé au build), et un test fige l'empreinte des octets de la pente en degrés de la colline. Décisions du propriétaire (2026-11-05) : refus et non avertissement ; liste fermée ; `libm` ; le noyau ne reprojette pas (AD-19).

</frozen-after-approval>

## Implementation Notes

Voie courte : environ 80 lignes dans `core/src/dem.rs`, `core/src/commands/terrain_slope.rs`, `terrain_line_of_sight.rs`, `terrain_viewshed.rs`, `core/Cargo.toml`. Le rédacteur GeoTIFF pose déjà la clé d'unité au mètre : écrire un `Dem { epsg: 3857, .. }` donne exactement le cas à refuser. Les trois commandes ouvrent le MNT par `Dem::open` avant tout contrôle des points, donc un seul contrôle couvre les trois. La CI lance `cargo test -p cassini-core` sous Linux (job `noyau`) et sous Windows (job `hors-ligne-windows`) : le test des octets figés y tourne des deux côtés. Risque high : relecture `thorough` (CLAUDE.md §8, rétro A6) au lieu de la relecture `quick` de la voie courte.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` -- expected: tout vert, dont les nouveaux tests 3857/3395 et l'empreinte de la pente
- `cargo clippy -p cassini-core --all-targets --locked -- -D warnings` et `cargo fmt --all --check` -- expected: aucun avertissement
- `cargo deny check licenses` -- expected: `libm` acceptée

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). high 0, medium 3, low 6, false 5, maybe-false 0.

- medium, patch — `Cargo.lock` : `cssparser-macros` passé de `syn 2.0.119` à `syn 3.0.6`, sans lien avec `libm`. Hunk rétabli ; `cargo metadata --locked` passe, seul `libm` s'ajoute.
- medium, patch — le test des octets figés ne distingue pas `libm::atan` de `f64::atan` (l'arrondi en f32 absorbe un ulp). Commentaire du test rendu exact ; version de `libm` figée (`=0.2.15`). Empreinte vérifiée à la main sous Windows (`cargo.exe test`, 2026-11-05), identique à Linux.
- medium, defer — liste fermée sans des systèmes métriques légitimes (EPSG:9794, 3035, UTM outre-mer 5490, 2972, 2975), ni page de doc de la liste. La décision du propriétaire renvoie ces ajouts à un ticket à part.
- low, patch — les tests de ligne de vue et de champ de vision ne vérifiaient pas le conseil QGIS : assertion ajoutée.
- low, defer — un journal de pente en degrés écrit avant ce changement peut rendre `replay.diverged` si `libm` diffère de la libm du système sur ce MNT ; refus bruyant, pas silencieux.
- low, rejeté — branche `None if is_known_metric` devenue redondante, deux messages selon la présence de la clé d'unité : les deux nomment l'EPSG et invitent à reprojeter, le test existant fige le message sans clé.
- low, rejeté — le test de ligne de vue nettoie son dossier à la main (pas de `Scratch` dans ce fichier) : un dossier temporaire laissé seulement en cas d'échec.
- low, rejeté — FNV-1a réécrit dans le test au lieu de réutiliser `Fnv` du journal, privé à son module ; dix lignes de test.
- low, rejeté — cas 3857 sans clé d'unité non testé : il suit la branche déjà testée (2263 sans clé).
- false — le test du plan incliné utilise `f64::atan` pour l'attendu : tolérance 1e-4, insensible à un ulp.
- false — le message ne nomme aucun système accepté : `REPROJECT_HINT` cite Lambert-93.
- false — plan en mode 100755 : tout fichier de `/mnt/c` sous WSL l'est.
- false — `ticket: '11'` au lieu de `4.11` : le workflow demande l'`id` de `find`.
- false — `review` et `lenses_ran` vides : remplis avant le lancement des lentilles (diff pris juste avant).
