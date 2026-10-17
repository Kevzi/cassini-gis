//! Commandes de test du journal (2.5), réservées aux tests : une entrée relative
//! au projet, une sortie dans le dossier de travail, une commande déléguée à QGIS.
//! Propriétaire unique, réutilisé par les histoires suivantes.

#![cfg(test)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{Command, CommandFailure, CommandSpec, Delegated, Registration, RunsIn};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadFileParams {
    /// Fichier d'entrée, relatif au projet.
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BytesOutput {
    pub bytes: u64,
}

/// `test.read_file` : taille d'un fichier d'entrée (0 s'il est illisible).
pub struct TestReadFile;

impl Command for TestReadFile {
    type Params = ReadFileParams;
    type Output = BytesOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<ReadFileParams, BytesOutput>(
            "test.read_file",
            "Donne la taille d'un fichier d'entrée (tests seulement).",
            "Returns the size of an input file (tests only).",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["path"])
        .example(serde_json::json!({"path": "a"}))
        .tests(&["read_file_reports_the_size"])
    }

    fn run(&self, params: ReadFileParams) -> Result<BytesOutput, CommandFailure> {
        let bytes = std::fs::metadata(params.path).map(|m| m.len()).unwrap_or(0);
        Ok(BytesOutput { bytes })
    }
}

inventory::submit! { Registration(|r| r.register(TestReadFile)) }

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteOutputParams {
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    pub text: String,
}

/// `test.write_output` : écrit un texte dans la sortie donnée par le noyau.
pub struct TestWriteOutput;

impl Command for TestWriteOutput {
    type Params = WriteOutputParams;
    type Output = BytesOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<WriteOutputParams, BytesOutput>(
            "test.write_output",
            "Écrit un texte dans le fichier de sortie du noyau (tests seulement).",
            "Writes a text to the core-provided output file (tests only).",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .output_params(&["out"])
        .tests(&["write_output_writes_the_text"])
    }

    fn run(&self, params: WriteOutputParams) -> Result<BytesOutput, CommandFailure> {
        let written = std::fs::write(&params.out, &params.text).is_ok();
        Ok(BytesOutput {
            bytes: if written { params.text.len() as u64 } else { 0 },
        })
    }
}

inventory::submit! { Registration(|r| r.register(TestWriteOutput)) }

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QgisProbeParams {
    pub layer: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct QgisProbeOutput {
    pub ok: bool,
}

/// `test.qgis_probe` : commande déléguée à QGIS, servie par un exécutant factice.
pub struct TestQgisProbe;

impl Delegated for TestQgisProbe {
    type Params = QgisProbeParams;
    type Output = QgisProbeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<QgisProbeParams, QgisProbeOutput>(
            "test.qgis_probe",
            "Sonde déléguée à QGIS (tests seulement).",
            "QGIS-delegated probe (tests only).",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .tests(&["qgis_probe_is_delegated"])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<TestQgisProbe>()) }

#[cfg(test)]
mod tests {
    use crate::registry::Registry;
    use serde_json::json;

    #[test]
    fn read_file_reports_the_size() {
        let path = std::env::temp_dir().join(format!("cassini-rf-{}.txt", std::process::id()));
        std::fs::write(&path, "abcd").unwrap();
        let out = Registry::standard()
            .call("test.read_file", json!({"path": path.to_string_lossy()}))
            .unwrap();
        assert_eq!(out, json!({"bytes": 4}));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn write_output_writes_the_text() {
        let path = std::env::temp_dir().join(format!("cassini-wo-{}.out", std::process::id()));
        let out = Registry::standard()
            .call(
                "test.write_output",
                json!({"out": path.to_string_lossy(), "text": "ok"}),
            )
            .unwrap();
        assert_eq!(out, json!({"bytes": 2}));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ok");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn qgis_probe_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("test.qgis_probe").unwrap();
        assert_eq!(spec.runs_in, Some(crate::registry::RunsIn::Qgis));
        let err = registry
            .call("test.qgis_probe", json!({"layer": "l"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }
}
