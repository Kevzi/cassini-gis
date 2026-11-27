//! Refus au démarrage d'un programme mal composé (entrée 2.14), par les points d'entrée, et
//! recette du catalogue paramétrée sur le programme d'exemple.

use cassini::{AddedCommands, Program, ProgramInfo};
use cassini_core::recipe::{check_added_catalogue, check_catalogue};
use cassini_core::registry::{Command, CommandFailure, CommandSpec, Registry, RunsIn};
use programme_exemple::{program, CATALOGUE};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NoParams {}

#[derive(Serialize, JsonSchema)]
struct NoOutput {}

macro_rules! named_command {
    ($name:ident, $id:literal, $runs_in:expr) => {
        struct $name;
        impl Command for $name {
            type Params = NoParams;
            type Output = NoOutput;

            fn spec() -> CommandSpec {
                let spec = CommandSpec::new::<NoParams, NoOutput>($id, "Essai.", "Test.")
                    .cancellable(false);
                match $runs_in {
                    Some(runs_in) => spec.runs_in(runs_in),
                    None => spec,
                }
            }

            fn run(&self, _params: NoParams) -> Result<NoOutput, CommandFailure> {
                Ok(NoOutput {})
            }
        }
    };
}

named_command!(InCatalogueFamily, "layer.foo", Some(RunsIn::Core));
named_command!(OfficialId, "system.about", Some(RunsIn::Core));
named_command!(Undeclared, "exemple.sans_lieu", None);
named_command!(DeclaredQgis, "exemple.vers_qgis", Some(RunsIn::Qgis));

/// Lance le programme `Program` sur `args` et rend (code, erreur standard).
fn start(program: &Program, args: &[&str]) -> (i32, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cassini::run_program(
        program,
        &args,
        &mut std::io::Cursor::new(Vec::new()),
        &mut out,
        &mut err,
    );
    (code, String::from_utf8(err).unwrap())
}

fn refused(added: fn() -> AddedCommands, id: &str, code: &str) {
    let program = Program::with_added(ProgramInfo::new("derive", "1.0.0"), added);
    for entry in ["run", "serve", "mcp", "replay"] {
        let args: &[&str] = match entry {
            "run" => &["run", "system.about"],
            "replay" => &["replay", "inexistant.qgz"],
            other => &[other],
        };
        let (status, err) = start(&program, args);
        assert_eq!(status, 1, "{entry} : {err}");
        let report: Value = serde_json::from_str(err.trim()).unwrap();
        assert_eq!(report["code"], code, "{entry} : {report}");
        assert!(
            report["message"].as_str().unwrap().contains(id),
            "{entry} : {report}"
        );
    }
}

#[test]
fn a_command_in_a_catalogue_family_stops_every_entry_point_naming_the_id() {
    refused(
        || AddedCommands::new().with(InCatalogueFamily),
        "layer.foo",
        "program.reserved_family",
    );
}

#[test]
fn an_official_id_a_missing_runs_in_and_a_qgis_command_are_refused_naming_the_id() {
    refused(
        || AddedCommands::new().with(OfficialId),
        "system.about",
        "program.duplicate_id",
    );
    refused(
        || AddedCommands::new().with(Undeclared),
        "exemple.sans_lieu",
        "program.not_core",
    );
    refused(
        || AddedCommands::new().with(DeclaredQgis),
        "exemple.vers_qgis",
        "program.not_core",
    );
}

#[test]
fn the_example_program_starts() {
    let registry = program().registry().unwrap();
    // `with_added` ne contrôle pas le contrat : l'intégrateur le fait chez lui.
    assert!(registry.check_contract().is_empty());
    assert_eq!(registry.added_commands(), ["exemple.echo"]);
    assert_eq!(registry.program().name, "programme-exemple");
}

#[test]
fn the_parameterized_recipe_is_green_on_the_example_and_on_the_official_catalogue() {
    let registry = program().registry().unwrap();
    check_added_catalogue(&registry, CATALOGUE).unwrap_or_else(|e| panic!("{}", e.join("\n")));

    // Le catalogue public reste celui du registre officiel seul.
    let public = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md",
    ))
    .unwrap();
    check_catalogue(&Registry::standard(), &public).unwrap();
}

#[test]
fn the_recipe_names_a_gap_between_the_example_catalogue_and_its_registry() {
    let registry = program().registry().unwrap();
    let drifted = CATALOGUE.replace(r#""result": ["text", "length"]"#, r#""result": ["text"]"#);
    let errors = check_added_catalogue(&registry, &drifted).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("exemple.echo") && e.contains("length")),
        "{errors:?}"
    );
}
