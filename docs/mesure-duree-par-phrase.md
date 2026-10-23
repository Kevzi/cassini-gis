# Mesure de la durée par phrase sur processeur (ticket 11)

Point de départ chiffré de l'exigence « une longue phrase résolue en moins de 30 s sur un
poste sans carte graphique ». Mesure du **binaire réel** : chaque phrase passe par
`compose_plan`, comme dans le produit (modèle rechargé à chaque appel, redemande comprise,
aucun cache). Relevé du 2026-10-17 sur un Ryzen 7 5800X (8 cœurs, 16 fils), Windows,
modèle `Qwen3.5-9B-Q4_K_M.gguf`, llama.cpp compilé **sans CUDA** (AVX2, `--release`).

C'est une mesure de référence : aucune durée n'est vérifiée par un test. La justesse des
plans reste portée par `plan_reel`, sur GPU.

## Mode opératoire

L'outil est l'essai `ai/tests/duree_reel.rs`, ignoré par défaut. Variables :

- `CASSINI_TEST_MODEL` : chemin du modèle (sans elle, l'essai ne fait rien) ;
- `CASSINI_TEST_THREADS` : nombre de fils du moteur, lecture et écriture (sans elle : défaut de llama.cpp) ;
- `CASSINI_TEST_INVENTORY` : un inventaire JSON ; l'essai passe alors les phrases A à E du
  scénario de démo au lieu des 25 phrases de l'épreuve.

Dans PowerShell (voir `modele-local-sous-windows.md` pour `LIBCLANG_PATH`) :

```powershell
$env:CARGO_TARGET_DIR = "C:\ct"
cargo test --release -p cassini-ai --test duree_reel --no-run   # jamais --features cuda
$exe = Get-ChildItem C:\ct\release\deps\duree_reel-*.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$env:CASSINI_TEST_MODEL = "$PWD\ai\models\Qwen3.5-9B-Q4_K_M.gguf"
$env:CASSINI_TEST_THREADS = "8"
$p = Start-Process $exe.FullName -ArgumentList "--ignored","--nocapture","duree_reel" `
       -NoNewWindow -PassThru -RedirectStandardOutput mesure-8.txt
$p.ProcessorAffinity = [IntPtr]0x5555   # 8 cœurs physiques ; 0x55 = 4 cœurs physiques
$p.WaitForExit()
```

Sur le 5800X, les deux fils d'un même cœur portent des numéros voisins (0-1, 2-3…) : le
masque `0x5555` donne un fil par cœur sur les 8 cœurs, `0x55` un fil par cœur sur 4 cœurs.
Le tableau s'imprime en Markdown, suivi d'une ligne de moyennes. Une passe des 25 phrases
dure 48 min à 8 fils, 73 min à 4 fils.

Colonnes : durées en secondes, temps réel. **Chargement** = lecture du fichier du modèle,
création du contexte et découpage de l'invite ; **lecture du prompt** = décodage de
l'invite d'un bloc ; **écriture** = réponse jeton par jeton, grammaire comprise. Une phrase
qui demande une redemande additionne ses deux appels (colonne `appels`). Un appel qui
échoue (par exemple `PromptTooLong` à la redemande, que `compose_plan` absorbe alors que le
modèle est déjà chargé) n'est ni compté ni chronométré. Le chargement compte aussi le
démarrage du moteur llama.cpp (une fois, à la première phrase) et l'attente de son verrou,
et la libération du modèle en fin d'appel n'entre dans aucune colonne : le total est donc un
peu inférieur à la durée complète de l'appel.

## 8 fils, 8 cœurs physiques (`0x5555`)

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,1 | 91,7 | 5127 | 5,3 | 23 | 101,1 | plan, 1 étape(s) |
| S02 | 1 | 3,9 | 93,1 | 5129 | 13,7 | 57 | 110,7 | plan, 1 étape(s) |
| S03 | 1 | 4,0 | 93,9 | 5131 | 3,6 | 17 | 101,5 | plan, 1 étape(s) |
| S04 | 1 | 3,9 | 94,7 | 5129 | 7,5 | 24 | 106,1 | refus |
| S05 | 1 | 4,2 | 100,5 | 5130 | 9,5 | 29 | 114,2 | refus |
| S06 | 1 | 4,1 | 106,7 | 5131 | 20,2 | 72 | 131,0 | plan, 1 étape(s) |
| S07 | 1 | 4,1 | 105,9 | 5137 | 17,1 | 69 | 127,2 | plan, 1 étape(s) |
| S08 | 1 | 4,1 | 99,8 | 5141 | 17,7 | 73 | 121,7 | plan, 1 étape(s) |
| S09 | 1 | 4,0 | 97,4 | 5129 | 7,2 | 28 | 108,6 | refus |
| S10 | 1 | 4,0 | 96,0 | 5128 | 8,2 | 33 | 108,2 | refus |
| S11 | 1 | 3,9 | 103,3 | 5133 | 18,3 | 74 | 125,5 | plan, 1 étape(s) |
| S12 | 1 | 3,8 | 94,6 | 5127 | 17,8 | 74 | 116,2 | plan, 1 étape(s) |
| S13 | 1 | 4,0 | 92,2 | 5133 | 25,7 | 104 | 121,9 | plan, 1 étape(s) |
| S14 | 1 | 4,0 | 90,4 | 5152 | 24,7 | 105 | 119,1 | plan, 2 étape(s) |
| S15 | 1 | 4,0 | 90,3 | 5133 | 12,4 | 52 | 106,7 | refus |
| S16 | 1 | 4,0 | 95,7 | 5169 | 23,7 | 102 | 123,4 | plan, 1 étape(s) |
| S17 | 1 | 3,9 | 95,1 | 5159 | 58,8 | 247 | 157,7 | plan, 5 étape(s) |
| S18 | 1 | 3,9 | 92,8 | 5143 | 10,2 | 43 | 107,0 | refus |
| S19 | 1 | 3,9 | 94,0 | 5138 | 40,4 | 164 | 138,2 | plan, 3 étape(s) |
| S20 | 1 | 3,9 | 94,3 | 5172 | 6,4 | 27 | 104,6 | question |
| S21 | 1 | 4,0 | 93,0 | 5179 | 10,0 | 42 | 106,9 | question |
| S22 | 1 | 4,0 | 89,3 | 5130 | 5,5 | 24 | 98,8 | refus |
| S23 | 1 | 3,9 | 89,5 | 5132 | 8,7 | 36 | 102,0 | refus |
| S24 | 1 | 3,9 | 90,5 | 5175 | 10,0 | 42 | 104,5 | refus |
| S25 | 1 | 3,9 | 92,2 | 5164 | 9,2 | 35 | 105,3 | refus |

Moyennes : chargement 4,0 s, lecture du prompt 95,1 s, écriture 15,7 s, **total 114,7 s**.

## 4 fils, 4 cœurs physiques (`0x55`)

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,0 | 159,8 | 5127 | 5,1 | 23 | 168,9 | plan, 1 étape(s) |
| S02 | 1 | 4,4 | 156,1 | 5129 | 13,3 | 57 | 173,8 | plan, 1 étape(s) |
| S03 | 1 | 4,0 | 155,8 | 5131 | 3,5 | 17 | 163,3 | plan, 1 étape(s) |
| S04 | 1 | 4,0 | 155,2 | 5129 | 5,5 | 24 | 164,7 | refus |
| S05 | 1 | 4,0 | 155,3 | 5130 | 6,8 | 29 | 166,1 | refus |
| S06 | 1 | 3,9 | 154,6 | 5131 | 17,1 | 72 | 175,5 | plan, 1 étape(s) |
| S07 | 1 | 4,0 | 155,1 | 5137 | 16,6 | 69 | 175,7 | plan, 1 étape(s) |
| S08 | 1 | 4,0 | 155,6 | 5141 | 17,1 | 73 | 176,7 | plan, 1 étape(s) |
| S09 | 1 | 3,9 | 154,6 | 5129 | 6,4 | 28 | 165,0 | refus |
| S10 | 1 | 3,9 | 154,2 | 5128 | 7,6 | 33 | 165,7 | refus |
| S11 | 1 | 3,9 | 154,5 | 5133 | 17,2 | 74 | 175,6 | plan, 1 étape(s) |
| S12 | 1 | 4,0 | 154,2 | 5127 | 17,3 | 74 | 175,5 | plan, 1 étape(s) |
| S13 | 1 | 3,9 | 154,6 | 5133 | 24,6 | 104 | 183,1 | plan, 1 étape(s) |
| S14 | 1 | 3,9 | 155,1 | 5152 | 24,5 | 105 | 183,5 | plan, 2 étape(s) |
| S15 | 1 | 3,9 | 154,5 | 5133 | 9,4 | 40 | 167,8 | refus |
| S16 | 1 | 3,8 | 155,8 | 5169 | 23,6 | 102 | 183,2 | plan, 1 étape(s) |
| S17 | 1 | 4,0 | 155,8 | 5159 | 58,0 | 247 | 217,8 | plan, 5 étape(s) |
| S18 | 1 | 4,0 | 155,0 | 5143 | 6,7 | 29 | 165,7 | refus |
| S19 | 1 | 3,9 | 154,7 | 5138 | 38,8 | 164 | 197,4 | plan, 3 étape(s) |
| S20 | 1 | 4,0 | 155,8 | 5172 | 6,3 | 27 | 166,0 | question |
| S21 | 1 | 4,0 | 155,9 | 5179 | 8,6 | 37 | 168,5 | refus |
| S22 | 1 | 4,0 | 156,6 | 5130 | 5,5 | 24 | 166,0 | refus |
| S23 | 1 | 4,0 | 153,8 | 5132 | 6,5 | 29 | 164,3 | refus |
| S24 | 1 | 3,9 | 155,5 | 5175 | 9,7 | 42 | 169,1 | refus |
| S25 | 1 | 4,0 | 155,0 | 5164 | 8,2 | 35 | 167,2 | refus |

Moyennes : chargement 4,0 s, lecture du prompt 155,3 s, écriture 14,6 s, **total 173,8 s**.

## Lecture

- **On est loin des 30 s** : environ 4 fois au-dessus à 8 fils (115 s), 6 fois à 4 fils
  (174 s). Aucune phrase ne passe sous 98 s.
- **La lecture du prompt fait l'essentiel** : 83 % du total à 8 fils, 89 % à 4 fils. Elle
  est presque la même pour toutes les phrases (5 130 à 5 180 jetons, dont la phrase ne
  représente que quelques dizaines) : c'est le catalogue des commandes et l'inventaire qui
  coûtent, relus à chaque appel. À elle seule, elle dépasse trois fois le budget de 30 s.
  Débit : environ 54 jetons/s à 8 fils, 33 jetons/s à 4 fils.
- **L'écriture ne dépend presque pas du nombre de fils** (15,7 s contre 14,6 s, environ
  4 jetons/s dans les deux cas) : elle est bornée par la mémoire, pas par le calcul. Elle
  croît avec la longueur du plan : 58 s pour les 247 jetons de S17 (cinq étapes).
- **Le chargement est stable**, 4 s par appel, payé à chaque phrase puisque le modèle
  n'est pas gardé.
- **Aucune redemande** dans ces passes : 25 appels pour 25 phrases à chaque réglage.
- Les issues ne sont pas tout à fait identiques d'un réglage à l'autre (S21 : question à
  8 fils, refus à 4 fils) : changer le nombre de fils change l'ordre des sommes en virgule
  flottante. Ce n'est pas une mesure de justesse.

**Constat sur les fils par défaut.** Le produit ne règle pas le nombre de fils : llama.cpp
prend alors `GGML_DEFAULT_N_THREADS`, qui vaut **4**, quelle que soit la machine. Le
produit tourne donc aujourd'hui à peu près comme la colonne « 4 fils », sans l'épinglage
sur 4 cœurs physiques : passer au nombre de cœurs physiques gagnerait, estimation, environ
un tiers sur un poste qui en a 8. Ce réglage n'est pas changé
ici ; il est reporté au ticket 5.12, avec le chargement unique du modèle et la mise en
cache de la partie fixe de l'invite, qui visent directement les deux postes les plus
lourds.

**Comparaison avec la mesure 8.6** (`tests/epreuve-modele-local/resultats-8-6-cpu.json`,
8 fils, `llama-completion.exe`) : chargement 1,9 s, prompt 6,0 s pour 4 873 jetons,
écriture 18,6 s pour 105 jetons. Ce n'était **pas une mesure CPU pure** : ce llama.cpp
était compilé avec CUDA et, même sans couche déportée (`ngl 0`), il confie au GPU la
lecture des grands lots (pic de mémoire vidéo de 1,5 Go dans le relevé). Son chiffre de
lecture du prompt n'est donc pas comparable à celui-ci. Seule son écriture, faite sur le
processeur, l'est à peu près (5,6 jetons/s contre environ 4 ici, avec la grammaire en plus).

**Limite du substitut.** Brider à 4 fils sur 4 cœurs physiques d'un 5800X ne reproduit ni
la mémoire ni la fréquence d'un portable : c'est un substitut, sans doute optimiste pour
l'écriture, qui dépend surtout de la bande passante mémoire.

Le relevé des phrases A à E du scénario Villeneuve-le-Roi, sur l'inventaire du projet de
test (hors dépôt), est noté dans le plan du ticket 11.

## Après le cache (ticket 12)

Depuis le ticket 12, le modèle est **chargé une fois** et gardé dans le processus du noyau
(il n'est rechargé que si le fichier change : chemin, taille ou date), et l'état du moteur
est gardé en mémoire à deux points de l'invite : après la **partie fixe** (consignes,
catalogue, exemples — l'inventaire vient désormais après eux) et après le **tour système**
(partie fixe + inventaire). Une demande reprend le plus profond de ces instantanés dont
les jetons sont un début des siens et ne lit que la suite. Rien n'est écrit sur disque, et
rien d'une demande n'est gardé pour la suivante. Le nombre de fils par défaut suit
désormais le nombre de cœurs physiques de la machine (crate `num_cpus`).

Relevé du 2026-10-18, même poste, même modèle, même binaire sans CUDA, 4 fils sur 4 cœurs
physiques (`0x55`), les 25 phrases dans **un même processus**. La colonne « jetons
repris » compte les jetons de l'invite repris d'un instantané, donc non relus ; la
« lecture du prompt » comprend la reprise de l'instantané et la prise des nouveaux.

| id | appels | chargement (s) | lecture du prompt (s) | jetons de l'invite | jetons repris | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,1 | 163,7 | 5128 | 0 | 5,6 | 23 | 173,4 | plan, 1 étape(s) |
| S02 | 1 | 0,1 | 1,3 | 5130 | 5109 | 14,2 | 57 | 15,5 | plan, 1 étape(s) |
| S03 | 1 | 0,1 | 1,3 | 5132 | 5109 | 3,8 | 17 | 5,1 | plan, 1 étape(s) |
| S04 | 1 | 0,1 | 1,1 | 5130 | 5109 | 5,9 | 24 | 7,1 | refus |
| S05 | 1 | 0,0 | 1,2 | 5131 | 5109 | 15,1 | 61 | 16,3 | refus |
| S06 | 1 | 0,1 | 1,2 | 5132 | 5109 | 17,4 | 72 | 18,7 | plan, 1 étape(s) |
| S07 | 1 | 0,1 | 1,3 | 5138 | 5109 | 16,8 | 69 | 18,2 | plan, 1 étape(s) |
| S08 | 1 | 0,1 | 1,5 | 5142 | 5109 | 17,9 | 73 | 19,5 | plan, 1 étape(s) |
| S09 | 1 | 0,1 | 1,1 | 5130 | 5109 | 7,5 | 31 | 8,6 | refus |
| S10 | 1 | 0,1 | 1,0 | 5129 | 5109 | 5,1 | 23 | 6,2 | plan, 1 étape(s) |
| S11 | 1 | 0,1 | 1,2 | 5134 | 5109 | 17,9 | 74 | 19,2 | plan, 1 étape(s) |
| S12 | 1 | 0,1 | 1,0 | 5128 | 5109 | 18,0 | 74 | 19,1 | plan, 1 étape(s) |
| S13 | 1 | 0,1 | 1,2 | 5134 | 5109 | 25,9 | 104 | 27,2 | plan, 1 étape(s) |
| S14 | 1 | 0,1 | 1,9 | 5153 | 5109 | 25,6 | 105 | 27,6 | plan, 2 étape(s) |
| S15 | 1 | 0,1 | 1,2 | 5134 | 5109 | 10,9 | 45 | 12,2 | refus |
| S16 | 1 | 0,1 | 2,5 | 5170 | 5109 | 23,8 | 102 | 26,4 | plan, 1 étape(s) |
| S17 | 1 | 0,1 | 2,2 | 5160 | 5109 | 59,1 | 247 | 61,4 | plan, 5 étape(s) |
| S18 | 1 | 0,1 | 1,6 | 5144 | 5109 | 6,8 | 29 | 8,5 | refus |
| S19 | 1 | 0,1 | 1,4 | 5139 | 5109 | 39,4 | 164 | 40,8 | plan, 3 étape(s) |
| S20 | 1 | 0,1 | 2,1 | 5173 | 5109 | 7,0 | 30 | 9,2 | refus |
| S21 | 1 | 0,1 | 2,6 | 5180 | 5109 | 8,3 | 35 | 11,0 | refus |
| S22 | 1 | 0,1 | 1,1 | 5131 | 5109 | 5,8 | 25 | 7,0 | refus |
| S23 | 1 | 0,1 | 1,2 | 5133 | 5109 | 8,6 | 36 | 9,8 | refus |
| S24 | 1 | 0,1 | 2,5 | 5176 | 5109 | 9,6 | 40 | 12,1 | refus |
| S25 | 1 | 0,1 | 2,4 | 5165 | 5109 | 8,6 | 34 | 11,0 | refus |

**Premier appel, à part.** S01 paie seul le chargement (4,1 s) et la lecture complète de
l'invite (163,7 s, contre 159,8 s au ticket 11 : la lecture est découpée en trois segments
et deux instantanés d'environ 150 Mo sont pris au passage). Ce coût n'est payé qu'une fois
par session dans un processus qui dure, `cassini serve` (le canal de l'extension). La
commande ponctuelle `cassini plan` démarre un processus par demande : elle paie le
chargement et la lecture complète à chaque appel, et ne gagne que sur sa redemande, qui
reprend l'instantané du tour système.

**Appels suivants (S02 à S25).** Moyennes : chargement 0,1 s, lecture du prompt 1,5 s,
écriture 15,8 s, **total 17,4 s** (173,8 s au ticket 11). 5 109 jetons sur 5 130 environ
sont repris : l'inventaire étant le même d'une phrase à l'autre, il ne reste à lire que la
phrase et l'ouverture du tour assistant. 22 des 24 phrases passent sous 30 s. Quand
l'inventaire change (une couche ouverte ou fermée), seule la partie fixe est reprise : sur
l'inventaire de l'épreuve, environ 936 jetons (5 109 − 4 173, relevé de `cache_reel`) sont
relus, soit environ 28 s au débit de lecture à 4 fils (environ 33 jetons/s). Estimation,
non mesurée sur CPU.

**Verdict S17 : 61,4 s, au-dessus des 30 s.** La lecture de son invite ne coûte plus que
2,2 s ; il reste l'écriture des 247 jetons de son plan à cinq étapes, 59,1 s à environ
4 jetons/s. Ce débit est borné par la mémoire, pas par le nombre de fils, et le cache n'y
peut rien : il gagne la lecture, pas l'écriture. S19 (164 jetons écrits, 40,8 s) est dans
le même cas. L'écart est porté par l'entrée 10 ; c'est la longueur du plan écrit qu'il
faut réduire, ou le débit d'écriture qu'il faut augmenter.

**Justesse.** Sur GPU (`--features cuda`), `plan_reel` donne 23/25 plans conformes avec
l'ordre nouveau du message système (échecs S10 et S21). `cache_reel` vérifie sur le vrai
modèle qu'à invite égale la sortie est identique avec ou sans instantané, que le 2ᵉ appel
reprend 5 109 jetons sans recharger le modèle, et qu'un inventaire changé ne reprend que
la partie fixe (4 173 jetons). Sur CPU, S05 et S10 n'ont pas la même issue qu'au ticket 11
(S10 : un plan au lieu d'un refus) : l'invite a changé d'ordre (l'inventaire vient après
les exemples) et la partie fixe est désormais découpée en jetons à part de l'inventaire, ce
qui peut changer les jetons à la jonction. Ce n'est pas une mesure de justesse.

## Modèle ajusté (ticket 5.10, tâche 4b)

`CASSINI_TEST_MODEL` prend aussi une déclaration `*.modele.json` (base + adaptateur ou modèle
fusionné, prompt court). Avec `CASSINI_TEST_INVENTORY`, le scénario commence par `A0`, la phrase
de démo exacte de `CLAUDE.md`, précédée d'une amorce exclue des moyennes. Relevé du 2026-10-23
(4 fils, `0x55`) : A0 en 41,4 s avec le modèle fusionné, invite de 1 737 jetons ; détail dans
`entrainement-du-modele.md`, « Tâche 4 : mesures ».
