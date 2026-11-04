---
title: 'Export à l''échelle sans le mot PDF'
type: 'feature'
ticket: '40'
created: '2026-11-04'
status: 'built'
baseline_revision: 'b66403dad3845da667419dede812c0ecaee0b110'
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

**Problem:** Le modèle de 5.39 refuse « Exporte la carte au 1:25 000. » (« seulement une page A4 paysage ») : toutes les tournures de la famille `export_pdf_scale` du générateur contiennent « PDF », et les refus de format (A3, portrait) de `refus_hors_registre` ressemblent à une demande sans ce mot.

**Approach:** `export_pdf_scale` écrit aussi des demandes sans « PDF » (« Imprime la carte au 1/50000 », « Sors-moi la carte au 25 000e »…), la carte seule et au bout d'une recherche, qui rendent `layout.export_pdf` à `scale`. Les refus de format restent tels quels. Aucun ré-entraînement ici (prochain ré-entraînement groupé).

</frozen-after-approval>

## Implementation Notes

Voie oneshot : un seul fichier de code (`training/src/families.rs`, fonction `export_pdf_scale` et ses tests, ~70 lignes) et la ligne de la famille dans `docs/entrainement-du-modele.md`.

- Ne recopier aucune phrase d'`echelle_utm_reel` E01–E05 : le test de recouvrement compare le texte entier normalisé, chiffres et communes masqués. Éviter donc la tête exacte « Exporte la carte {at} » et « Je voudrais un PDF… » ; « Exporte-moi la carte {at} » est permise.
- Têtes sans « PDF » ajoutées à la carte seule (« Imprime la carte », « Sors-moi la carte », « Exporte-moi la carte », « Fais une mise en page de la carte »…) et à la clause de fin de recherche (« et imprime la carte {at} », « , puis sors la carte {at} »).
- Tests : (1) au jeu par défaut, des exemples `export_pdf_scale` sans « pdf » (insensible à la casse), chacun passe `check_plan` et porte exactement un `layout.export_pdf` avec `scale` ; (2) au jeu par défaut, les refus de format (A3, A0, portrait) gardent la raison `ONLY_A4` ; (3) recouvrement et identité à l'octet : tests existants.

Fait (2026-11-04) : cinq têtes sans « PDF » pour la carte seule (`below(5)` → `below(10)`) et deux clauses de fin de recherche (`below(3)` → `below(5)`) dans `export_pdf_scale` ; deux tests dans `training/src/lib.rs` ; ligne de la famille dans `docs/entrainement-du-modele.md`. Jeu par défaut : 135 exemples `export_pdf_scale`, dont 60 sans « PDF » ; aucun refus de format touché ; deux générations identiques à l'octet (`cmp`). 65 tests verts, recouvrement compris ; clippy propre.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tout vert, `recouvrement` compris
- `cargo clippy -p cassini-training --all-targets -- -D warnings` et `cargo fmt --check` -- expected: aucun avertissement

## Review Triage Log

Passe 1 (quick) : 0 high, 3 medium/low corrigés, 1 low rejeté.

- medium, patch : le test des exports sans « PDF » ne prouvait pas la clause de fin de recherche → il exige désormais un exemple à un appel et un exemple à plusieurs appels.
- low, patch : A0 annoncé mais non vérifié → le test exige un refus A3, A0 et portrait.
- medium, patch : filtre par sous-chaîne « a3 »/« a0 » fragile → filtre sur la famille `refus_hors_registre` et la raison A4 ; aucun `export_pdf_scale` ne nomme un papier ni une orientation.
- low, rejeté : « …la carte …, intitulé « … » » (accord) existait déjà pour les têtes d'avant ; sans effet sur le plan appris (le titre est recopié), et rare.
