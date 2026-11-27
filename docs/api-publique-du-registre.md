# API publique du registre : composer un programme noyau

Un intégrateur peut ajouter ses commandes métier à Cassini **sans forker le dépôt** : il écrit
son propre programme, qui reprend le registre officiel et les quatre points d'entrée
(`run`, `serve`, `mcp`, `replay`), et y enregistre ses commandes. Le programme est **composé à la
compilation** : rien n'est chargé à l'exécution, ni bibliothèque partagée, ni script (AD-13,
amendement du 2026-11-10, entrée 2.14).

## Surface publique (version 1)

La version de l'API est `REGISTRY_API_VERSION` (`cassini_core::registry`), distincte de
`PROTOCOL_VERSION`, qui versionne le canal. Elle monte à chaque changement incompatible de la
surface ci-dessous.

| Élément | Crate | Rôle |
|---|---|---|
| `Command`, `CommandSpec`, `CommandFailure` | `cassini-core` | écrire une commande : types Rust, schémas JSON engendrés |
| `AddedCommands::new().with(MaCommande)` | `cassini-core` (réexporté par `cassini`) | la liste des commandes ajoutées |
| `ProgramInfo::new(nom, version)` | idem | identité du programme, annoncée |
| `Registry::with_added(info, ajoutees)` | `cassini-core` | compose le registre, ou refuse |
| `Registry::program()`, `added_commands()` | `cassini-core` | ce qu'annonce le registre |
| `Program::with_added(info, \|\| ajoutees)` | `cassini` (bibliothèque) | le programme passé aux points d'entrée |
| `cassini::run_program(&programme, args, entrée, sortie, erreurs)` | `cassini` | les quatre points d'entrée |
| `recipe::check_catalogue`, `recipe::check_added_catalogue` | `cassini-core` | la recette du catalogue, paramétrée |

`Registry::standard()` et le comportement du programme officiel ne changent pas : le binaire
`cassini` appelle `cassini::run`, qui prend `Program::official()`.

## Écrire un programme

Le modèle est `tests/programme-exemple/` : une bibliothèque qui compose le programme, un binaire
de quelques lignes.

```rust
pub fn program() -> cassini::Program {
    cassini::Program::with_added(
        cassini::ProgramInfo::new("mon-programme", env!("CARGO_PKG_VERSION")),
        || cassini::AddedCommands::new().with(MaCommande),
    )
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cassini::run_program(
        &mon_programme::program(),
        &args,
        &mut std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    std::process::ExitCode::from(code as u8)
}
```

Le serveur MCP reconstruit le programme dans son fil d'exécution : la fonction passée à
`Program::with_added` doit pouvoir être appelée plusieurs fois.

## Ce que le registre refuse, au démarrage

Un programme mal composé **ne démarre pas** : chaque point d'entrée rend le code de sortie 1 et un
rapport d'erreur JSON sur la sortie d'erreur, dont le message nomme l'identifiant.

| Code | Cas |
|---|---|
| `program.reserved_family` | l'identifiant appartient à une famille du catalogue public (`layer`, `project`, `analysis`, `network`, `raster`, `data`, `layout`, `geocode`, `terrain`, `coord`, `geo`, `map`, `system`, `model`) |
| `program.duplicate_id` | l'identifiant est déjà au registre (officiel, ou ajouté plus tôt) |
| `program.not_core` | la commande ne déclare pas `runs_in: core`, ou se déclare déléguée à QGIS |
| `program.invalid_id` | l'identifiant n'a pas la forme `domaine.verbe` en snake_case |
| `program.name` | le nom du programme est vide ou usurpe `cassini` |

Une commande ajoutée **calcule dans le noyau seulement** : la table des gestes de l'extension est
fermée et sous GPL (AD-24). Elle rend ses sorties par les conventions existantes (raster par
`layer.open_raster` dicté, entités en GeoJSON par `layer.open_vector` dicté, style par
`layer.set_style`).

## Seule voie supportée : `AddedCommands`

N'inscrivez jamais une commande par `inventory::submit!` dans votre crate : cette voie contourne
les contrôles de `with_added` (famille réservée, collision, `runs_in`) et la commande n'est pas
annoncée (`added_commands`). `AddedCommands` est la seule voie supportée. `with_added` ne lance pas
`check_contract` : lancez-le vous-même sur le registre de votre programme, dans vos tests, et
exigez une liste vide.

```rust
let registry = program().registry().unwrap();
assert!(registry.check_contract().is_empty());
```

(`Registry::check_contract`, dans `cassini_core::contract`, rend les `ContractViolation`.)

## Annonce, journal et rejeu

`system.about` et la réponse de `session.hello` portent quatre champs, figés :
`program_name`, `program_version`, `core_version` (le noyau dont le programme dérive) et
`added_commands` (les identifiants ajoutés). Le programme officiel annonce `cassini` et une liste
vide. Les commandes ajoutées apparaissent au catalogue de la session comme les autres.

Elles sont journalisées et rejouées comme les autres. Un journal du programme dérivé rejoué par un
autre programme **échoue en nommant la commande manquante et l'étape** (`registry.unknown_command`) :
rien n'est sauté en silence.

## Catalogue et recette

Le catalogue public décrit le registre officiel seul (`core/tests/catalogue.rs`). L'intégrateur tient
**son propre catalogue**, au même format (un bloc ` ```json fiche ` par commande), et lance la même
recette chez lui :

```rust
cassini_core::recipe::check_added_catalogue(&registry, CATALOGUE).unwrap();
```

`check_catalogue` confronte tout le registre, `check_added_catalogue` les seules commandes ajoutées.
Voir `tests/programme-exemple/catalogue.md` et ses tests.

## Exigences pour l'intégrateur

- **AD-1 : ne lier ni QGIS, ni GDAL, ni GEOS, ni PROJ.** C'est ce qui fonde l'argument de distance
  de bras (CLAUDE.md §2.3 et §2.4) pour le programme dérivé aussi. Cassini ne peut pas le vérifier
  chez l'intégrateur.
- Les crates de Cassini sont sous Apache-2.0 ; la licence des commandes ajoutées est le choix de
  l'intégrateur.
- La preuve hors ligne d'AD-14 (recette débranchée) ne porte que sur le programme officiel :
  l'intégrateur lance la même recette sur son programme.
- Le modèle local ne connaît pas les commandes ajoutées (entrée 5.52) ; elles restent offertes par
  le formulaire du panneau et par le MCP.
