# Modèle magnétique mondial WMM2025

Coefficients du modèle magnétique mondial (World Magnetic Model, WMM2025), publiés par la
NOAA (National Centers for Environmental Information) et le British Geological Survey, et
leurs valeurs de test. Le noyau les compile dans son binaire (`core/src/wmm.rs`,
`include_str!`) : aucun fichier n'est lu à l'exécution, rien n'est téléchargé (D6).

| Fichier | Contenu | SHA-256 |
|---|---|---|
| `WMM2025.COF` | coefficients de Gauss g, h et leur variation séculaire, degré 12 ; en-tête « 2025.0 WMM-2025 11/13/2024 » (identique au `WMM.COF` du paquet) | `dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f` |
| `WMM2025_TestValues.txt` | 100 valeurs de test publiées (année décimale, altitude en km, latitude, longitude, D, I, H, X, Y, Z, F, dérivées) ; lues par les tests seulement | `e6975b093dddeb6153e0b23cc418425c438167e7c5b1dd795da379cb654f5819` |

- **Source** : <https://www.ncei.noaa.gov/products/world-magnetic-model>, paquet
  « WMM2025COF » téléchargé par le propriétaire le 2026-11-10 ; fichiers recopiés tels quels.
- **Rapport technique** : NOAA et BGS, *The US/UK World Magnetic Model for 2025-2030:
  Technical Report*, doi:10.25923/prbc-s316. Les équations du noyau sont écrites d'après son
  §1.2, jamais d'après le logiciel de la NOAA.
- **Droits** : données du gouvernement des États-Unis, non soumises au droit d'auteur
  (17 U.S.C. 403) ; domaine public.
- **Validité** : du 2025-01-01 au 2029-12-31. Une date hors de cette période est calculée
  quand même, avec un avertissement.
- **Mise à jour** : le prochain modèle (WMM2030, attendu fin 2029), ou une révision hors cycle,
  arrive par une nouvelle version du noyau, installée par « Mettre à jour Cassini » ou par la
  porte hors ligne (AD-17) ; jamais par le réseau du noyau. Remplacer les deux fichiers, mettre
  à jour ce tableau et les tests de `core/src/wmm.rs`.
