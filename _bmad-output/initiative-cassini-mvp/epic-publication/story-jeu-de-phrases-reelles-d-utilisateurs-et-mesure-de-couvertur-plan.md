---
title: 'Jeu de phrases réelles d''utilisateurs et mesure de couverture'
type: 'feature'
ticket: '11'
created: '2026-11-20'
status: 'built'
baseline_revision: '2240fa97a2ccfbb61345e2f6023c15bff3a65521'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'none'
review_source: 'auto'
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** CAP-14 mesure la couverture du registre par des phrases réelles d'utilisateurs, mais aucun jeu de telles phrases ni aucune mesure n'existe ; 10.8 en dépend.

**Approach:** Écrire un jeu de phrases de sources publiques, avec la source de chacune, et mesurer hors ligne sa couverture par le registre actuel (plan valide ou question justifiée), avec le harnais de l'épreuve du modèle local.

## Boundaries & Constraints

**Always:** Sources publiques seulement, source et licence consignées par phrase, phrases réécrites quand la licence l'exige ; hors ligne ; rien d'Esri ni de militaire.

**Never:** Code de collecte, téléchargement à l'exécution, entrée du jeu dans l'entraînement.

</frozen-after-approval>

## Tasks & Acceptance

**Acceptance Criteria:**
- Given le jeu de phrases, when le harnais le mesure, then un rapport donne pour chaque phrase sa source, son verdict et la commande manquante, et la part couverte.

## Implementation Notes

Construit sans relecture ni plan préalable par un agent non supervisé, commit `df20812` : `ai/tests/couverture_reelle/phrases.json` (44 phrases), `ai/tests/couverture_reelle_reel.rs`, `docs/couverture-phrases-reelles.md`. Couverture : 33 sur 44 (75,0 %) ; modèle local installé : 21 sur 44 bien rendues (47,7 %). Tests du harnais verts selon l'agent (14 passés, 2 ignorés qui chargent le modèle). Sources : documents ou thèmes publics nommés, URL de page non vérifiées une à une. À reprendre en 10.8 : ajouter le fichier à `training/tests/recouvrement.rs` (garde anti-fuite) ; la notation « 5.90 E 45.90 N » (CV14, CV18, CV28, CV29) tombe sur le garde-fou de coordonnées ; CV26 passera au rouge quand 10.12 sera livré.
