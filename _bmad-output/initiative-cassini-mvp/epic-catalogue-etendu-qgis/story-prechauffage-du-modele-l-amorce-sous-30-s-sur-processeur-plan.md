---
title: "Préchauffage du modèle : l'amorce sous 30 s sur processeur"
type: 'feature'
ticket: '34'
created: '2026-11-21'
status: 'built'
baseline_revision: '7daf1ce60f983729ec2ff3a3fe228f77e0ac426d'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/mesure-duree-par-phrase.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La première phrase d'une session (l'amorce) paie le chargement du modèle et la lecture de la partie fixe de l'invite : 31,6 s sur 4 cœurs (dont 26,5 s de lecture, 1 572 jetons), au-dessus de la barre de 30 s du ticket 10.17. Les phrases suivantes reprennent un instantané de cette partie et coûtent environ 1 s de lecture.

**Approach:** Le noyau charge le modèle et lit la partie fixe de l'invite avant la première phrase, dans un fil à part, pour créer l'instantané. L'amorce ne paie alors plus que sa demande.

## Boundaries & Constraints

**Always:** Le préchauffage utilise les mêmes jetons que `plan.propose` (même `available`, même style d'invite, même modèle), sinon l'instantané n'est pas repris. Le canal JSON-RPC ne se bloque pas (AD-4). Une demande qui arrive pendant le préchauffage attend la fin du verrou du moteur, sans perte. Un échec de préchauffage (modèle absent ou illisible) est silencieux : la première vraie demande rend l'erreur habituelle. Aucun réseau, aucune vérification d'empreinte automatique (AD-17).

**Never:** Ne pas sortir l'amorce de la mesure. Ne pas changer l'invite, le catalogue ni les plans rendus (A0 à E identiques). Ne pas toucher à la grammaire ni au modèle.

## I/O & Edge-Case Matrix

| Scénario | État | Comportement attendu | Erreur |
|----------|------|---------------------|--------|
| Session normale | modèle présent, `available` connu | instantané de la tête créé avant la 1re phrase ; amorce < 30 s | — |
| Demande pendant le préchauffage | préchauffage en cours | la demande attend le verrou, puis reprend l'instantané | aucune perte |
| Modèle absent ou illisible | pas de modèle | rien ne se passe, pas de message | la 1re demande rend l'erreur habituelle |
| Préchauffage désactivé | variable `CASSINI_NO_WARMUP` posée | comportement actuel (amorce ≈ 31,6 s) | — |
| Autre ensemble de commandes | `available` différent | instantané non repris, aucun faux résultat | amorce lente, plans corrects |

**Décision du propriétaire (2026-11-21) :** le préchauffage se déclenche à l'ouverture de la session (`session.hello`), pas à l'ouverture du panneau ; `CASSINI_NO_WARMUP` le coupe. Aucun changement dans l'extension QGIS.

**Renégociation du périmètre (2026-11-21, après mesure) :** la partie fixe de l'invite ne fait que 258 jetons ; l'amorce passe à 27,8 s mais lit encore 24,6 s d'inventaire (1 314 jetons). Le propriétaire choisit de préchauffer aussi l'inventaire, par la conception **B** : le noyau, dans sa boucle principale, juste après avoir répondu à `session.hello`, lit `project.inventory` par le canal (comme `plan.propose`, sans nouveau message public), en construit la vue sans demande, puis la passe à un fil détaché qui lit l'invite complète (tête + inventaire) pour créer les instantanés. Un échec de lecture de l'inventaire (commande non annoncée, erreur, `busy`) retombe sur le préchauffage de la tête seule, sans message. Une demande reçue pendant la lecture de l'inventaire par la boucle reçoit la réponse « occupé » habituelle ; la lecture est courte et ne journalise rien. La barre de A0 (< 30 s) dépend de la vitesse d'écriture de la machine et n'est pas du ressort de ce ticket : elle est consignée, pas abaissée.

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs` -- `engine()` l.323 (un moteur, un `Mutex`), `generate_on` l.799-814 (les instantanés se prennent pendant la lecture), `Checkpoints::best` l.260 (reprise par préfixe) ; ajouter une fonction publique `warm` qui lit la tête et s'arrête (`max_new_tokens` minimal).
- `ai/src/plan.rs` -- `fixed_system_text` l.400, `head_for` l.1478, `compose_plan_anchored_cut` l.726 ; réutiliser, ne pas modifier l'invite.
- `cli/src/main.rs` -- `ModelComposer` l.255-301, `choose` ; ajouter l'implémentation de la méthode de préchauffage du compositeur.
- `core/src/serve.rs` -- trait `Composer` l.1941 (implémenté aussi par `FixedComposer` des tests), `session.hello` l.959 où `announced` est fixé : point où `available` est connu.
- `ai/tests/duree_reel.rs` -- l'amorce `WARMUP` l.82 passe par `compose_plan_anchored` ; garde l.272 (une amorce sans appel panique) ; appeler `warm` avant la boucle l.238.
- À ne pas toucher : invite, catalogue, grammaire, `verify_fingerprint` (lib.rs:198).

## Tasks & Acceptance

**Execution:**
- [ ] `ai/src/lib.rs` -- fonction `warm` : charge le modèle et lit la tête fixe, crée l'instantané `SLOT_HEAD`, n'écrit rien -- cœur du gain
- [ ] `core/src/serve.rs`, `cli/src/main.rs` -- méthode `warm` du `Composer` (défaut : rien), appelée dans un fil à la fin de `session.hello`, avec les commandes annoncées ; saute si `CASSINI_NO_WARMUP` -- déclenchement
- [ ] `ai/src/lib.rs`, `core/src/serve.rs` -- tests : `warm` ne produit aucun jeton écrit, un échec ne remonte pas, une demande concurrente attend sans erreur -- matrice
- [ ] `ai/tests/duree_reel.rs` -- appeler `warm` avant l'amorce, garde adaptée -- mesure
- [ ] `core/src/serve.rs`, `core/src/serve/plans.rs`, `cli/src/main.rs`, `ai/src/plan.rs` -- après la réponse à `session.hello`, lecture de `project.inventory` dans la boucle principale, vue sans demande, crochet `Composer::warm_inventory` qui rend la tâche détachée lisant tête + inventaire ; repli sur la tête seule -- conception B
- [ ] `docs/mesure-duree-par-phrase.md` -- relevé mesuré après préchauffage (tableau de `duree_reel`), jamais écrit à la main -- preuve

**Acceptance Criteria:**
- Given `base-17-1`, 4 cœurs, when `duree_reel` passe avec le préchauffage de l'inventaire, then la lecture du prompt de l'amorce est inférieure à 5 s et son total reste sous 30 s.
- Given `base-17-1`, 4 cœurs (`0x55`), même inventaire, when `duree_reel` passe avec préchauffage, then l'amorce est strictement sous 30 s ; A0 est consignée telle que mesurée (son écriture dépend de la machine).
- Given les mêmes phrases, when le test passe, then les plans de A0 à E sont identiques à ceux du relevé du 2026-11-21.
- Given un modèle absent, when la session s'ouvre, then aucune erreur n'est émise avant la première demande.
- Given `cargo test --workspace`, clippy et fmt, then verts.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (lentille quick) : 0 high, 3 medium, 1 low, 4 false.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | defer | Lecture synchrone de `project.inventory` dans la boucle, `choose(None)` fait deux fois : prévu par la renégociation (conception B) ; même attente que `plan.propose`, sans délai de garde dans `Channel::execute` (code antérieur). |
| medium | defer | Vue d'inventaire préchauffée sans demande : pour un gros projet à vue triée, l'instantané n'est pas repris, et un projet modifié entre l'ouverture et la première phrase reprend le cache jusqu'à la couche changée ; vitesse seule, jamais la justesse. Non mesuré sur gros projet. |
| medium | defer | `CASSINI_NO_WARMUP` et le verrou concurrent du moteur sans test automatique : variable d'environnement partagée entre tests, moteur réel nécessaire ; couverts par la mesure `duree_reel` et le test de `session.busy`. |
| low | rejeté | `warm_files` génère un jeton jeté : le plan dit « n'écrit rien », le jeton n'est rendu à personne ; la reprise de 1554 jetons sur l'amorce, mesurée deux fois, prouve que le préfixe est identique. |
| false | rejeté | Attendu de `raster.reproject_work` dans `a_dictated_command_leaves_the_prompt_and_the_grammar_unchanged` : test resté rouge depuis 10.33 (commande dictée ajoutée sans mise à jour de la liste), corrigé ; aucune commande du catalogue touchée. |
| false | rejeté | Extension QGIS non prête pour un `order.execute` hors `plan.propose` : `client.py` `_handle_call` l'accepte à tout moment, refuse `busy` seulement si un ordre est en cours ; aucun changement d'extension. |
| false | rejeté | A0 > 30 s contredit « Verification » : le renégocié consigne A0 tel que mesuré (écriture seule, machine plus lente ce soir) ; corriger le plan n'est pas un constat de code. |
| false | rejeté | `warm_due` posé avant la fin de `hello` : rien d'intermédiaire ne peut échouer, la boucle s'arrête sur l'écriture ratée. |

## Design Notes

Le coût du préchauffage tombe au lancement de la session : environ 26 s de calcul sur 4 cœurs et 2,6 Go de mémoire, avant que l'utilisateur ait tapé. C'est le seul prix ; le propriétaire l'accepte à l'ouverture de la session.

## Verification

**Commands:**
- `cargo test -p cassini-ai -p cassini-core -p cassini` -- expected: vert
- `C:\Users\kevin\duree.ps1` (build `C:\ct`, 4 cœurs, `0x55`) -- expected: amorce < 30 s, A0 < 30 s, plans A0 à E inchangés
- `cargo fmt --check` et `cargo clippy` -- expected: sans avertissement
