# Amendement d'architecture : empreinte du modèle vérifiée au lancement (2026-11-10)

Statut : **approuvé par le propriétaire le 2026-11-10** (« ok pour 7, 8 et 9 »), avec les
recommandations de la party du §6. Conditions de l'approbation :

- **sous réserve de mesurer le temps de lancement sur l'i5** de référence (§3), mesure consignée
  avant de trancher le calcul en parallèle du chargement ;
- les empreintes viennent du **seul manifeste des versions** de la release (§2.4) ;
- l'empreinte est **facultative pendant une version** : une déclaration sans empreintes ou un GGUF
  seul sont acceptés et signalés ; elle ne devient obligatoire que si le propriétaire le confirme.

Le texte du §4 est porté à AD-15 du contrat d'architecture le 2026-11-10, avec un renvoi sous
AD-17 ; l'entrée du §7 est créée (5.51).

## 1. Le problème

- **Déclencheur :** demande du propriétaire du 2026-11-10.
- **Constat :** l'installeur en ligne (porte 1, amendement du 2026-11-10 d'AD-17) vérifie
  l'empreinte SHA-256 de chaque fichier **au moment de l'installer**, d'après le manifeste des
  versions. Après, plus rien : un GGUF ou un adaptateur abîmé (copie interrompue, disque
  défaillant), remplacé ou modifié est chargé tel quel. Un modèle posé à la main (porte 2 sur
  une clé USB, dossier désigné par `CASSINI_MODELS`) n'est jamais vérifié.
- **Effet :** un plan composé par un modèle qui n'est pas celui qu'on croit, sans que rien ne
  le dise ; et un journal qui ne permet pas de dire quel modèle exact a composé un plan.

## 2. Proposition

1. La déclaration `*.modele.json` (lue par le noyau seul, AD-15) gagne `model_sha256` et, s'il y
   a un adaptateur, `adapter_sha256` : empreintes SHA-256, en hexadécimal, des fichiers qu'elle
   nomme.
2. Au premier chargement d'un modèle dans la session, le noyau calcule les empreintes et
   **refuse** un fichier qui ne correspond pas : erreur `models.tampered` (AD-21), qui nomme le
   fichier par son seul nom (AD-9) et dit de réinstaller par l'installeur ou de recopier depuis
   la porte hors ligne. **Jamais de téléchargement de réparation** (AD-17).
3. Le résultat est gardé **en mémoire pour la session** (fichier, taille, date de
   modification) ; aucun cache sur disque, qu'il suffirait d'éditer pour contourner la
   vérification.
4. **Une seule source pour les empreintes :** le manifeste des versions de la release. L'installeur
   en ligne recopie dans la déclaration qu'il installe les empreintes qu'il vient de vérifier ;
   la porte hors ligne embarque des déclarations déjà remplies ; la chaîne d'entraînement
   reproductible écrit les empreintes quand elle produit une déclaration.
5. **Traçabilité (option) :** chaque ligne de plan du journal porte le nom de la déclaration et
   l'empreinte abrégée du modèle qui l'a composée ; le rejeu ne la lit pas (même règle que
   `plan_step`, AD-11).

**Cas de transition :** une déclaration sans empreintes (antérieure à l'amendement) est
acceptée, et le panneau dit « empreinte du modèle non déclarée ». Un GGUF seul, sans
déclaration (essais, développement), est accepté de même. Le propriétaire décide si, après une
version publiée, l'empreinte devient obligatoire.

**Limite assumée :** comme pour le journal, c'est une preuve d'**intégrité**, pas
d'authenticité : qui modifie à la fois le GGUF et sa déclaration n'est pas détecté. Une
signature du manifeste demanderait une gestion de clés, hors de ce périmètre.

## 3. Coût et exigence des 30 s

Hacher un GGUF de plusieurs gigaoctets coûte quelques secondes de processeur, **une fois par
session**, au chargement du modèle, jamais par phrase : l'exigence « une phrase en moins de
30 s » n'est pas touchée, le temps de lancement l'est. À **mesurer sur le poste i5 de
référence** avant de trancher ; si le coût gêne, le calcul se fait en parallèle du chargement
en mémoire (le fichier est lu de toute façon), et la première phrase attend la fin de la
vérification.

## 4. Texte porté à AD-15 (appliqué le 2026-11-10), avec renvoi à AD-17

> - **Amendement (2026-11-10, changement de cap `change-empreinte-du-modele-verifiee`, approuvé
>   par le propriétaire le 2026-11-10) —
>   modèle vérifié.** Une déclaration `*.modele.json` porte l'empreinte SHA-256 de chaque
>   fichier qu'elle nomme. Le noyau la vérifie au premier chargement dans la session et refuse
>   un fichier qui ne correspond pas (`models.tampered`), sans jamais rien télécharger. Les
>   empreintes viennent du manifeste des versions des deux portes d'AD-17. Une déclaration
>   sans empreintes ou un GGUF seul sont acceptés et signalés.

AD-17 ne change pas : la déclaration remplie par l'installeur est une conséquence de sa règle
« empreintes vérifiées ».

## 5. Dépendance et risques

- Même besoin de SHA-256 que le journal chaîné (`change-journal-chaine-par-empreinte`) : crate
  `sha2` (**MIT OR Apache-2.0**), déjà au `Cargo.lock` par `/cli`, à ajouter à la crate qui lit la
  déclaration. **Décision du propriétaire du 2026-11-10 : « ok pour sha2 »** — la crate est
  acceptée dans le noyau pour les deux amendements. Le reste de l'amendement est approuvé
  le 2026-11-10 (« ok pour 7, 8 et 9 »).
- **Risque : high** (chargement du modèle, comportement visible au lancement, installeur et
  chaîne d'entraînement à aligner).

## 6. Avis de la party du 2026-11-10

> 🏛️ **Architecte :** Pour. Le noyau seul sait ce qu'est un modèle (AD-15), c'est donc le noyau
> qui vérifie. Et une seule source d'empreintes, le manifeste : pas trois listes.
>
> 🗺️ **Cartographe :** Pour, si le lancement ne devient pas une attente de trente secondes. Mes
> collègues ouvrent QGIS dix fois par jour.
>
> 📐 **Géomètre :** Un instrument non étalonné ne sort pas du coffre. Le même modèle, la même
> empreinte, le même plan : c'est ce que je veux lire dans le journal.
>
> 🏛️ **Architecte :** Alors on mesure d'abord, et on garde l'empreinte au journal en option, sans
> que le rejeu la lise.

**Recommandation de la party :** **approuver**, sous réserve de la mesure du temps de lancement
sur l'i5 ; empreinte facultative pendant une version, puis obligatoire si le propriétaire le
confirme.

## 7. Entrée créée le 2026-11-10 : 5.51

Epic 5 (`epic-agent-et-mcp`), après 5.50 :

- « Empreinte du modèle vérifiée au lancement » — `covers` CAP-4, CAP-13 ; `after` 5.50 ;
  `risk` high ; vérification : tests du noyau — une déclaration juste charge, un octet changé
  dans le GGUF ou l'adaptateur rend `models.tampered` sans charger, une déclaration sans
  empreintes charge et le signale ; durée du lancement mesurée sur l'i5 et consignée ;
  l'installeur et la chaîne d'entraînement écrivent les empreintes du manifeste.

Créée par `bmad-ticket` le 2026-11-10 : entrée **5.51** de `epic-agent-et-mcp`, placée après
5.50, `after` = 5.50, `risk` high.
