---
title: 'Faire repondre un modele quantifie hors ligne sur le poste'
type: 'chore'
ticket: '1'
created: '2026-10-14'
status: 'built'
baseline_revision: '10a094b6d8b647ae20c22a76ca144bf6d70e3187'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** personne n'a encore vu un modèle quantifié répondre sur le poste du propriétaire, hors ligne. Rien de l'epic 8 ne peut être mesuré avant : c'est le traceur.

**Approach:** installer hors du dépôt, dans `C:\Users\kevin\cassini-modeles\`, la version compilée de llama.cpp pour Windows et CUDA 12.4, et un modèle GGUF quantifié ; puis, câble réseau débranché, poser une question en français en appelant llama.cpp en ligne de commande, sans mode serveur ni port ouvert, et constater qu'aucune connexion ne part. Ce dossier n'est jamais versé au dépôt.

</frozen-after-approval>

## Implementation Notes

Route oneshot : aucune ligne de code, des installations et une démonstration, dont la partie réseau coupé est faite par le propriétaire (hitl).

Vérifié le 2026-10-14 :
- Poste : RTX 3070 Ti, 8 Go de mémoire vidéo, pilote NVIDIA 616.92 (`nvidia-smi`) ; 32 Go de RAM (propriétaire).
- llama.cpp (MIT) : les binaires Windows sont publiés sur les versions de nuit `bNNNNN` du dépôt ggml-org/llama.cpp, les versions `v0.x` n'en portent pas. Dernière au relevé : b11624, `llama-b11624-bin-win-cuda-12.4-x64.zip` (265 Mo) et `cudart-llama-bin-win-cuda-12.4-x64.zip` (391 Mo). CUDA 12.4 choisi plutôt que 13.4, pris en charge par un pilote de cette génération sans hypothèse supplémentaire.
- Modèle : Qwen3.5-9B, licence Apache-2.0 (carte du modèle Qwen/Qwen3.5-9B, publiée le 2026-02-27), 201 langues, appel d'outils annoncé, mode « réflexion » actif par défaut. Fichier `Qwen3.5-9B-Q4_K_M.gguf`, 5,68 Go, dépôt unsloth/Qwen3.5-9B-GGUF (licence apache-2.0) : tient entier dans les 8 Go de la carte.
- Aucun port : hypothèse de départ « on appelle `llama-cli`, jamais `llama-server` », **démentie** plus bas — dans b11624, `llama-cli` embarque lui-même un serveur TCP local ; l'outil retenu est `llama-completion`.

Installation faite le 2026-10-14 dans `C:\Users\kevin\cassini-modeles\` : les deux archives testées intactes et conservées sous les noms `llama.zip` (SHA-256 `712096c39d31946410b152356753856b31164cf2653c3ab51b836b3f4e7f9e44`) et `cudart.zip` (SHA-256 `8c79a9b226de4b3cacfd1f83d24f962d0773be79f1e7b75c6af4ded7e32ae1d6`) ; l'empreinte SHA-256 du modèle égale celle publiée par Hugging Face, `03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8`, 5 680 522 464 octets. `llama-cli --version` : 0.6.0-dev, build 11624, commit af0579c78 ; `--list-devices` : CUDA0, RTX 3070 Ti, 7 109 Mio libres.

Essai réseau branché (avant l'épreuve hitl), `llama-cli -m ..\modeles\Qwen3.5-9B-Q4_K_M.gguf -c 8192 -ngl 99 --reasoning-budget 0 -n 200 -st -p "…"` : réponse juste et en bon français sur la zone tampon ; lecture de l'invite 340,6 jetons/s, génération 81,4 jetons/s, modèle entier en mémoire vidéo. `--reasoning-budget 0` coupe le mode réflexion de Qwen3.5, actif par défaut ; son effet sur la qualité des plans est à mesurer en 8.5.

**Surprise, épreuve du propriétaire avec `llama-cli` (2026-10-14).** Réponse juste et détaillée en français sur l'intersection de couches (génération 74,1 jetons/s). Mais `Get-NetTCPConnection` sur le processus montre un port **en écoute sur 127.0.0.1:50250** et une connexion établie de `llama-cli` vers lui-même : dans cette version, `llama-cli` embarque un serveur local et lui parle par TCP. Rien ne sort de la machine (toutes les adresses sont en boucle locale), mais c'est un port ouvert, que l'Intent exclut. Le verify échoue donc avec `llama-cli`.

**Correction.** `llama-completion.exe`, livré dans la même archive, en mode brut (`-no-cnv`), sans aucun port. Relevé définitif, réseau branché :

```
llama-completion.exe -m ..\modeles\Qwen3.5-9B-Q4_K_M.gguf -c 4096 -ngl 99 -n 1200 -no-cnv -f <invite>
```

où `<invite>` est un fichier au format de conversation ChatML du modèle (`<|im_start|>system … <|im_end|>`, puis `user`, puis `assistant` suivi d'un bloc `<think>` vide qui coupe la réflexion). Huit relevés à une seconde d'intervalle pendant la génération, par `Get-NetTCPConnection` et `Get-NetUDPEndpoint -OwningProcess` : **TCP = 0, UDP = 0** à chaque fois. Réponse juste, structurée et en bon français sur la zone tampon ; lecture de l'invite rapide, génération 82,8 jetons/s.

Deux pièges relevés pour 8.3. En `-no-cnv -p` sans gabarit, le modèle part en réflexion libre en anglais et `--reasoning-budget 0` n'a pas d'effet : l'invite doit porter le gabarit ChatML elle-même. En mode conversation (`-cnv -st --jinja`), le programme attend le clavier et s'arrête aussitôt quand l'entrée standard est fermée : inutilisable par un harnais. C'est l'outil retenu pour l'épreuve et pour le harnais de 8.3 ; `llama-cli` est écarté. Sans effet sur le produit, qui liera la bibliothèque (AD-15) et n'appellera aucun de ces programmes.

**Verdict du verify, décidé par le propriétaire le 2026-10-14.** Sa première épreuve s'est faite câble Ethernet débranché, sans autre accès au dehors (le poste n'a pas de Wi-Fi ; la carte virtuelle de WSL2 ne mène nulle part câble débranché) : le modèle a répondu hors ligne, et toutes les adresses relevées étaient en boucle locale. Le propriétaire juge la preuve suffisante et retient `llama-completion` sans refaire l'épreuve débranchée avec lui : l'absence de tout port, TCP et UDP, est établie par le relevé ci-dessus (réseau branché), l'absence de besoin de réseau par sa propre épreuve. Spike conclu : un modèle quantifié répond sur le poste, hors ligne, et l'outil sans port est identifié. Les épreuves débranchées de 8.4 et 8.5 se feront avec `llama-completion`.

## Verification

**Manual checks (if no CLI):**
- Câble Ethernet débranché, le modèle répond en français à une question posée par le propriétaire — fait avec `llama-cli`.
- Pendant la génération, `Get-NetTCPConnection` et `Get-NetUDPEndpoint -OwningProcess` ne montrent aucune connexion ni aucun port pour le processus `llama-completion` — fait, réseau branché ; le propriétaire a renoncé à refaire l'épreuve débranchée avec cet outil (voir le verdict).

## Review Triage Log

Passe rapide (`quick`), 8 constats : 5 réels corrigés (patch), 3 sans action.

- medium, patch — la section Verification nommait encore `llama-cli`, l'outil écarté. Réécrite pour `llama-completion`, avec ce qui a été fait réseau branché ou débranché.
- medium, patch — l'absence de port n'était prouvée qu'en TCP. Relevé refait en TCP et UDP, huit fois pendant la génération : 0 et 0.
- medium, patch — aucune trace d'une réponse de `llama-completion` ni de sa commande complète. Ajoutées, avec le gabarit ChatML et les deux pièges du mode brut et du mode conversation, utiles à 8.3.
- low, patch — « on appelle `llama-cli` » laissé comme prémisse alors qu'il est démenti ; marqué comme démenti.
- low, patch — « sans autre interface réseau » inexact sous WSL2 ; reformulé, la preuve tenant à l'adresse en boucle locale.
- low, patch — empreinte du modèle tronquée, archives sans empreinte ; empreintes complètes et noms sur disque ajoutés.
- false — « aucune exécution ne remplit les deux conditions du verify » : exact sur le fond, mais le propriétaire y a renoncé par écrit dans ce plan ; c'est sa décision, consignée au verdict.
- false — statut `in-progress` contre « spike conclu » : le statut passe à `built` à la finalisation, ce qui est l'étape suivante.
