# Amendement d'architecture : journal chaîné par empreinte (2026-11-10)

Statut : **proposé, en attente du propriétaire.** Rien n'est appliqué au contrat
d'architecture ni aux tickets. La party du 2026-11-10 donne son avis au §6.

## 1. Le problème

- **Déclencheur :** demande du propriétaire du 2026-11-10 : traçabilité et audit des
  bureaux d'études, qui rendent à leurs clients des cartes et des analyses dont ils doivent
  pouvoir montrer le chemin.
- **Constat :** AD-11 fait du journal un fichier en **ajout seul** à un seul écrivain. C'est
  une discipline d'écriture du noyau, pas une preuve : une ligne modifiée, retirée ou insérée
  à la main dans `journal.jsonl` ne se voit pas. Le rejeu rejoue ce qu'il lit.
- **Besoin :** pouvoir dire, à la réouverture d'un projet ou au rejeu, « ce journal est celui
  que Cassini a écrit, ligne pour ligne », ou « il a été modifié à partir de la ligne k ».

## 2. Proposition

1. Chaque ligne écrite porte une clé facultative `prev` : l'empreinte **SHA-256**, en
   hexadécimal, des **octets exacts** de la ligne précédente telle qu'elle est écrite dans le
   fichier (sans son saut de ligne). On hache les octets écrits, pas une forme recanonisée :
   la vérification ne dépend pas du sérialiseur.
2. La première ligne d'un journal neuf porte `prev: null` ; la clé `chain` (valeur `1`,
   version du chaînage) dit qu'une ligne est chaînée.
3. **Vérification** à l'ouverture de la session (lecture seule comprise) : le noyau relit le
   journal, recalcule les empreintes et rend dans la réponse de `session.hello` l'état du
   chaînage : `intact`, `unchained` (non chaîné jusqu'à la ligne k) ou `broken` (rompu à la ligne k). Le panneau
   l'affiche en clair.
4. **Rejeu :** une chaîne rompue est **signalée avant de commencer, sans refuser** (même règle
   qu'AD-11 pour les versions de QGIS et de chaîne) ; le rejeu ne lit pas `prev` pour rejouer.
5. **Empreinte de tête** : l'empreinte de la dernière ligne est rendue par le noyau ; elle peut
   être imprimée, abrégée, au pied d'un export PDF (option à trancher, voir §6), pour qu'une
   carte rendue à un client désigne l'état exact du journal qui l'a produite.

**Limite assumée :** un chaînage sans secret prouve l'**intégrité** (modification
détectée), pas l'**authenticité** : quelqu'un qui réécrit tout le fichier en recalculant les
empreintes n'est pas détecté. L'empreinte de tête recopiée hors du poste (dans un PDF, un
rapport) referme ce trou pour ce qui a été rendu. Une signature demanderait une gestion de
clés, hors de ce périmètre.

## 3. Effet sur les anciens journaux et sur le rejeu

- Un journal écrit avant l'amendement n'a ni `prev` ni `chain` : il est lu `unchained` et se
  rejoue **exactement comme aujourd'hui** (la clé est ignorée du rejeu, comme `plan_step`).
- Une session qui ajoute à un ancien journal chaîne sa première ligne aux octets de la
  dernière ligne ancienne : la vérification dit « chaîné à partir de la ligne k ». Les lignes
  anciennes ne sont jamais réécrites (ajout seul).
- Une dernière ligne tronquée par un arrêt brutal : comportement à fixer au ticket, sans rien
  réécrire (la ligne suivante se chaîne aux octets lus, et la vérification signale la ligne
  incomplète).
- Le verrou exclusif d'AD-11 garantit un seul écrivain : le chaînage n'ajoute pas de course.

## 4. Texte proposé pour AD-11 (non appliqué)

> - **Amendement (proposé le 2026-11-10, changement de cap `change-journal-chaine-par-empreinte`) —
>   journal chaîné.** Chaque ligne écrite porte `prev`, l'empreinte SHA-256 des octets de la
>   ligne précédente (`null` pour la première), et `chain`, la version du chaînage. Le noyau
>   vérifie la chaîne à l'ouverture de la session et en rend l'état ; une chaîne rompue est
>   signalée avant le rejeu, sans refuser. Le rejeu ne lit pas ces clés ; une ligne qui ne les
>   porte pas (journal ancien) est non chaînée et se rejoue à l'identique. Le chaînage prouve
>   l'intégrité du journal, pas son auteur.

## 5. Dépendance et risques

- **SHA-256 dans le noyau :** la crate `sha2` (RustCrypto, **MIT OR Apache-2.0**) est déjà au
  `Cargo.lock`, tirée par une dépendance de `/cli`. **Décision du propriétaire du 2026-11-10 :
  « ok pour sha2 »** — la crate est acceptée dans le noyau pour cet amendement et pour
  `change-empreinte-du-modele-verifiee`. Le reste de l'amendement attend toujours son accord.
- **Risque : high** (AD-11, rejeu) : relecture à plusieurs lentilles (`thorough`, CLAUDE.md §8).
- Coût : une empreinte par ligne écrite, négligeable ; la vérification relit le journal à
  l'ouverture, linéaire en sa taille.

## 6. Avis de la party du 2026-11-10

> 🏛️ **Architecte :** Pour, à trois conditions : le rejeu ne lit pas la clé, une chaîne rompue
> se signale sans refuser, et on hache les octets écrits, pas un JSON recanonisé. Sinon on
> casse les journaux d'hier.
>
> 📐 **Géomètre :** Pour. Un levé sans carnet vérifiable ne vaut rien devant un expert ; un
> journal, c'est pareil.
>
> 🗺️ **Cartographe :** Pour, et je veux l'empreinte de tête au pied du PDF. Sans ça, la chaîne
> ne prouve rien à mon client, qui ne voit jamais le journal.
>
> 🏛️ **Architecte :** Au pied du PDF, en option seulement : ça touche `layout.export_pdf`, donc le
> catalogue. Pas dans le même ticket.

**Recommandation de la party :** **approuver**, après la démo ; l'empreinte de tête dans le PDF
en décision séparée (changement de cap du catalogue si retenue).

## 7. Entrée prévue (non créée)

Epic 2 (`epic-registre-projet-import`), après 2.12 :

- « Journal chaîné par empreinte » — `covers` CAP-7, CAP-10 ; `after` 2.12 ; `risk` high ;
  vérification : tests du noyau sans QGIS — une ligne modifiée, retirée ou insérée est
  signalée à la bonne ligne ; un journal ancien est `unchained` et se rejoue à l'identique ;
  un ancien journal prolongé est chaîné à partir de sa première ligne neuve ; une session en
  lecture seule vérifie sans écrire ; recette QGIS : rejeu identique après réouverture.

À créer par `bmad-ticket` seulement après l'accord du propriétaire sur l'amendement (la crate `sha2` est déjà acceptée le 2026-11-10).
