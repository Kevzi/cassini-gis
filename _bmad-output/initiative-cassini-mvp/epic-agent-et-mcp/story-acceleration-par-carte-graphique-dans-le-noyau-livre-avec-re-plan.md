---
title: 'Accélération par carte graphique dans le noyau livré, avec repli sur le processeur'
type: 'feature'
ticket: '21'
created: '2026-10-30'
status: 'built'
baseline_revision: 'b406ee5834ab5d889267a9d57f7f410b72446388'
route: 'full'
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

**Problem:** Le noyau livré est construit sans carte graphique : sur un poste qui en a une, la phrase est composée à la vitesse du processeur (≈ 6 jetons/s en écriture). L'option `cuda` de `cassini-ai` ne sert qu'aux mesures et n'est jamais livrée.

**Approach:** Livrer un noyau dont le moteur charge ses « moteurs de calcul » au démarrage, comme des bibliothèques à part posées à côté de `cassini.exe` : un moteur processeur (variantes par jeu d'instructions) toujours présent, un moteur carte graphique pris s'il se charge et trouve une carte, ignoré sinon. Repli sur le processeur aussi quand le chargement du modèle sur la carte échoue. Aucun réglage de l'utilisateur.

**Décisions (2026-10-30, propriétaire) :**
- **Vulkan seul** : un paquet pour NVIDIA, AMD et Intel ; CUDA n'est pas livré (bibliothèques propriétaires NVIDIA, contraires à §8) et reste mesuré pour comparaison ; si Vulkan est plus de deux fois plus lent que CUDA sur la 3070 Ti, s'arrêter et présenter les chiffres.
- **SDK Vulkan** installé par le propriétaire (`winget install KhronosGroup.VulkanSDK`) sur le poste de construction seulement, jamais livré.
- **Cartes dédiées seules** : une carte intégrée n'est jamais utilisée ; sans carte dédiée, processeur.

## Boundaries & Constraints

**Always:** la carte est utilisée dans le processus du noyau, sans serveur ni réseau ; aucune bibliothèque livrée n'importe une DLL réseau (test `hors_ligne` étendu à toutes les DLL du dossier) ; licences liées au noyau MIT, Apache-2.0 ou BSD ; `cargo deny` vert ; la recette par défaut (CI Linux et Windows) se construit sans SDK de carte graphique ; mesures CPU sur le 5800X bridé (`0x55`, 4 fils), carte masquée.

**Never:** téléchargement de quoi que ce soit à l'exécution ; réglage demandé à l'utilisateur ; changer le prompt, la grammaire, le budget d'inventaire ou le modèle ; abaisser le critère de 30 s sur processeur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Carte présente | pilote à jour, moteur carte chargé | modèle entier sur la carte | — |
| Aucune carte, ou pilote sans Vulkan | `vulkan-1.dll` absente ou aucun périphérique | moteur carte ignoré, processeur | aucune erreur visible |
| Carte trop petite | chargement du modèle sur la carte échoue | nouveau chargement sur processeur, une fois | erreur seulement si le processeur échoue aussi |
| Carte intégrée seule | aucune carte dédiée, une carte intégrée vue par Vulkan | processeur, la carte intégrée n'est pas prise | — |
| Moteur carte retiré du dossier | `ggml-vulkan.dll` absente | processeur | — |
| Processeur ancien | sans AVX2 | variante processeur adaptée choisie seule | — |

</frozen-after-approval>

## Code Map

- `ai/Cargo.toml` -- `llama-cpp-2 =0.1.159` (sys 0.1.160) a les features `vulkan` et `dynamic-backends` (= `dynamic-link` + `GGML_BACKEND_DL` + `GGML_CPU_ALL_VARIANTS`) ; garder `cuda` pour les mesures.
- `ai/src/lib.rs:266` `engine()` -- après `LlamaBackend::init`, charger les moteurs depuis le dossier de l'exécutable (`load_backends_from_path`), jamais le dossier compilé du poste de construction. `:491` `model_params()` -- défaut llama.cpp `n_gpu_layers = -1` (tout sur la carte) mais il prend une carte intégrée faute de carte dédiée (`llama.cpp/src/llama.cpp:280`) : passer à `n_gpu_layers = 0` quand aucun périphérique de type `GPU` (dédié) n'est listé (`list_llama_ggml_backend_devices`, type `IGPU` exclu), et vérifier que rien ne part sur la carte intégrée (cf. `docs/mesure-duree-par-phrase.md:145` : même à `ngl 0`, un moteur carte chargé peut recevoir les gros lots ; si c'est le cas, ne pas charger le moteur carte du tout). Repli processeur sur échec de chargement.
- `llama-cpp-sys-2/build.rs:1222` -- les moteurs vont dans `OUT_DIR/backends`, exposé comme `DEP_LLAMA_BACKENDS_DIR` aux dépendants directs ; les DLL partagées (`llama.dll`, `ggml*.dll`) sont déjà liées dans `target/<profil>` et `deps/`. Un `ai/build.rs` copie les moteurs au même endroit.
- `cli/tests/hors_ligne.rs` -- étendre aux DLL voisines de l'exécutable.
- `qgis-plugin/cassini_qgis/locator.py:12` -- le paquet est `%LOCALAPPDATA%\Cassini\bin` ; rien à changer si les DLL y sont posées à côté de `cassini.exe`.
- `.github/workflows/recette.yml` -- CI sans SDK Vulkan : la feature reste hors défaut.
- Poste : RTX 3070 Ti 8 Go, pilote 616.92, `vulkan-1.dll` présente, CUDA 12.4 installé ; SDK Vulkan installé (`C:\VulkanSDK\1.4.363.0`, `VULKAN_SDK` posé au niveau machine, à passer aux commandes PowerShell ; requis par `build.rs` pour compiler les shaders).
- Masquer la carte : `GGML_VK_VISIBLE_DEVICES` (ggml-vulkan), ou retirer `ggml-vulkan.dll` d'une copie du paquet.

## Tasks & Acceptance

**Execution:**
- [ ] `ai/Cargo.toml`, `cli/Cargo.toml` -- feature `gpu` (`vulkan` + `dynamic-backends`) dans `cassini-ai`, relayée par `cassini` ; le paquet se construit avec elle.
- [ ] `ai/build.rs` -- copier les moteurs à côté des binaires du profil.
- [ ] `ai/src/lib.rs` -- chargement des moteurs depuis le dossier de l'exécutable ; repli processeur ; test sans modèle de la décision de repli.
- [ ] `cli/tests/hors_ligne.rs` -- toutes les DLL du dossier.
- [ ] mesure (sonde hors dépôt) -- Vulkan contre CUDA sur la 3070 Ti, phrase de démo, durées et taille du paquet ; si Vulkan est plus de deux fois plus lent que CUDA, s'arrêter et présenter les chiffres (HITL).
- [ ] `docs/mesure-duree-par-phrase.md` -- durées carte et processeur, taille du paquet, scores ; `docs/` : construction du paquet (SDK Vulkan, commande, fichiers à copier).

**Acceptance Criteria:**
- Given la 3070 Ti, when la phrase de démo est composée par le paquet livré, then elle l'est bien plus vite que sur processeur, durées consignées.
- Given le même paquet, carte masquée puis moteur carte retiré, when la recette enchaînée tourne sur le 5800X bridé, then elle compose sans erreur et pas plus lentement qu'en 5.19.
- Given le paquet livré sur la carte, when `plan_reel`, `question_reel`, `ecart_reel` tournent, then 25/25, 12/12, 32/40 au moins.
- Given la recette, when `cargo deny`, `cargo test`, `clippy`, `fmt` tournent, then verts, CI comprise.

## Design Notes

Pourquoi des moteurs chargés à part : lié directement à `vulkan-1.dll`, `cassini.exe` refuserait de démarrer sur un poste sans pilote Vulkan. Chargés à part, un moteur qui ne se charge pas est simplement ignoré par ggml : c'est le repli. Contrepartie : le paquet n'est plus un seul fichier, mais un dossier (`cassini.exe` + quelques DLL).

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` (PowerShell, avec et sans `--features gpu`) -- expected: verts.

**Manual checks (if no CLI):**
- Hors critères (risque high) : le propriétaire installe le paquet et lance une phrase dans QGIS, avec puis sans carte (`GGML_VK_VISIBLE_DEVICES` vide).


## Review Triage Log

Relecture `thorough` (quatre lentilles), itération 0.

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|---|---|---|---|---|
| 1 | verification-gap, blind-hunter | Le rechargement sur processeur après échec du contexte sur la carte n'a aucun test | medium | patch | Aucun test n'atteint le bras `AiError::Context` ni `force_cpu` ; supprimer le bras laisse tout vert. |
| 2 | blind-hunter | `hors_ligne` ne vérifie l'absence d'import `vulkan-1` que sur l'exe, ni l'absence de `llama-common.dll` | low | patch | `llama.dll`, `ggml*.dll` sont importées au démarrage ; la règle de la doc n'est pas affirmée. Correction directe de quelques lignes. |
| 3 | verification-gap | `processor_parameters_offload_nothing` ne voit pas la suppression de `with_devices(&[])` | medium | patch (commentaire) + defer | `devices()` est vide aussi pour les paramètres par défaut (tableau nul dans la crate) ; la vraie preuve demande un poste à carte intégrée. |
| 4 | blind-hunter | Doc de mesure : « 13,6 s contre 26,9 s » absent du tableau | low | patch | Le tableau donne 13,9/27,9 (A0) et 6,9/13,1 (écriture moyenne). |
| 5 | verification-gap, blind-hunter, intent-alignment | Le chemin `gpu` (build.rs, load_backends, bloc gpu de `hors_ligne`) n'est jamais compilé en CI | medium | defer | La CI n'a pas le SDK Vulkan (contrainte « Always » du plan) ; seule la commande manuelle de `paquet-du-noyau.md` le vérifie. |
| 6 | edge-case-hunter, blind-hunter, intent-alignment | Échec sur la carte au décodage ou à l'adaptateur : pas de repli | maybe-false | defer | llama.cpp alloue cache et tampons à la création du contexte ; un manque de mémoire plus tard n'est pas démontré. Si réel : medium. À trancher sur une carte de 4 Go. |
| 7 | edge-case-hunter | `llama-common.dll` posée par le build.rs de llama-cpp-sys-2 malgré le filtre | false | rejet | `C:\ct\release` ne contient pas `llama-common.dll` après la construction `gpu`, et `hors_ligne` (toutes les DLL) est vert. |
| 8 | blind-hunter | Épinglages `=0.1.159` / `=0.1.160` incohérents | false | rejet | `Cargo.lock` n'a qu'un `llama-cpp-sys-2` (0.1.160), celui que tire llama-cpp-2 0.1.159. |
| 9 | blind-hunter, edge-case-hunter | Sous `cuda`, le repli recharge avec les mêmes paramètres | low | rejet | `cuda` sert aux mesures seules, jamais livré ; un échec de chargement y est improbable et le corriger ajoute une branche. |
| 10 | blind-hunter, edge-case-hunter, verification-gap | `with_devices(gpus)` en échec → `cpu()` mais `on_gpu = true` | low | rejet | Les index viennent de `Placement::current()` ; l'échec n'est pas atteignable en usage, la correction ajoute une surface d'erreur. |
| 11 | blind-hunter | Aucun signal de l'endroit où tourne le modèle | low | rejet | `CASSINI_DEBUG_LLAMA=1` le donne ; demande de fonction, pas un défaut. |
| 12 | edge-case-hunter | `GGML_BACKEND_PATH` peut charger une DLL de plus | low | rejet | Variable posée par l'utilisateur sur son propre poste ; la retirer (`remove_var`) n'est pas sûre dans un processus à plusieurs fils. Noté dans le rapport du build. |
| 13 | edge-case-hunter | `engine()` garde l'erreur de `load_backends` pour toute la vie du processus | low | rejet | Moteurs absents = paquet mal installé ; relancer QGIS après réparation suffit, comme pour `LlamaBackend::init` avant 5.21. |
| 14 | edge-case-hunter, blind-hunter | build.rs : `ancestors().nth(3)`, dossier absent, copie en échec, déclencheurs de relance | low | rejet | `build-dir` séparé n'est pas utilisé ; `target/<profil>` existe toujours ; une DLL verrouillée par un `cassini.exe` lancé bloque aussi l'édition de liens de l'exe. |
| 15 | edge-case-hunter, blind-hunter | `.so` versionnées sous Linux non copiées | low | rejet | Le paquet `gpu` n'est construit que sous Windows dans ce ticket. |
| 16 | edge-case-hunter, blind-hunter | `PeFile64::parse` panique sur une DLL 32 bits ; moteurs restés d'un build `gpu` | low | rejet | Le dossier de construction ne contient que des DLL 64 bits produites par cargo ; l'exe processeur ne charge pas de moteur. |
| 17 | blind-hunter | DLL chargées depuis un dossier modifiable par l'utilisateur | false | rejet | Le même dossier contient déjà `cassini.exe` ; qui peut y écrire peut remplacer l'exe : aucun risque nouveau. |
| 18 | blind-hunter | Doc du paquet : 18 s au premier usage de Vulkan, nombre de `ggml-cpu-*` | low | rejet | La phrase 1 est hors critère des 30 s ; le nombre (neuf) est déjà dans `mesure-duree-par-phrase.md`. |
| 19 | intent-alignment | « Livré » n'est atteint que par une procédure manuelle, sans chaîne de publication | false | rejet | La publication (installeurs) est l'étape 7 ; l'intention demande que le paquet sache utiliser la carte, ce que la construction `gpu` fait. |
