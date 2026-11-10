use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::registry::RegistryError;

#[test]
fn output_file_uses_the_declared_extension() {
    let one = ["out".to_string()];
    assert_eq!(
        output_file(1, "analysis.buffer", &one, "out", ".gpkg"),
        "1-analysis.buffer.gpkg"
    );
    assert_eq!(output_file(2, "x", &one, "out", ".out"), "2-x.out");
    let two = ["a".to_string(), "b".to_string()];
    assert_eq!(output_file(3, "x", &two, "b", ".gpkg"), "3-x-b.gpkg");
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Dossier de projet neuf et unique ; rend (dossier, fichier de projet, jeton unique).
fn temp_project() -> (PathBuf, PathBuf, String) {
    let token = format!(
        "cassini-journal-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let dir = std::env::temp_dir().join(&token);
    fs::create_dir_all(&dir).unwrap();
    let project = dir.join("p.qgz");
    (dir, project, token)
}

fn lines(session: &Session) -> Vec<Value> {
    fs::read_to_string(session.journal_path())
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

struct FakeQgis;
impl Executor for FakeQgis {
    fn execute(&self, _id: &str, _p: Value) -> Result<Value, RegistryError> {
        Ok(json!({"ok": true}))
    }
}

#[test]
fn three_commands_give_three_lines_without_absolute_paths() {
    let (dir, project, token) = temp_project();
    fs::write(dir.join("data.txt"), "abc").unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    assert_eq!(s.mode(), Mode::Write);
    s.call("system.about", json!({})).unwrap();
    // Chemin donné en absolu : journalisé en relatif.
    let absolute = dir.join("data.txt").to_string_lossy().into_owned();
    let out = s.call("test.read_file", json!({"path": absolute})).unwrap();
    assert_eq!(out, json!({"bytes": 3}));
    s.call("test.write_output", json!({"text": "salut"}))
        .unwrap();

    let raw = fs::read_to_string(s.journal_path()).unwrap();
    assert_eq!(raw.lines().count(), 3);
    assert!(
        !raw.contains(&token),
        "chemin absolu dans le journal : {raw}"
    );
    let l = lines(&s);
    assert_eq!(l[0]["step"], 1);
    assert_eq!(l[1]["params"], json!({"path": "data.txt"}));
    assert_eq!(l[1]["inputs"][0]["path"], "data.txt");
    assert_eq!(l[1]["inputs"][0]["portable"], true);
    assert_eq!(l[1]["inputs"][0]["size"], 3);
    assert!(l[1]["inputs"][0]["modified"].is_u64());
    assert_eq!(l[2]["params"]["out"], "3-test.write_output.out");
    assert_eq!(l[2]["result"], json!({"bytes": 5}));
    assert_eq!(l[2]["versions"]["core"], crate::VERSION);
    assert_eq!(l[2]["versions"]["protocol"], PROTOCOL_VERSION);
    // La sortie est dans le dossier de travail, et nulle part ailleurs.
    assert_eq!(
        fs::read_to_string(s.work_dir().join("3-test.write_output.out")).unwrap(),
        "salut"
    );
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_second_session_is_read_only_and_writes_nothing() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut first = Session::open(&registry, &project).unwrap();
    first.call("system.about", json!({})).unwrap();
    let before = fs::read_to_string(first.journal_path()).unwrap();

    let mut second = Session::open(&registry, &project).unwrap();
    assert_eq!(second.mode(), Mode::ReadOnly);
    let err = second
        .call("test.write_output", json!({"text": "x"}))
        .unwrap_err();
    assert_eq!(err.code, "journal.read_only");
    assert_eq!(fs::read_to_string(first.journal_path()).unwrap(), before);
    assert!(!first.work_dir().join("2-test.write_output.out").exists());
    drop((first, second));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn closing_a_session_frees_the_lock_and_numbering_continues() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    {
        let mut a = Session::open(&registry, &project).unwrap();
        a.call("system.about", json!({})).unwrap();
        a.call("system.about", json!({})).unwrap();
    }
    let mut b = Session::open(&registry, &project).unwrap();
    assert_eq!(b.mode(), Mode::Write);
    assert_eq!(b.next_step(), 3);
    b.call("system.about", json!({})).unwrap();
    assert_eq!(lines(&b)[2]["step"], 3);
    drop(b);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_file_outside_the_project_is_not_portable() {
    let (dir, project, token) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.read_file", json!({"path": "../x.tif"}))
        .unwrap();
    let l = lines(&s);
    assert_eq!(l[0]["inputs"][0]["path"], "../x.tif");
    assert_eq!(l[0]["inputs"][0]["portable"], false);
    assert_eq!(l[0]["inputs"][0]["size"], Value::Null);
    // Un absolu hors projet sur le même disque : relatif, non portable.
    let outside = std::env::temp_dir().join("other-folder").join("y.tif");
    s.call("test.read_file", json!({"path": outside.to_string_lossy()}))
        .unwrap();
    let second = &lines(&s)[1];
    assert_eq!(second["inputs"][0]["portable"], false);
    assert_eq!(second["inputs"][0]["path"], "../other-folder/y.tif");
    let raw = fs::read_to_string(s.journal_path()).unwrap();
    assert!(!raw.contains(&token));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[cfg(windows)]
#[test]
fn another_drive_is_unrepresentable() {
    assert_eq!(relative_to(Path::new(r"C:\a\b"), Path::new(r"D:\x")), None);
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    let here = dir
        .to_string_lossy()
        .chars()
        .next()
        .unwrap()
        .to_ascii_uppercase();
    let other = if here == 'Z' { "Y" } else { "Z" };
    let err = s
        .call(
            "test.read_file",
            json!({"path": format!(r"{other}:\x.tif")}),
        )
        .unwrap_err();
    assert_eq!(err.code, "journal.path_unrepresentable");
    assert!(lines(&s).is_empty());
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_refused_command_writes_nothing() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    let err = s
        .call("test.read_file", json!({"path": "a", "extra": 1}))
        .unwrap_err();
    assert_eq!(err.code, "registry.invalid_params");
    let err = s.call("no.such", json!({})).unwrap_err();
    assert_eq!(err.code, "registry.unknown_command");
    assert!(!s.journal_path().exists());
    assert_eq!(s.next_step(), 1);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_failed_execution_is_journaled_with_its_code() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    let err = s
        .call("test.qgis_probe", json!({"layer": "l"}))
        .unwrap_err();
    assert_eq!(err.code, "registry.unavailable");
    let l = lines(&s);
    assert_eq!(l[0]["error"], "registry.unavailable");
    assert!(l[0].get("result").is_none());
    drop(s);

    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(FakeQgis);
    assert_eq!(
        s.call("test.qgis_probe", json!({"layer": "l"})).unwrap(),
        json!({"ok": true})
    );
    assert_eq!(lines(&s)[1]["result"], json!({"ok": true}));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_result_path_under_the_output_folder_becomes_a_file_name() {
    let out = std::env::temp_dir().join("sortie");
    let value = json!({
        "file": out.join("4-x.pdf").to_string_lossy(),
        "other": std::env::temp_dir().join("ailleurs").join("y.pdf").to_string_lossy(),
        "list": [out.join("5-z.gpkg").to_string_lossy()],
        "n": 3,
    });
    let r = relativize(&value, &out);
    assert_eq!(r["file"], "4-x.pdf");
    assert_eq!(r["list"], json!(["5-z.gpkg"]));
    assert_eq!(r["other"], value["other"]);
    assert_eq!(r["n"], 3);
}

#[test]
fn a_big_result_is_summarized() {
    let big = json!({"text": "x".repeat(500)});
    let summary = summarize(&big);
    assert_eq!(summary["truncated"], true);
    assert_eq!(summary["bytes"], big.to_string().len());
    assert_eq!(
        summarize(&json!({"b": 1, "a": 2})).to_string(),
        r#"{"a":2,"b":1}"#
    );
}

#[test]
fn params_are_canonical_and_the_registry_version_is_stable() {
    let v = canonical(&json!({"b": {"d": 1, "c": 2}, "a": [{"z": 1, "y": 2}]}));
    assert_eq!(v.to_string(), r#"{"a":[{"y":2,"z":1}],"b":{"c":2,"d":1}}"#);
    let registry = Registry::standard();
    let a = registry_version(&registry);
    assert_eq!(a, registry_version(&Registry::standard()));
    assert_eq!(a.len(), 16);
    assert_ne!(a, registry_version(&Registry::new()));
}

fn replay_of(
    registry: &Registry,
    project: &Path,
) -> Result<(ReplayReport, Vec<String>), ErrorReport> {
    let replay = Replay::load(registry, project)?;
    let warnings = replay.warnings().to_vec();
    replay
        .run(&Router::new(registry), &mut HashSet::new())
        .map(|report| (report, warnings))
}

#[test]
fn an_output_reference_is_journaled_as_is_and_replays_identically() {
    let (dir, project, token) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.write_output", json!({"text": "salut"}))
        .unwrap();
    let out = s
        .call("test.read_file", json!({"path": "out:1.1"}))
        .unwrap();
    assert_eq!(out, json!({"bytes": 5}));
    let l = lines(&s);
    assert_eq!(l[1]["params"], json!({"path": "out:1.1"}));
    assert_eq!(l[1]["inputs"], json!([]));
    let before = fs::read_to_string(s.journal_path()).unwrap();

    // La session tient encore le verrou : le rejeu reste permis.
    let (report, warnings) = replay_of(&registry, &project).unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 2,
            identical: true
        }
    );
    assert!(warnings.is_empty());
    assert_eq!(fs::read_to_string(s.journal_path()).unwrap(), before);
    let work = s.work_dir().to_path_buf();
    assert_eq!(
        fs::read_to_string(work.join("replay/1-test.write_output.out")).unwrap(),
        "salut"
    );
    assert_eq!(
        fs::read_to_string(work.join("1-test.write_output.out")).unwrap(),
        "salut"
    );
    assert!(!before.contains(&token));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Écrit un journal à la main, une ligne JSON par étape.
fn hand_journal(project: &Path, rows: &[Value]) {
    let work = project.parent().unwrap().join("p.cassini");
    fs::create_dir_all(&work).unwrap();
    let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
    fs::write(work.join(JOURNAL_FILE), text).unwrap();
}

#[test]
fn an_unknown_command_fails_by_name_and_executes_nothing() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    hand_journal(
        &project,
        &[
            json!({"step":1,"id":"test.write_output","params":{"out":"1-test.write_output.out","text":"a"},"inputs":[],"result":{"bytes":1}}),
            json!({"step":2,"id":"no.such","params":{},"inputs":[],"result":{}}),
        ],
    );
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "registry.unknown_command");
    assert!(err.message.contains("no.such"));
    assert_eq!(err.step, Some(2));
    assert!(!dir.join("p.cassini/replay").exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_qgis_command_without_executor_is_unavailable_on_replay() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.qgis_probe", json!({"layer": "l"}))
        .unwrap_err();
    drop(s);
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "registry.unavailable");
    assert!(err.message.contains("test.qgis_probe"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_non_portable_input_is_warned_and_replay_goes_on() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.read_file", json!({"path": "../x.tif"}))
        .unwrap();
    drop(s);
    let (report, warnings) = replay_of(&registry, &project).unwrap();
    assert_eq!(report.steps, 1);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("../x.tif"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_changed_input_diverges_at_its_step() {
    let (dir, project, _) = temp_project();
    fs::write(dir.join("data.txt"), "abc").unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("system.about", json!({})).unwrap();
    s.call("test.read_file", json!({"path": "data.txt"}))
        .unwrap();
    drop(s);
    fs::write(dir.join("data.txt"), "abcdef").unwrap();
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(2));
    assert!(!err.message.contains("cassini-journal-"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_output_with_other_bytes_diverges() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.write_output", json!({"text": "salut"}))
        .unwrap();
    let original = s.work_dir().join("1-test.write_output.out");
    drop(s);
    fs::write(original, "autre").unwrap();
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(1));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_invalid_reference_is_refused_both_ways() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("system.about", json!({})).unwrap();
    for bad in ["out:9.1", "out:1.1", "out:1", "out:x.1", "out:0.1"] {
        let err = s.call("test.read_file", json!({"path": bad})).unwrap_err();
        assert_eq!(err.code, "replay.bad_reference", "{bad}");
    }
    assert_eq!(lines(&s).len(), 1);
    drop(s);
    hand_journal(
        &project,
        &[
            json!({"step":1,"id":"test.read_file","params":{"path":"out:9.1"},"inputs":[],"result":{"bytes":0}}),
        ],
    );
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.bad_reference");
    assert_eq!(err.step, Some(1));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_journal_is_refused() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.no_journal");
    fs::create_dir_all(dir.join("p.cassini")).unwrap();
    fs::write(dir.join("p.cassini").join(JOURNAL_FILE), "\n").unwrap();
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.no_journal");
    let _ = fs::remove_dir_all(dir);
}

/// QGIS de test : rend un inventaire fixe, sans aucun chemin.
struct InventoryQgis;
impl Executor for InventoryQgis {
    fn execute(&self, id: &str, _p: Value) -> Result<Value, RegistryError> {
        assert_eq!(id, "project.inventory");
        Ok(json!({
            "layers": [
                {"layer_id": "a_1", "name": "gares", "source": "gares.gpkg", "crs": "EPSG:2154"},
                {"layer_id": "b_2", "name": "parcelles", "source": "parcelles.shp", "crs": "EPSG:2154"}
            ],
            "project_crs": "EPSG:2154",
            "extent": {"xmin": 1.0, "ymin": 2.0, "xmax": 3.0, "ymax": 4.0, "crs": "EPSG:2154"}
        }))
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn a_project_copied_with_its_journal_folder_replays_identically() {
    let (dir, project, token) = temp_project();
    fs::write(&project, "projet").unwrap();
    fs::write(dir.join("data.txt"), "abc").unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(InventoryQgis);
    s.call("test.read_file", json!({"path": "data.txt"}))
        .unwrap();
    s.call("project.inventory", json!({})).unwrap();
    let journal = fs::read_to_string(s.journal_path()).unwrap();
    drop(s);
    assert!(!journal.contains(&token));

    // Le projet et son dossier `.cassini` partent ensemble ailleurs.
    let (moved, moved_project, moved_token) = temp_project();
    fs::copy(&project, &moved_project).unwrap();
    fs::copy(dir.join("data.txt"), moved.join("data.txt")).unwrap();
    // `fs::copy` garde la date de modification sous Windows mais pas sous Linux :
    // une vraie copie de dossier (explorateur, `cp -p`) la conserve.
    let modified = fs::metadata(dir.join("data.txt"))
        .unwrap()
        .modified()
        .unwrap();
    fs::File::options()
        .write(true)
        .open(moved.join("data.txt"))
        .unwrap()
        .set_modified(modified)
        .unwrap();
    copy_dir(&dir.join("p.cassini"), &moved.join("p.cassini"));
    let _ = fs::remove_dir_all(&dir);

    let replay = Replay::load(&registry, &moved_project).unwrap();
    assert!(replay.warnings().is_empty());
    let report = replay
        .run(
            &Router::new(&registry).with_qgis(InventoryQgis),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 2,
            identical: true
        }
    );
    let copied = fs::read_to_string(moved.join("p.cassini").join(JOURNAL_FILE)).unwrap();
    assert_eq!(copied, journal);
    assert!(!copied.contains(&moved_token));
    let _ = fs::remove_dir_all(moved);
}

/// QGIS de test : chaque « session » (`generation`) rend d'autres ids de couche
/// et des octets de sortie différents, comme le vrai QGIS.
struct SessionQgis {
    generation: u32,
    count: std::cell::Cell<u32>,
    feature_count: u64,
    empty_output: bool,
    /// Couches créées par cette session : (id, source rendue par QGIS).
    created: std::cell::RefCell<Vec<(String, String)>>,
    /// Nombre d'entités annoncé par l'inventaire pour les couches créées.
    inventory_feature_count: u64,
    /// Inventaire réduit aux ids, gardé tel quel au journal (≤ 200 octets).
    short_inventory: bool,
    /// Une couche ouverte à la main après le tampon, sans équivalent à l'origine.
    stranger: bool,
    /// Le tampon est refusé, comme pour une couche en degrés : erreur, aucun fichier écrit.
    refuse_buffer: bool,
}

impl SessionQgis {
    fn new(generation: u32) -> Self {
        SessionQgis {
            generation,
            count: std::cell::Cell::new(0),
            feature_count: 4,
            empty_output: false,
            created: std::cell::RefCell::new(Vec::new()),
            inventory_feature_count: 4,
            short_inventory: false,
            stranger: false,
            refuse_buffer: false,
        }
    }

    fn layer_id(&self, n: u32) -> String {
        format!("couche_g{}_{n}", self.generation)
    }

    fn inventory(&self) -> Value {
        let mut layers: Vec<Value> = Vec::new();
        if !self.short_inventory {
            // Couche du projet : id stable, le projet étant rouvert depuis sa copie.
            layers.push(json!({
                "layer_id": "communes_0", "name": "communes",
                "source": "communes.gpkg", "crs": "EPSG:2154"
            }));
        }
        let mut created = self.created.borrow().clone();
        // Ouverte à la main entre le tampon et le second inventaire.
        if self.stranger && created.len() >= 2 {
            created.push((format!("main_g{}", self.generation), "main.gpkg".into()));
        }
        for (id, source) in created {
            layers.push(if self.short_inventory {
                json!({"layer_id": id, "feature_count": self.inventory_feature_count})
            } else {
                json!({
                    "layer_id": id, "name": "couche", "source": source,
                    "crs": "EPSG:2154", "feature_count": self.inventory_feature_count
                })
            });
        }
        if self.short_inventory {
            json!({"layers": layers})
        } else {
            json!({"layers": layers, "project_crs": "EPSG:2154"})
        }
    }
}

impl Executor for SessionQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        let n = self.count.get() + 1;
        self.count.set(n);
        match id {
            "layer.add" => {
                let layer = self.layer_id(n);
                self.created
                    .borrow_mut()
                    .push((layer.clone(), "pts.csv".into()));
                Ok(json!({"layer_id": layer, "name": "points"}))
            }
            "analysis.buffer" => {
                // Le paramètre `layer_id` doit être une couche de CETTE session.
                let source = p["layer_id"].as_str().unwrap();
                assert!(
                    self.created.borrow().iter().any(|(l, _)| l == source),
                    "couche d'une autre session : {source}"
                );
                if self.refuse_buffer {
                    return Err(RegistryError::Delegated {
                        code: "analysis.crs_not_metric".into(),
                        message: "couche en degrés".into(),
                    });
                }
                let bytes = if self.empty_output {
                    String::new()
                } else {
                    format!("gpkg-{}", self.generation)
                };
                let out = p["out"].as_str().unwrap().to_string();
                fs::write(&out, bytes).unwrap();
                let layer = self.layer_id(n);
                // Source absolue sous le dossier de sortie, comme le vrai QGIS.
                self.created.borrow_mut().push((layer.clone(), out));
                Ok(json!({
                    "layer_id": layer,
                    "feature_count": self.feature_count,
                }))
            }
            "project.inventory" => Ok(self.inventory()),
            other => panic!("commande inattendue : {other}"),
        }
    }
}

/// Journal de deux étapes QGIS écrit par la génération 1.
fn qgis_journal(project: &Path) {
    let registry = Registry::standard();
    let mut s = Session::open(&registry, project)
        .unwrap()
        .with_qgis(SessionQgis::new(1));
    let added = s
        .call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    let first = added["layer_id"].as_str().unwrap().to_string();
    s.call(
        "analysis.buffer",
        json!({"layer_id": first, "distance": {"value": 500, "unit": "m"}}),
    )
    .unwrap();
    let l = lines(&s);
    assert_eq!(l[0]["layer_id"], json!("couche_g1_1"));
    assert_eq!(l[1]["layer_id"], json!("couche_g1_2"));
}

/// Journal de 5.30 en petit : inventaire, couche, tampon, inventaire qui liste les
/// couches créées. Rend la taille gardée du dernier inventaire au journal.
fn inventory_journal(project: &Path, short: bool) -> Value {
    let registry = Registry::standard();
    let mut qgis = SessionQgis::new(1);
    qgis.short_inventory = short;
    let mut s = Session::open(&registry, project).unwrap().with_qgis(qgis);
    s.call("project.inventory", json!({})).unwrap();
    let added = s
        .call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    let first = added["layer_id"].as_str().unwrap().to_string();
    s.call(
        "analysis.buffer",
        json!({"layer_id": first, "distance": {"value": 500, "unit": "m"}}),
    )
    .unwrap();
    s.call("project.inventory", json!({})).unwrap();
    lines(&s)[3]["result"].clone()
}

fn replay_inventory(
    short: bool,
    tweak: impl FnOnce(&mut SessionQgis),
) -> Result<ReplayReport, ErrorReport> {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let kept = inventory_journal(&project, short);
    assert_eq!(kept.get("truncated") == Some(&json!(true)), !short);
    if short {
        assert!(kept.to_string().contains("couche_g1_3"));
    }
    let mut qgis = SessionQgis::new(2);
    qgis.short_inventory = short;
    tweak(&mut qgis);
    let outcome = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new());
    let _ = fs::remove_dir_all(dir);
    outcome
}

#[test]
fn an_inventory_after_created_layers_replays_identically() {
    for short in [false, true] {
        assert_eq!(
            replay_inventory(short, |_| {}).unwrap(),
            ReplayReport {
                steps: 4,
                identical: true
            },
            "inventaire court : {short}"
        );
    }
}

#[test]
fn an_inventory_with_a_layer_without_equivalent_diverges() {
    for short in [false, true] {
        let err = replay_inventory(short, |q| q.stranger = true).unwrap_err();
        assert_eq!(err.code, "replay.diverged");
        assert_eq!(err.step, Some(4));
        assert!(err.message.contains("project.inventory"));
    }
}

#[test]
fn an_inventory_with_another_feature_count_diverges() {
    for short in [false, true] {
        let err = replay_inventory(short, |q| q.inventory_feature_count = 5).unwrap_err();
        assert_eq!(err.code, "replay.diverged");
        assert_eq!(err.step, Some(4));
        assert!(err.message.contains("project.inventory"));
    }
}

/// Une vraie divergence garde le motif ordinaire, sans le remède de la même session.
#[test]
fn a_true_divergence_keeps_the_plain_reason() {
    for short in [false, true] {
        for tweak in [
            (|q: &mut SessionQgis| q.stranger = true) as fn(&mut SessionQgis),
            |q| q.inventory_feature_count = 5,
        ] {
            let err = replay_inventory(short, tweak).unwrap_err();
            let context = format!("inventaire court : {short} ; {}", err.message);
            assert_eq!(err.code, "replay.diverged", "{context}");
            assert_eq!(err.step, Some(4), "{context}");
            assert!(
                err.message.contains("ne redonne pas le même résultat"),
                "{context}"
            );
            assert!(!err.message.contains("Rouvrez"), "{context}");
            assert!(!err.message.contains("exécution précédente"), "{context}");
        }
    }
}

/// Rejeu dans la session d'origine : le projet porte encore les couches de la génération 1,
/// que l'inventaire de l'étape 1 voit. Le motif nomme la couche et dit le remède.
#[test]
fn a_replay_in_the_same_session_names_the_earlier_layer() {
    for short in [false, true] {
        let err = replay_inventory(short, |q| {
            q.created.borrow_mut().extend([
                ("couche_g1_2".to_string(), "pts.csv".to_string()),
                ("couche_g1_3".to_string(), "tampon.gpkg".to_string()),
            ]);
        })
        .unwrap_err();
        assert_eq!(err.code, "replay.diverged");
        assert_eq!(err.step, Some(1));
        assert!(err.message.contains("project.inventory"), "{}", err.message);
        assert!(err.message.contains("« couche_g1_2 »"), "{}", err.message);
        assert!(
            err.message.contains("exécution précédente"),
            "{}",
            err.message
        );
        assert!(err.message.contains("Rouvrez"), "{}", err.message);
    }
}

/// Même session, lecture après les créations : le résultat gardé nomme déjà les couches g1,
/// mais le rejeu en pose de neuves (g2) à côté ; l'inventaire voit les g1, motif nommé.
#[test]
fn a_read_after_the_creations_in_the_same_session_names_the_earlier_layer() {
    for short in [false, true] {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut kept = SessionQgis::new(1);
        kept.short_inventory = short;
        let mut s = Session::open(&registry, &project).unwrap().with_qgis(kept);
        let added = s
            .call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
            .unwrap();
        let first = added["layer_id"].as_str().unwrap().to_string();
        s.call(
            "analysis.buffer",
            json!({"layer_id": first, "distance": {"value": 500, "unit": "m"}}),
        )
        .unwrap();
        s.call("project.inventory", json!({})).unwrap();
        drop(s);
        let mut qgis = SessionQgis::new(2);
        qgis.short_inventory = short;
        qgis.created.borrow_mut().extend([
            ("couche_g1_1".to_string(), "pts.csv".to_string()),
            ("couche_g1_2".to_string(), "tampon.gpkg".to_string()),
        ]);
        let err = Replay::load(&registry, &project)
            .unwrap()
            .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
            .unwrap_err();
        let _ = fs::remove_dir_all(dir);
        let context = format!("inventaire court : {short} ; {}", err.message);
        assert_eq!(err.code, "replay.diverged", "{context}");
        assert_eq!(err.step, Some(3), "{context}");
        assert!(err.message.contains("« couche_g1_"), "{context}");
        assert!(err.message.contains("exécution précédente"), "{context}");
        assert!(err.message.contains("Rouvrez"), "{context}");
    }
}

/// Une couche posée par un rejeu antérieur (`replayed_layers`), vue par la lecture : même motif.
#[test]
fn a_layer_of_an_earlier_replay_is_named_too() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    inventory_journal(&project, true);
    let mut qgis = SessionQgis::new(3);
    qgis.short_inventory = true;
    qgis.created
        .borrow_mut()
        .push(("couche_g2_2".to_string(), "pts.csv".to_string()));
    let mut earlier: HashSet<String> = ["couche_g2_2".to_string()].into();
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut earlier)
        .unwrap_err();
    let _ = fs::remove_dir_all(dir);
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(1));
    assert!(err.message.contains("« couche_g2_2 »"), "{}", err.message);
    assert!(
        err.message.contains("exécution précédente"),
        "{}",
        err.message
    );
    assert!(err.message.contains("Rouvrez"), "{}", err.message);
}

#[test]
fn a_replay_binds_new_layer_ids_and_accepts_other_qgis_bytes() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    qgis_journal(&project);
    let replay = Replay::load(&registry, &project).unwrap();
    let report = replay
        .run(
            &Router::new(&registry).with_qgis(SessionQgis::new(2)),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 2,
            identical: true
        }
    );
    let work = dir.join("p.cassini");
    assert_eq!(
        fs::read_to_string(work.join("replay/2-analysis.buffer.gpkg")).unwrap(),
        "gpkg-2"
    );
    assert_eq!(
        fs::read_to_string(work.join("2-analysis.buffer.gpkg")).unwrap(),
        "gpkg-1"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_qgis_step_with_another_result_stops_the_replay_naming_the_step() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    qgis_journal(&project);
    let mut other = SessionQgis::new(2);
    other.feature_count = 5;
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(other),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(2));
    assert!(err.message.contains("analysis.buffer"));
    let _ = fs::remove_dir_all(dir);
}

/// Journal de 11.18 en petit : une couche, un tampon refusé par QGIS, puis une autre couche.
fn refused_buffer_journal(project: &Path) {
    let registry = Registry::standard();
    let mut qgis = SessionQgis::new(1);
    qgis.refuse_buffer = true;
    let mut s = Session::open(&registry, project).unwrap().with_qgis(qgis);
    let added = s
        .call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:4326"}))
        .unwrap();
    let first = added["layer_id"].as_str().unwrap().to_string();
    let err = s
        .call(
            "analysis.buffer",
            json!({"layer_id": first, "distance": {"value": 500, "unit": "m"}}),
        )
        .unwrap_err();
    assert_eq!(err.code, "analysis.crs_not_metric");
    s.call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    assert_eq!(lines(&s)[1]["error"], "analysis.crs_not_metric");
}

#[test]
fn a_qgis_step_refused_again_replays_identically_and_the_replay_goes_on() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    refused_buffer_journal(&project);
    let mut qgis = SessionQgis::new(2);
    qgis.refuse_buffer = true;
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
        .unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 3,
            identical: true
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_qgis_step_refused_at_first_but_done_on_replay_diverges() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    refused_buffer_journal(&project);
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(SessionQgis::new(2)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(2));
    assert!(err.message.contains("analysis.buffer"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_empty_qgis_output_diverges() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    qgis_journal(&project);
    let mut other = SessionQgis::new(2);
    other.empty_output = true;
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(other),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(2));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_replay_without_qgis_names_the_missing_command() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    qgis_journal(&project);
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "registry.unavailable");
    assert!(err.message.contains("layer.add"));
    assert_eq!(err.step, Some(1));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn lexical_paths_behave() {
    let base = Path::new("/p/proj");
    let t = normalize(&base.join("a/../b/./c.tif"));
    assert_eq!(posix(&relative_to(base, &t).unwrap()), "b/c.tif");
    assert_eq!(posix(&relative_to(base, base).unwrap()), ".");
}

#[test]
fn an_unreadable_line_is_named_by_its_step() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("system.about", json!({})).unwrap();
    s.call("system.about", json!({})).unwrap();
    let journal = s.journal_path();
    drop(s);
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str("{pas du json\n");
    fs::write(&journal, text).unwrap();
    let err = Replay::load(&registry, &project).err().unwrap();
    assert_eq!(err.code, "replay.unreadable_line");
    assert_eq!(err.step, Some(3));
    assert!(err.message.contains("étape 3"));
    // Une ligne sans `id` est tout aussi illisible.
    let first = fs::read_to_string(&journal).unwrap();
    let first = first.lines().next().unwrap().to_string();
    fs::write(&journal, format!("{first}\n{{\"step\":2}}\n")).unwrap();
    let err = Replay::load(&registry, &project).err().unwrap();
    assert_eq!(err.code, "replay.unreadable_line");
    assert_eq!(err.step, Some(2));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_big_result_is_compared_by_fingerprint() {
    let a = json!({"text": "a".repeat(500), "layer_id": "L1"});
    let b = json!({"text": "b".repeat(500), "layer_id": "L1"});
    let kept = summarize(&a);
    assert_eq!(kept["truncated"], true);
    assert!(kept["fingerprint"].is_string());
    // Même longueur, autre contenu : ce n'est plus « identique ».
    assert_eq!(summarize(&a)["bytes"], summarize(&b)["bytes"]);
    assert!(!same_result(&b, &kept));
    assert!(same_result(&a, &kept));
    // Le `layer_id` est une valeur de session : il ne compte pas dans l'empreinte.
    let renamed = json!({"text": "a".repeat(500), "layer_id": "L2"});
    assert!(same_result(&renamed, &kept));
    // Un journal ancien, sans empreinte, ne se compare que par la taille.
    let legacy = json!({"truncated": true, "bytes": a.to_string().len()});
    assert!(same_result(&b, &legacy));
    assert!(!same_result(&json!({"text": "c".repeat(400)}), &legacy));
}

#[test]
fn an_input_touched_since_the_journal_diverges_even_if_the_result_is_the_same() {
    let (dir, project, _) = temp_project();
    let data = dir.join("data.txt");
    fs::write(&data, "abc").unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("test.read_file", json!({"path": "data.txt"}))
        .unwrap();
    drop(s);
    // Même taille, même résultat (3 octets), mais une autre date de modification.
    fs::write(&data, "xyz").unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    File::options()
        .write(true)
        .open(&data)
        .unwrap()
        .set_modified(later)
        .unwrap();
    let err = replay_of(&registry, &project).unwrap_err();
    assert_eq!(err.code, "replay.diverged");
    assert_eq!(err.step, Some(1));
    assert!(err.message.contains("data.txt"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_external_origin_is_journaled_and_ignored_by_the_replay() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("system.about", json!({})).unwrap();
    s.call_from(
        "test.write_output",
        json!({"text": "salut"}),
        Some("external"),
    )
    .unwrap();
    let l = lines(&s);
    assert!(l[0].get("origin").is_none());
    assert_eq!(l[1]["origin"], "external");
    let before = fs::read_to_string(s.journal_path()).unwrap();
    let (report, _) = replay_of(&registry, &project).unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 2,
            identical: true
        }
    );
    // Le rejeu n'écrit rien : les lignes gardent leur origine.
    assert_eq!(fs::read_to_string(s.journal_path()).unwrap(), before);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Journal de deux étapes : une sortie écrite par le noyau, puis relue par repère.
#[cfg(unix)]
fn journal_with_output(registry: &Registry, project: &Path) -> PathBuf {
    let mut s = Session::open(registry, project).unwrap();
    s.call("test.write_output", json!({"text": "salut"}))
        .unwrap();
    s.call("test.read_file", json!({"path": "out:1.1"}))
        .unwrap();
    s.work_dir().to_path_buf()
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(unix)]
#[test]
fn a_held_replay_folder_falls_back_to_a_fresh_one() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let work = journal_with_output(&registry, &project);
    // Fichier périmé, au contenu différent, que le rejeu ne peut pas supprimer.
    let stale = work.join("replay/1-test.write_output.out");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "périmé, plus long").unwrap();
    set_mode(&work.join("replay"), 0o555);

    let outcome = replay_of(&registry, &project);
    set_mode(&work.join("replay"), 0o755);
    let (report, _) = outcome.unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 2,
            identical: true
        }
    );
    assert_eq!(
        fs::read_to_string(work.join("replay-2/1-test.write_output.out")).unwrap(),
        "salut"
    );
    assert_eq!(fs::read_to_string(&stale).unwrap(), "périmé, plus long");
    assert!(!work.join("replay-3").exists());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn a_held_first_fallback_moves_on_to_the_next() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let work = journal_with_output(&registry, &project);
    for held in ["replay", "replay-2"] {
        let stale = work.join(held).join("1-test.write_output.out");
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, "périmé").unwrap();
        set_mode(&work.join(held), 0o555);
    }

    let outcome = replay_of(&registry, &project);
    set_mode(&work.join("replay"), 0o755);
    set_mode(&work.join("replay-2"), 0o755);
    assert!(outcome.unwrap().0.identical);
    assert_eq!(
        fs::read_to_string(work.join("replay-3/1-test.write_output.out")).unwrap(),
        "salut"
    );
    assert!(!work.join("replay-4").exists());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn two_replays_in_a_row_reuse_the_replay_folder() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let work = journal_with_output(&registry, &project);
    for _ in 0..2 {
        let (report, _) = replay_of(&registry, &project).unwrap();
        assert!(report.identical);
        assert_eq!(
            fs::read_to_string(work.join("replay/1-test.write_output.out")).unwrap(),
            "salut"
        );
    }
    assert!(!work.join("replay-2").exists());
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn no_fresh_replay_folder_names_the_held_file() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let work = journal_with_output(&registry, &project);
    let stale = work.join("replay/1-test.write_output.out");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "périmé").unwrap();
    let journal = fs::read_to_string(work.join(JOURNAL_FILE)).unwrap();
    set_mode(&work.join("replay"), 0o555);
    set_mode(&work, 0o555);

    let outcome = replay_of(&registry, &project);
    set_mode(&work, 0o755);
    set_mode(&work.join("replay"), 0o755);
    let err = outcome.unwrap_err();
    assert_eq!(err.code, "replay.output_locked");
    assert!(
        err.message.contains("replay/1-test.write_output.out"),
        "{}",
        err.message
    );
    assert!(!err.message.contains(&*work.to_string_lossy()));
    assert!(!work.join("replay-2").exists());
    assert_eq!(fs::read_to_string(&stale).unwrap(), "périmé");
    assert_eq!(
        fs::read_to_string(work.join(JOURNAL_FILE)).unwrap(),
        journal
    );
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn a_qgis_journal_replays_identically_into_a_fallback_folder() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    inventory_journal(&project, false);
    let work = dir.join("p.cassini");
    let stale = work.join("replay/3-analysis.buffer.gpkg");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "périmé").unwrap();
    set_mode(&work.join("replay"), 0o555);

    let outcome = Replay::load(&registry, &project).unwrap().run(
        &Router::new(&registry).with_qgis(SessionQgis::new(2)),
        &mut HashSet::new(),
    );
    set_mode(&work.join("replay"), 0o755);
    assert_eq!(
        outcome.unwrap(),
        ReplayReport {
            steps: 4,
            identical: true
        }
    );
    assert_eq!(
        fs::read_to_string(work.join("replay-2/3-analysis.buffer.gpkg")).unwrap(),
        "gpkg-2"
    );
    assert_eq!(fs::read_to_string(&stale).unwrap(), "périmé");
    let _ = fs::remove_dir_all(dir);
}

// ---- Ticket 5.36 : rejeux de suite sans rouvrir le projet ----

/// Faux projet QGIS, qui survit d'une exécution à l'autre comme la session QGIS de
/// l'utilisateur : ses couches visibles, (id, fichier source), dans l'ordre d'ajout.
#[derive(Default)]
struct OpenProject {
    layers: std::cell::RefCell<Vec<(String, String)>>,
}

impl OpenProject {
    fn with_base() -> std::rc::Rc<Self> {
        let project = OpenProject::default();
        project
            .layers
            .borrow_mut()
            .push(("communes_0".into(), "communes.gpkg".into()));
        std::rc::Rc::new(project)
    }
}

/// QGIS de test dont l'export compte les couches visibles du faux projet, sauf celles
/// que `leave_out` nomme, comme le geste de l'extension.
struct ProjectQgis {
    project: std::rc::Rc<OpenProject>,
    generation: u32,
    count: std::cell::Cell<u32>,
    /// `leave_out` reçu par chaque export, dans l'ordre.
    seen: std::rc::Rc<std::cell::RefCell<Vec<Value>>>,
    /// `center` reçu par chaque export, dans l'ordre (`Null` s'il n'y en a pas).
    centers: std::rc::Rc<std::cell::RefCell<Vec<Value>>>,
}

impl ProjectQgis {
    fn new(project: &std::rc::Rc<OpenProject>, generation: u32) -> Self {
        ProjectQgis {
            project: project.clone(),
            generation,
            count: std::cell::Cell::new(0),
            seen: Default::default(),
            centers: Default::default(),
        }
    }
}

impl Executor for ProjectQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        let n = self.count.get() + 1;
        self.count.set(n);
        match id {
            "layer.add" => {
                let layer = format!("pts_g{}_{n}", self.generation);
                self.project
                    .layers
                    .borrow_mut()
                    .push((layer.clone(), "pts.csv".into()));
                Ok(json!({
                    "layer_id": layer, "name": "pts", "source_crs": "EPSG:2154",
                    "project_crs": "EPSG:2154", "reprojected": false
                }))
            }
            // Comme le geste : le raster du noyau entre au projet, nommé du radical du fichier.
            "layer.open_raster" => {
                let path = Path::new(p["path"].as_str().unwrap());
                assert!(path.is_absolute() && path.is_file(), "{}", path.display());
                let file = path.file_name().unwrap().to_string_lossy().into_owned();
                let layer = format!("raster_g{}_{n}", self.generation);
                self.project.layers.borrow_mut().push((layer.clone(), file));
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                Ok(json!({"layer_id": layer, "name": name}))
            }
            // Comme le geste : le GeoJSON du noyau entre au projet, dans le système qu'il déclare.
            "layer.open_vector" => {
                let path = Path::new(p["path"].as_str().unwrap());
                assert!(path.is_absolute() && path.is_file(), "{}", path.display());
                let file = path.file_name().unwrap().to_string_lossy().into_owned();
                let layer = format!("vector_g{}_{n}", self.generation);
                self.project.layers.borrow_mut().push((layer.clone(), file));
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                Ok(json!({"layer_id": layer, "name": name, "crs": "EPSG:2154"}))
            }
            // Comme le geste de `data.convert_raster` : QGIS écrit le GeoTIFF à l'emplacement
            // que le noyau donne (ici, une copie du MNT de test), l'original est intact, et
            // rien n'entre au projet : l'ouverture est l'étape que le noyau dicte ensuite.
            "data.convert_raster" => {
                let source = Path::new(p["raster"].as_str().unwrap());
                assert!(
                    source.is_absolute() && source.is_file(),
                    "{}",
                    source.display()
                );
                let out = p["out"].as_str().unwrap().to_string();
                assert!(out.ends_with(".tif"), "{out}");
                let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../fixtures/mnt-plan-incline-2154.tif");
                fs::copy(fixture, &out).unwrap();
                Ok(json!({"file": out, "epsg": p.get("assign_epsg").cloned()
                    .unwrap_or(json!(2154))}))
            }
            "layout.export_pdf" => {
                self.seen
                    .borrow_mut()
                    .push(p.get("leave_out").cloned().unwrap_or(Value::Null));
                let leave_out: Vec<&str> = p
                    .get("leave_out")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let layers = self.project.layers.borrow();
                let drawn: Vec<&(String, String)> = layers
                    .iter()
                    .filter(|(id, _)| !leave_out.contains(&id.as_str()))
                    .collect();
                let sources: BTreeSet<&str> = drawn.iter().map(|(_, s)| s.as_str()).collect();
                let out = p["out"].as_str().unwrap().to_string();
                fs::write(&out, format!("pdf-{}", self.generation)).unwrap();
                let mut result = json!({
                    "file": out, "page_count": 1, "layer_count": drawn.len(),
                    "sources": sources, "truncated": false
                });
                let center = p.get("center").cloned().unwrap_or(Value::Null);
                self.centers.borrow_mut().push(center.clone());
                // Comme le geste : l'échelle et le centre ne sont rendus que si l'échelle a été
                // demandée (`null` vaut absente). Le centre est celui reçu, sinon celui de la
                // « vue » de QGIS, propre à chaque génération : elle a bougé d'une à l'autre.
                if let Some(scale) = p.get("scale").filter(|v| !v.is_null()) {
                    result["scale"] = scale.clone();
                    result["center"] = if center.is_null() {
                        json!({"x": 650000.0 + f64::from(self.generation) * 1000.0,
                               "y": 6850000.0, "epsg": 2154})
                    } else {
                        center
                    };
                }
                Ok(result)
            }
            other => panic!("commande inattendue : {other}"),
        }
    }
}

/// Session d'origine : ouvre une couche, exporte la carte. Le projet reste ouvert.
fn export_journal(project_file: &Path, project: &std::rc::Rc<OpenProject>) {
    let registry = Registry::standard();
    let qgis = ProjectQgis::new(project, 1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, project_file)
        .unwrap()
        .with_qgis(qgis);
    s.call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    s.call("layout.export_pdf", json!({"title": "Carte"}))
        .unwrap();
    let l = lines(&s);
    assert_eq!(l[1]["result"]["layer_count"], json!(2));
    // Hors rejeu, l'export ne reçoit aucun `leave_out` et le journal n'en garde pas.
    assert_eq!(*seen.borrow(), vec![Value::Null]);
    assert!(l[1]["params"].get("leave_out").is_none());
}

fn replay_in(
    project_file: &Path,
    project: &std::rc::Rc<OpenProject>,
    generation: u32,
    replayed_layers: &mut HashSet<String>,
) -> (Result<ReplayReport, ErrorReport>, Vec<Value>) {
    let registry = Registry::standard();
    let qgis = ProjectQgis::new(project, generation);
    let seen = qgis.seen.clone();
    let outcome = Replay::load(&registry, project_file)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), replayed_layers);
    let seen = seen.borrow().clone();
    (outcome, seen)
}

#[test]
fn two_replays_in_a_row_leave_earlier_layers_out_of_the_map() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    export_journal(&project_file, &project);
    let journal = fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
    let mut replayed = HashSet::new();
    let identical = ReplayReport {
        steps: 2,
        identical: true,
    };

    // Premier rejeu, projet resté ouvert : la couche de la session d'origine est laissée hors.
    let (outcome, seen) = replay_in(&project_file, &project, 2, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["pts_g1_1"])]);
    assert_eq!(replayed, HashSet::from(["pts_g2_1".to_string()]));

    // Second rejeu : celles du premier aussi, triées ; jamais celle de ce rejeu.
    let (outcome, seen) = replay_in(&project_file, &project, 3, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["pts_g1_1", "pts_g2_1"])]);

    // Les couches laissées hors de la carte restent dans le projet ; le journal est intact.
    assert_eq!(project.layers.borrow().len(), 4);
    assert_eq!(
        fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap(),
        journal
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_export_without_scale_and_one_with_scale_replay_identically() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project_file)
        .unwrap()
        .with_qgis(ProjectQgis::new(&project, 1));
    // Journal ancien : un export sans échelle ; puis un export à 1:25 000.
    s.call("layout.export_pdf", json!({"title": "Carte"}))
        .unwrap();
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "scale": 25000}),
    )
    .unwrap();
    let l = lines(&s);
    assert!(l[0]["params"].get("scale").is_none());
    assert!(l[0]["result"].get("scale").is_none());
    assert_eq!(l[1]["params"]["scale"], json!(25000));
    assert_eq!(l[1]["result"]["scale"], json!(25000));
    drop(s);
    let (outcome, _) = replay_in(&project_file, &project, 2, &mut HashSet::new());
    assert_eq!(
        outcome.unwrap(),
        ReplayReport {
            steps: 2,
            identical: true,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_replay_after_the_view_moved_gets_the_logged_center() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    let registry = Registry::standard();
    let qgis = ProjectQgis::new(&project, 1);
    let centers = qgis.centers.clone();
    let mut s = Session::open(&registry, &project_file)
        .unwrap()
        .with_qgis(qgis);
    // `scale: null` vaut absente : ni échelle ni centre au résultat.
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "scale": null}),
    )
    .unwrap();
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "scale": 25000}),
    )
    .unwrap();
    let l = lines(&s);
    assert!(l[0]["result"].get("scale").is_none());
    assert!(l[0]["result"].get("center").is_none());
    let logged = json!({"x": 651000.0, "y": 6850000.0, "epsg": 2154});
    assert_eq!(l[1]["result"]["center"], logged);
    // Le centre est aussi gardé sous la clé réservée `replay` ; sans échelle, pas de clé.
    assert_eq!(l[1]["replay"], json!({"center": logged}));
    assert!(l[0].get("replay").is_none());
    // Hors rejeu, le geste ne reçoit aucun centre, et le journal n'en garde pas en paramètre.
    assert_eq!(*centers.borrow(), vec![Value::Null, Value::Null]);
    assert!(l[1]["params"].get("center").is_none());
    drop(s);
    // Au rejeu, la « vue » de QGIS est ailleurs (génération 2) : le geste reçoit le centre du
    // journal, cadre au même endroit, et le résultat est identique.
    let qgis = ProjectQgis::new(&project, 2);
    let replay_centers = qgis.centers.clone();
    let outcome = Replay::load(&registry, &project_file)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new());
    assert_eq!(
        outcome.unwrap(),
        ReplayReport {
            steps: 2,
            identical: true,
        }
    );
    assert_eq!(*replay_centers.borrow(), vec![Value::Null, logged]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_logged_center_survives_a_summarized_result() {
    // Un vrai export cite plusieurs sources : son résultat dépasse 200 octets et le journal
    // n'en garde que taille et empreinte. Le centre doit survivre, sous la clé `replay`.
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    for i in 0..10 {
        project.layers.borrow_mut().push((
            format!("couche_{i}"),
            format!("donnees/source_numero_{i}.gpkg"),
        ));
    }
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project_file)
        .unwrap()
        .with_qgis(ProjectQgis::new(&project, 1));
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "scale": 25000}),
    )
    .unwrap();
    let l = lines(&s);
    let logged = json!({"x": 651000.0, "y": 6850000.0, "epsg": 2154});
    // Résultat résumé : le centre n'y est plus, il est sous `replay`.
    assert!(l[0]["result"].get("center").is_none(), "{}", l[0]["result"]);
    assert_eq!(l[0]["replay"], json!({"center": logged}));
    drop(s);
    // Rejeu, « vue » ailleurs : le geste reçoit le centre du journal, résultat identique.
    let qgis = ProjectQgis::new(&project, 2);
    let replay_centers = qgis.centers.clone();
    let outcome = Replay::load(&registry, &project_file)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new());
    assert_eq!(
        outcome.unwrap(),
        ReplayReport {
            steps: 1,
            identical: true,
        }
    );
    assert_eq!(*replay_centers.borrow(), vec![logged]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_center_from_the_caller_is_dropped_outside_replay() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    let registry = Registry::standard();
    let qgis = ProjectQgis::new(&project, 1);
    let centers = qgis.centers.clone();
    let mut s = Session::open(&registry, &project_file)
        .unwrap()
        .with_qgis(qgis);
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "scale": 25000,
               "center": {"x": 1.0, "y": 2.0, "epsg": 4326}}),
    )
    .unwrap();
    assert_eq!(*centers.borrow(), vec![Value::Null]);
    let l = lines(&s);
    assert!(l[0]["params"].get("center").is_none());
    assert_eq!(l[0]["result"]["center"]["epsg"], json!(2154));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_replay_with_a_project_reopened_draws_the_same_map() {
    let (dir, project_file, _) = temp_project();
    export_journal(&project_file, &OpenProject::with_base());
    // Projet rouvert sans les couches de la session d'origine.
    let reopened = OpenProject::with_base();
    let (outcome, _) = replay_in(&project_file, &reopened, 2, &mut HashSet::new());
    assert!(outcome.unwrap().identical);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_layer_added_or_removed_by_hand_still_diverges_at_the_export() {
    for change in ["added", "removed"] {
        let (dir, project_file, _) = temp_project();
        let project = OpenProject::with_base();
        export_journal(&project_file, &project);
        let mut replayed = HashSet::new();
        let (first, _) = replay_in(&project_file, &project, 2, &mut replayed);
        assert!(first.unwrap().identical);
        let mut layers = project.layers.borrow_mut();
        if change == "added" {
            layers.push(("main_1".into(), "main.gpkg".into()));
        } else {
            layers.retain(|(id, _)| id != "communes_0");
        }
        drop(layers);
        let (outcome, _) = replay_in(&project_file, &project, 3, &mut replayed);
        let err = outcome.unwrap_err();
        assert_eq!(err.code, "replay.diverged", "{change}");
        assert_eq!(err.step, Some(2), "{change}");
        assert!(err.message.contains("layout.export_pdf"), "{change}");
        // Même un rejeu qui diverge laisse la mémoire de ses couches.
        assert!(replayed.contains("pts_g3_1"), "{change}");
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn a_leave_out_from_the_caller_is_dropped_outside_replay() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    let registry = Registry::standard();
    let qgis = ProjectQgis::new(&project, 1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project_file)
        .unwrap()
        .with_qgis(qgis);
    let result = s
        .call(
            "layout.export_pdf",
            json!({"title": "Carte", "leave_out": ["communes_0"]}),
        )
        .unwrap();
    assert_eq!(result["layer_count"], json!(1));
    assert_eq!(*seen.borrow(), vec![Value::Null]);
    assert!(lines(&s)[0]["params"].get("leave_out").is_none());
    let _ = fs::remove_dir_all(dir);
}

// -- couches intermédiaires déduites du journal (ticket 5.34) --

#[test]
fn a_plan_step_is_written_only_by_a_plan_and_replay_ignores_it() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call_in_plan("test.write_output", json!({"text": "a"}), 1)
        .unwrap();
    s.call_in_plan("test.read_file", json!({"path": "out:1.1"}), 2)
        .unwrap();
    s.call("system.about", json!({})).unwrap();
    let l = lines(&s);
    assert_eq!(l[0]["plan_step"], 1);
    assert_eq!(l[1]["plan_step"], 2);
    assert!(l[2].get("plan_step").is_none());
    let (report, _) = replay_of(&registry, &project).unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 3,
            identical: true
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn intermediate_files_follow_the_rule_of_an_executed_plan() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let ok = |id: &str, plan_step: Option<u64>, params: Value, layer: Option<&str>| {
        let mut row = json!({"id": id, "params": params, "result": {}});
        if let Some(n) = plan_step {
            row["plan_step"] = json!(n);
        }
        if let Some(layer) = layer {
            row["layer_id"] = json!(layer);
        }
        row
    };
    let rows = [
        // Plan 1 : sélection, tampon, intersection, intersection, PDF.
        ok(
            "analysis.select_by_attribute",
            Some(1),
            json!({"layer_id": "plu"}),
            Some("r1"),
        ),
        ok(
            "analysis.buffer",
            Some(2),
            json!({"layer_id": "r1"}),
            Some("r2"),
        ),
        ok(
            "analysis.intersect",
            Some(3),
            json!({"input_layer_id": "r2", "overlay_layer_id": "gares", "n": 3}),
            Some("r3"),
        ),
        // Une sortie entière (`step:k` nu) consomme aussi la couche qu'elle porte.
        ok(
            "analysis.intersect",
            Some(4),
            json!({"input_layer_id": {"layer_id": "r3", "feature_count": 2}}),
            Some("r4"),
        ),
        // Une étape sans couche (PDF) ne consomme rien.
        ok(
            "layout.export_pdf",
            Some(5),
            json!({"layer_id": "r4"}),
            None,
        ),
        // Ligne illisible : ignorée.
        json!("pas un objet"),
        // Plan 2 (phrase suivante) : reprend r4, puis s'arrête sur une erreur.
        ok(
            "analysis.buffer",
            Some(1),
            json!({"layer_id": "r4"}),
            Some("r7"),
        ),
        json!({"id": "analysis.intersect", "plan_step": 2,
            "params": {"input_layer_id": "r7"}, "error": "qgis.failed"}),
        // Plan 3 : `layer.add` puis une étape qui reprend la couche ouverte.
        ok("layer.add", Some(1), json!({"path": "a.gpkg"}), Some("a1")),
        ok(
            "analysis.buffer",
            Some(2),
            json!({"layer_id": "a1"}),
            Some("r10"),
        ),
        // Journal ancien, sans `plan_step` : rien n'est caché.
        ok(
            "analysis.buffer",
            None,
            json!({"layer_id": "plu"}),
            Some("x1"),
        ),
        ok(
            "analysis.intersect",
            None,
            json!({"input_layer_id": "x1"}),
            Some("x2"),
        ),
    ];
    hand_journal(&project, &rows);
    let journal = dir.join("p.cassini").join(JOURNAL_FILE);
    let mut got: Vec<String> = intermediate_files(&registry, &journal)
        .into_iter()
        .collect();
    got.sort();
    assert_eq!(
        got,
        [
            "1-analysis.select_by_attribute.gpkg",
            "2-analysis.buffer.gpkg",
            "3-analysis.intersect.gpkg"
        ]
    );
    // Une ligne au JSON invalide au milieu d'un plan ne casse rien d'autre.
    let mut text = fs::read_to_string(&journal).unwrap();
    text = text.replacen("\"pas un objet\"", "{pas du json", 1);
    fs::write(&journal, text).unwrap();
    assert_eq!(intermediate_files(&registry, &journal).len(), 3);
    // Journal absent : rien.
    assert!(intermediate_files(&registry, &dir.join("absent.jsonl")).is_empty());
    let _ = fs::remove_dir_all(dir);
}

// ---- Ticket 4.9 : l'ouverture d'un raster du noyau est une étape dictée et journalisée ----

/// Copie le MNT de test sous `dem/plan.tif` dans le dossier du projet.
fn with_dem(dir: &Path) {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-plan-incline-2154.tif");
    fs::create_dir_all(dir.join("dem")).unwrap();
    fs::copy(fixture, dir.join("dem/plan.tif")).unwrap();
}

fn slope_params() -> Value {
    json!({"dem": "dem/plan.tif", "unit": "degrees"})
}

#[test]
fn a_raster_marker_without_its_step_is_refused_and_not_journaled() {
    // Ticket 11.5 : `dem` est un `RasterPath` ; un repère hors du journal reste refusé.
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    let err = s
        .call(
            "terrain.slope",
            json!({"dem": "out:9.1", "unit": "degrees"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "replay.bad_reference");
    assert!(lines(&s).is_empty());
    // Sur la sortie d'une pente déjà journalisée, le repère est gardé tel quel.
    s.call("terrain.slope", slope_params()).unwrap();
    s.call(
        "terrain.slope",
        json!({"dem": "out:1.1", "unit": "percent"}),
    )
    .unwrap();
    let l = lines(&s);
    assert_eq!(l[1]["params"]["dem"], json!("out:1.1"));
    assert_eq!(l[1]["inputs"], json!([]));
    assert_eq!(l[0]["params"]["dem"], json!("dem/plan.tif"));
    assert_eq!(l[0]["inputs"].as_array().unwrap().len(), 1);
}

/// QGIS de test qui note chaque ordre reçu et ouvre un raster (ou échoue à l'ouvrir).
struct RasterQgis {
    calls: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
    unreadable: bool,
}

impl RasterQgis {
    fn new(unreadable: bool) -> Self {
        RasterQgis {
            calls: Default::default(),
            unreadable,
        }
    }
}

impl Executor for RasterQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.calls.borrow_mut().push((id.to_string(), p.clone()));
        assert_eq!(id, "layer.open_raster");
        if self.unreadable {
            return Err(RegistryError::Delegated {
                code: "layer.invalid_source".into(),
                message: "Le raster « 1-terrain.slope.tif » n'est pas lisible.".into(),
            });
        }
        let n = self.calls.borrow().len();
        let stem = Path::new(p["path"].as_str().unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        Ok(json!({"layer_id": format!("r{n}"), "name": stem}))
    }
}

#[test]
fn a_core_raster_is_followed_by_a_dictated_opening_in_the_three_flows() {
    let (dir, project, token) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let qgis = RasterQgis::new(false);
    let calls = qgis.calls.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_raster_opening(true);
    // `command.run`, appel d'une IA externe, étape 2 d'un plan.
    let by_hand = s.call("terrain.slope", slope_params()).unwrap();
    s.call_from("terrain.slope", slope_params(), Some("external"))
        .unwrap();
    s.call_in_plan("terrain.slope", slope_params(), 2).unwrap();
    // Le résultat rendu reste celui de la pente.
    assert!(by_hand["file"]
        .as_str()
        .unwrap()
        .ends_with("1-terrain.slope.tif"));
    let l = lines(&s);
    assert_eq!(l.len(), 6);
    for (k, origin, plan_step) in [
        (0, Value::Null, Value::Null),
        (2, json!("external"), Value::Null),
        (4, Value::Null, json!(2)),
    ] {
        let (slope, open) = (&l[k], &l[k + 1]);
        assert_eq!(slope["id"], "terrain.slope");
        assert_eq!(open["id"], "layer.open_raster");
        assert_eq!(open["step"], json!(k + 2));
        assert_eq!(open["params"], json!({"path": format!("out:{}.1", k + 1)}));
        assert_eq!(open["inputs"], json!([]));
        assert_eq!(open["layer_id"], json!(format!("r{}", k / 2 + 1)));
        assert_eq!(
            open["result"]["name"],
            json!(format!("{}-terrain.slope", k + 1))
        );
        for line in [slope, open] {
            assert_eq!(line.get("origin").cloned().unwrap_or(Value::Null), origin);
            assert_eq!(
                line.get("plan_step").cloned().unwrap_or(Value::Null),
                plan_step
            );
        }
    }
    // L'extension reçoit le chemin absolu du raster écrit, qui existe ; le journal, aucun.
    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    for (i, (_, params)) in calls.iter().enumerate() {
        let path = Path::new(params["path"].as_str().unwrap());
        assert_eq!(
            path,
            s.work_dir()
                .join(format!("{}-terrain.slope.tif", 2 * i + 1))
        );
        assert!(path.is_file());
    }
    assert!(!fs::read_to_string(s.journal_path())
        .unwrap()
        .contains(&token));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_line_of_sight_or_a_session_without_qgis_dictates_nothing() {
    let (dir, project, _) = temp_project();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-colline-2154.tif"),
        dir.join("colline.tif"),
    )
    .unwrap();
    with_dem(&dir);
    let registry = Registry::standard();
    let qgis = RasterQgis::new(false);
    let calls = qgis.calls.clone();
    {
        // Ligne de vue : aucun fichier écrit, rien n'est dicté.
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(qgis)
            .with_raster_opening(true);
        s.call(
            "terrain.line_of_sight",
            json!({"dem": "colline.tif",
                "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
                "target": {"x": 652130, "y": 6850250, "epsg": 2154},
                "observer_height": {"value": 2, "unit": "m"},
                "target_height": {"value": 2, "unit": "m"}}),
        )
        .unwrap();
        assert_eq!(lines(&s).len(), 1);
    }
    assert!(calls.borrow().is_empty());
    // Sans QGIS (CLI `run`, MCP autonome) ou sans le geste annoncé : la pente seule.
    let mut s = Session::open(&registry, &project).unwrap();
    s.call("terrain.slope", slope_params()).unwrap();
    let ids: Vec<Value> = lines(&s).iter().map(|l| l["id"].clone()).collect();
    assert_eq!(
        ids,
        [json!("terrain.line_of_sight"), json!("terrain.slope")]
    );
    drop(s);
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RasterQgis::new(false))
        .with_raster_opening(false);
    s.call("terrain.slope", slope_params()).unwrap();
    assert_eq!(lines(&s).len(), 3);
    assert_eq!(lines(&s)[2]["id"], "terrain.slope");
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_unreadable_raster_is_a_named_error_journaled_at_its_step() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RasterQgis::new(true))
        .with_raster_opening(true);
    let err = s.call("terrain.slope", slope_params()).unwrap_err();
    assert_eq!(err.code, "layer.invalid_source");
    assert!(
        err.message.contains("1-terrain.slope.tif"),
        "{}",
        err.message
    );
    let l = lines(&s);
    assert_eq!(l.len(), 2);
    assert!(l[0]["result"].is_object());
    assert_eq!(l[1]["id"], "layer.open_raster");
    assert_eq!(l[1]["error"], "layer.invalid_source");
    assert!(l[1].get("layer_id").is_none());
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Session d'origine de la reproduction du ticket 4.9 : une pente, puis l'export. Le raster
/// est ouvert par l'étape dictée ; l'export le montre.
fn raster_export_journal(project_file: &Path, project: &std::rc::Rc<OpenProject>) {
    let registry = Registry::standard();
    let mut s = Session::open(&registry, project_file)
        .unwrap()
        .with_qgis(ProjectQgis::new(project, 1))
        .with_raster_opening(true);
    s.call("terrain.slope", slope_params()).unwrap();
    s.call("layout.export_pdf", json!({"title": "Pente"}))
        .unwrap();
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        ["terrain.slope", "layer.open_raster", "layout.export_pdf"]
    );
    assert_eq!(l[1]["layer_id"], "raster_g1_1");
    assert_eq!(l[2]["result"]["layer_count"], json!(2));
    assert_eq!(
        l[2]["result"]["sources"],
        json!(["1-terrain.slope.tif", "communes.gpkg"])
    );
}

#[test]
fn a_raster_then_an_export_replays_identically_twice_in_the_same_session() {
    let (dir, project_file, _) = temp_project();
    with_dem(&dir);
    let project = OpenProject::with_base();
    raster_export_journal(&project_file, &project);
    let identical = ReplayReport {
        steps: 3,
        identical: true,
    };
    let mut replayed = HashSet::new();
    // Premier rejeu, sans rouvrir : le raster d'origine est laissé hors de la carte.
    let (outcome, seen) = replay_in(&project_file, &project, 2, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["raster_g1_1"])]);
    assert_eq!(replayed, HashSet::from(["raster_g2_1".to_string()]));
    // Second rejeu : celui du premier aussi.
    let (outcome, seen) = replay_in(&project_file, &project, 3, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["raster_g1_1", "raster_g2_1"])]);
    // Aucune couche n'est retirée du projet.
    assert_eq!(project.layers.borrow().len(), 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_raster_then_an_export_replays_identically_in_a_new_session() {
    let (dir, project_file, _) = temp_project();
    with_dem(&dir);
    raster_export_journal(&project_file, &OpenProject::with_base());
    // Projet rouvert sans le raster : le rejeu recalcule la pente et la rouvre avant l'export.
    let reopened = OpenProject::with_base();
    let (outcome, _) = replay_in(&project_file, &reopened, 2, &mut HashSet::new());
    assert!(outcome.unwrap().identical);
    let layers = reopened.layers.borrow();
    assert_eq!(layers.len(), 2);
    assert_eq!(
        layers[1],
        ("raster_g2_1".to_string(), "1-terrain.slope.tif".to_string())
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_opened_raster_consumed_by_a_later_plan_step_is_intermediate() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let rows = [
        json!({"id": "terrain.slope", "plan_step": 1,
            "params": {"dem": "dem/plan.tif", "unit": "degrees", "out": "1-terrain.slope.tif"},
            "result": {}}),
        json!({"id": "layer.open_raster", "plan_step": 1, "params": {"path": "out:1.1"},
            "layer_id": "R", "result": {}}),
        json!({"id": "analysis.zonal_statistics", "plan_step": 2,
            "params": {"layer_id": "R", "points_layer_id": "p"}, "layer_id": "S",
            "result": {}}),
    ];
    hand_journal(&project, &rows);
    let journal = dir.join("p.cassini").join(JOURNAL_FILE);
    let got: Vec<String> = intermediate_files(&registry, &journal)
        .into_iter()
        .collect();
    assert_eq!(got, ["1-terrain.slope.tif"]);
    // Ouvert sans être repris : rien n'est caché.
    hand_journal(&project, &rows[..2]);
    assert!(intermediate_files(&registry, &journal).is_empty());
    // Seule l'ouverture dictée lit un repère `out:<k>.<n>` dans ses paramètres.
    let mut other = rows.clone();
    other[1]["id"] = json!("layer.add");
    hand_journal(&project, &other);
    assert!(intermediate_files(&registry, &journal).is_empty());
    let _ = fs::remove_dir_all(dir);
}

// ---- Ticket 11.8 : l'ouverture d'un GeoJSON du noyau est une étape dictée et journalisée ----

/// Commande du noyau de test qui écrit, par l'écrivain unique, les points de la fixture
/// `points-2154.geojson` dans la sortie que le noyau lui donne.
struct TestWritePoints;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct WritePointsParams {
    #[serde(default)]
    out: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct WritePointsOutput {
    feature_count: usize,
}

impl crate::registry::Command for TestWritePoints {
    type Params = WritePointsParams;
    type Output = WritePointsOutput;

    fn spec() -> crate::registry::CommandSpec {
        crate::registry::CommandSpec::new::<WritePointsParams, WritePointsOutput>(
            "test.write_points",
            "Écrit trois points en GeoJSON (tests seulement).",
            "Writes three points as GeoJSON (tests only).",
        )
        .runs_in(crate::registry::RunsIn::Core)
        .cancellable(false)
        .output_params(&["out"])
        .output_ext(".geojson")
        .tests(&["a_core_geojson_is_followed_by_a_dictated_opening_in_the_three_flows"])
    }

    fn run(
        &self,
        params: WritePointsParams,
    ) -> Result<WritePointsOutput, crate::registry::CommandFailure> {
        let points = crate::geojson::fixture_points();
        crate::geojson::write(Path::new(&params.out), 2154, &points).map_err(|e| {
            crate::registry::CommandFailure {
                code: e.code.to_string(),
                message: e.message,
            }
        })?;
        Ok(WritePointsOutput {
            feature_count: points.len(),
        })
    }
}

/// Le registre standard, plus la commande qui écrit un GeoJSON.
fn vector_registry() -> Registry {
    let mut registry = Registry::standard();
    registry.register(TestWritePoints);
    registry
}

fn points_fixture() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/points-2154.geojson")).unwrap()
}

/// QGIS de test qui note chaque ordre reçu et ouvre un GeoJSON (ou échoue à l'ouvrir).
struct VectorQgis {
    calls: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
    unreadable: bool,
}

impl Executor for VectorQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.calls.borrow_mut().push((id.to_string(), p.clone()));
        assert_eq!(id, "layer.open_vector");
        if self.unreadable {
            return Err(RegistryError::Delegated {
                code: "layer.invalid_source".into(),
                message: "Le fichier « 1-test.write_points.geojson » n'est pas lisible.".into(),
            });
        }
        let n = self.calls.borrow().len();
        let stem = Path::new(p["path"].as_str().unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        Ok(json!({"layer_id": format!("v{n}"), "name": stem, "crs": "EPSG:2154"}))
    }
}

#[test]
fn a_core_geojson_is_followed_by_a_dictated_opening_in_the_three_flows() {
    let (dir, project, token) = temp_project();
    let registry = vector_registry();
    let qgis = VectorQgis {
        calls: Default::default(),
        unreadable: false,
    };
    let calls = qgis.calls.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_vector_opening(true);
    let by_hand = s.call("test.write_points", json!({})).unwrap();
    s.call_from("test.write_points", json!({}), Some("external"))
        .unwrap();
    s.call_in_plan("test.write_points", json!({}), 2).unwrap();
    // Le résultat rendu reste celui de l'étape demandée.
    assert_eq!(by_hand, json!({"feature_count": 3}));
    let l = lines(&s);
    assert_eq!(l.len(), 6);
    for (k, origin, plan_step) in [
        (0, Value::Null, Value::Null),
        (2, json!("external"), Value::Null),
        (4, Value::Null, json!(2)),
    ] {
        let (write, open) = (&l[k], &l[k + 1]);
        assert_eq!(write["id"], "test.write_points");
        assert_eq!(
            write["params"]["out"],
            json!(format!("{}-test.write_points.geojson", k + 1))
        );
        assert_eq!(open["id"], "layer.open_vector");
        assert_eq!(open["step"], json!(k + 2));
        assert_eq!(open["params"], json!({"path": format!("out:{}.1", k + 1)}));
        assert_eq!(open["inputs"], json!([]));
        assert_eq!(open["layer_id"], json!(format!("v{}", k / 2 + 1)));
        assert_eq!(open["result"]["crs"], json!("EPSG:2154"));
        assert_eq!(
            open["result"]["name"],
            json!(format!("{}-test.write_points", k + 1))
        );
        for line in [write, open] {
            assert_eq!(line.get("origin").cloned().unwrap_or(Value::Null), origin);
            assert_eq!(
                line.get("plan_step").cloned().unwrap_or(Value::Null),
                plan_step
            );
        }
    }
    // L'extension reçoit le chemin absolu du GeoJSON écrit, égal à la fixture ; le journal, aucun.
    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    for (i, (_, params)) in calls.iter().enumerate() {
        let path = Path::new(params["path"].as_str().unwrap());
        assert_eq!(
            path,
            s.work_dir()
                .join(format!("{}-test.write_points.geojson", 2 * i + 1))
        );
        assert_eq!(fs::read(path).unwrap(), points_fixture());
    }
    assert!(!fs::read_to_string(s.journal_path())
        .unwrap()
        .contains(&token));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_geojson_without_the_announced_gesture_dictates_nothing() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = vector_registry();
    // Sans QGIS (CLI `run`, MCP autonome) : l'écriture seule.
    {
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.write_points", json!({})).unwrap();
        assert_eq!(lines(&s).len(), 1);
    }
    // QGIS branché, seule l'ouverture raster annoncée : rien n'est dicté pour un GeoJSON.
    let qgis = VectorQgis {
        calls: Default::default(),
        unreadable: false,
    };
    let calls = qgis.calls.clone();
    {
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(qgis)
            .with_raster_opening(true)
            .with_vector_opening(false);
        s.call("test.write_points", json!({})).unwrap();
        assert_eq!(lines(&s).len(), 2);
    }
    assert!(calls.borrow().is_empty());
    // L'ouverture vecteur annoncée seule ne dicte rien après un raster.
    let qgis = VectorQgis {
        calls: Default::default(),
        unreadable: false,
    };
    let calls = qgis.calls.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_vector_opening(true);
    s.call("terrain.slope", slope_params()).unwrap();
    let ids: Vec<Value> = lines(&s).iter().map(|l| l["id"].clone()).collect();
    assert_eq!(
        ids,
        [
            json!("test.write_points"),
            json!("test.write_points"),
            json!("terrain.slope")
        ]
    );
    assert!(calls.borrow().is_empty());
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn both_openings_announced_dictate_the_right_one_after_each_output() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = vector_registry();
    let opened = OpenProject::with_base();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(ProjectQgis::new(&opened, 1))
        .with_raster_opening(true)
        .with_vector_opening(true);
    s.call("terrain.slope", slope_params()).unwrap();
    s.call("test.write_points", json!({})).unwrap();
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "terrain.slope",
            "layer.open_raster",
            "test.write_points",
            "layer.open_vector"
        ]
    );
    assert_eq!(l[1]["params"], json!({"path": "out:1.1"}));
    assert_eq!(l[3]["params"], json!({"path": "out:3.1"}));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_unreadable_geojson_is_a_named_error_journaled_at_its_step() {
    let (dir, project, _) = temp_project();
    let registry = vector_registry();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(VectorQgis {
            calls: Default::default(),
            unreadable: true,
        })
        .with_vector_opening(true);
    let err = s.call("test.write_points", json!({})).unwrap_err();
    assert_eq!(err.code, "layer.invalid_source");
    assert!(
        err.message.contains("1-test.write_points.geojson"),
        "{}",
        err.message
    );
    let l = lines(&s);
    assert_eq!(l.len(), 2);
    assert!(l[0]["result"].is_object());
    assert_eq!(l[1]["id"], "layer.open_vector");
    assert_eq!(l[1]["error"], "layer.invalid_source");
    assert!(l[1].get("layer_id").is_none());
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Session d'origine : des points écrits par le noyau, puis l'export ; le GeoJSON est ouvert
/// par l'étape dictée et l'export le montre.
fn vector_export_journal(project_file: &Path, project: &std::rc::Rc<OpenProject>) {
    let registry = vector_registry();
    let mut s = Session::open(&registry, project_file)
        .unwrap()
        .with_qgis(ProjectQgis::new(project, 1))
        .with_raster_opening(true)
        .with_vector_opening(true);
    s.call("test.write_points", json!({})).unwrap();
    s.call("layout.export_pdf", json!({"title": "Points"}))
        .unwrap();
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "test.write_points",
            "layer.open_vector",
            "layout.export_pdf"
        ]
    );
    assert_eq!(l[1]["layer_id"], "vector_g1_1");
    assert_eq!(l[2]["result"]["layer_count"], json!(2));
    assert_eq!(
        l[2]["result"]["sources"],
        json!(["1-test.write_points.geojson", "communes.gpkg"])
    );
}

fn replay_vector_in(
    project_file: &Path,
    project: &std::rc::Rc<OpenProject>,
    generation: u32,
    replayed_layers: &mut HashSet<String>,
) -> (Result<ReplayReport, ErrorReport>, Vec<Value>) {
    let registry = vector_registry();
    let qgis = ProjectQgis::new(project, generation);
    let seen = qgis.seen.clone();
    let outcome = Replay::load(&registry, project_file)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), replayed_layers);
    let seen = seen.borrow().clone();
    (outcome, seen)
}

#[test]
fn a_geojson_then_an_export_replays_identically_twice_in_the_same_session() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    vector_export_journal(&project_file, &project);
    let identical = ReplayReport {
        steps: 3,
        identical: true,
    };
    let mut replayed = HashSet::new();
    // Premier rejeu, sans rouvrir : la couche d'origine est laissée hors de la carte.
    let (outcome, seen) = replay_vector_in(&project_file, &project, 2, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["vector_g1_1"])]);
    assert_eq!(replayed, HashSet::from(["vector_g2_1".to_string()]));
    // Second rejeu : celle du premier aussi.
    let (outcome, seen) = replay_vector_in(&project_file, &project, 3, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    assert_eq!(seen, vec![json!(["vector_g1_1", "vector_g2_1"])]);
    assert_eq!(project.layers.borrow().len(), 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_geojson_then_an_export_replays_identically_in_a_new_session() {
    let (dir, project_file, _) = temp_project();
    vector_export_journal(&project_file, &OpenProject::with_base());
    // Projet rouvert sans la couche : le rejeu réécrit le GeoJSON et le rouvre avant l'export.
    let reopened = OpenProject::with_base();
    let (outcome, _) = replay_vector_in(&project_file, &reopened, 2, &mut HashSet::new());
    assert!(outcome.unwrap().identical);
    let layers = reopened.layers.borrow();
    assert_eq!(layers.len(), 2);
    assert_eq!(
        layers[1],
        (
            "vector_g2_1".to_string(),
            "1-test.write_points.geojson".to_string()
        )
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_geojson_whose_bytes_differ_does_not_replay_identically() {
    // Le rejeu compare le GeoJSON octet pour octet, comme toute sortie du noyau.
    let (dir, project_file, _) = temp_project();
    vector_export_journal(&project_file, &OpenProject::with_base());
    let original = dir.join("p.cassini").join("1-test.write_points.geojson");
    let mut bytes = fs::read(&original).unwrap();
    bytes.insert(bytes.len() - 1, b' ');
    fs::write(&original, bytes).unwrap();
    let (outcome, _) = replay_vector_in(
        &project_file,
        &OpenProject::with_base(),
        2,
        &mut HashSet::new(),
    );
    let err = outcome.unwrap_err();
    assert!(err.message.contains("mêmes octets"), "{}", err.message);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_opened_geojson_consumed_by_a_later_plan_step_is_intermediate() {
    let (dir, project, _) = temp_project();
    let registry = vector_registry();
    let rows = [
        json!({"id": "test.write_points", "plan_step": 1,
            "params": {"out": "1-test.write_points.geojson"}, "result": {}}),
        json!({"id": "layer.open_vector", "plan_step": 1, "params": {"path": "out:1.1"},
            "layer_id": "V", "result": {}}),
        json!({"id": "analysis.buffer", "plan_step": 2,
            "params": {"layer_id": "V", "distance": {"value": 10, "unit": "m"}},
            "layer_id": "B", "result": {}}),
    ];
    hand_journal(&project, &rows);
    let journal = dir.join("p.cassini").join(JOURNAL_FILE);
    let got: Vec<String> = intermediate_files(&registry, &journal)
        .into_iter()
        .collect();
    assert_eq!(got, ["1-test.write_points.geojson"]);
    hand_journal(&project, &rows[..2]);
    assert!(intermediate_files(&registry, &journal).is_empty());
    let _ = fs::remove_dir_all(dir);
}

/// Un chemin rangé dans les éléments d'une liste (`criteria.raster`) est journalisé relatif au
/// projet, exécuté absolu, listé dans `inputs`, et le rejeu le rend à l'identique (ticket 11.9).
#[test]
fn a_path_inside_a_list_of_objects_is_journaled_relative_and_replays_identically() {
    use crate::dem::Dem;
    let (dir, project, token) = temp_project();
    fs::create_dir_all(dir.join("donnees")).unwrap();
    let grid = |values: Vec<f32>| Dem {
        width: 2,
        height: 2,
        origin_x: 652000.0,
        origin_y: 6850020.0,
        cell_size_x: 10.0,
        cell_size_y: 10.0,
        epsg: 2154,
        nodata: Some(-9999.0),
        values,
    };
    grid(vec![1.0, 2.0, 3.0, 4.0])
        .write_geotiff(&dir.join("a.tif"))
        .unwrap();
    grid(vec![4.0, 3.0, 2.0, 1.0])
        .write_geotiff(&dir.join("donnees/b.tif"))
        .unwrap();
    grid(vec![0.0, 0.0, 0.0, 1.0])
        .write_geotiff(&dir.join("m.tif"))
        .unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    // Le second critère est donné en absolu : journalisé en relatif aussi.
    let absolute_b = dir.join("donnees/b.tif").to_string_lossy().into_owned();
    let out = s
        .call(
            "raster.weighted_overlay",
            json!({"criteria": [
                {"raster": "a.tif", "weight": 1.0},
                {"raster": absolute_b, "weight": 3.0}
            ], "exclusions": ["m.tif"]}),
        )
        .unwrap();
    assert_eq!(out["nodata_cells"], 1);
    assert_eq!(out["min"], json!(2.25));
    assert_eq!(out["max"], json!(3.25));
    let raw = fs::read_to_string(s.journal_path()).unwrap();
    assert!(
        !raw.contains(&token),
        "chemin absolu dans le journal : {raw}"
    );
    let l = lines(&s);
    assert_eq!(
        l[0]["params"]["criteria"],
        json!([
            {"raster": "a.tif", "weight": 1.0},
            {"raster": "donnees/b.tif", "weight": 3.0}
        ])
    );
    assert_eq!(l[0]["params"]["exclusions"], json!(["m.tif"]));
    let inputs: Vec<&str> = l[0]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["path"].as_str().unwrap())
        .collect();
    assert_eq!(inputs, ["a.tif", "donnees/b.tif", "m.tif"]);
    assert!(s.work_dir().join("1-raster.weighted_overlay.tif").exists());

    let (report, warnings) = replay_of(&registry, &project).unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 1,
            identical: true
        }
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        fs::read(s.work_dir().join("1-raster.weighted_overlay.tif")).unwrap(),
        fs::read(s.work_dir().join("replay/1-raster.weighted_overlay.tif")).unwrap()
    );

    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Plan pente → reclassement → chemin de moindre coût, sans QGIS, chaque étape citant la sortie
/// de la précédente par son repère ; le rejeu le rend à l'identique (ticket 11.10).
#[test]
fn a_slope_reclassify_least_cost_path_plan_replays_identically() {
    let (dir, project, token) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project).unwrap();
    s.call_in_plan("terrain.slope", slope_params(), 1).unwrap();
    let classes = s
        .call_in_plan(
            "raster.reclassify",
            json!({"raster": "out:1.1", "classes": [
                {"max": 5.0, "value": 1.0},
                {"min": 5.0, "value": 4.0}
            ]}),
            2,
        )
        .unwrap();
    assert!(classes["nodata_cells"].as_u64().unwrap() > 0);
    let found = s
        .call_in_plan(
            "terrain.least_cost_path",
            json!({"cost": "out:2.1",
                "start": {"x": 652015, "y": 6850285, "epsg": 2154},
                "end": {"x": 652385, "y": 6850015, "epsg": 2154}}),
            3,
        )
        .unwrap();
    assert_eq!(found["feature_count"], 1);
    assert_eq!(found["length"]["unit"], "m");
    assert!(found["total_cost"].as_f64().unwrap() > 0.0);
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "terrain.slope",
            "raster.reclassify",
            "terrain.least_cost_path"
        ]
    );
    assert_eq!(l[1]["params"]["raster"], json!("out:1.1"));
    assert_eq!(l[2]["params"]["cost"], json!("out:2.1"));
    assert_eq!(l[2]["inputs"], json!([]));
    assert!(!fs::read_to_string(s.journal_path())
        .unwrap()
        .contains(&token));
    let path = s.work_dir().join("3-terrain.least_cost_path.geojson");
    assert!(path.is_file());

    let (report, warnings) = replay_of(&registry, &project).unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 3,
            identical: true
        }
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        fs::read(&path).unwrap(),
        fs::read(
            s.work_dir()
                .join("replay/3-terrain.least_cost_path.geojson")
        )
        .unwrap()
    );
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

// --- Mise en forme topographique (entrée 11.20) : dictée, journal propre, rejeu. ---

/// QGIS de test de la mise en forme : rend l'inventaire synthétique de Villeneuve, plus les
/// couches ajoutées par cette « session » (`generation`), et note chaque ordre reçu.
struct StyleQgis {
    generation: u32,
    created: std::cell::RefCell<Vec<String>>,
    seen: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
    /// Refuse le n-ième `layer.set_style` (à partir de 1).
    refuse_style: Option<usize>,
    /// Légende remaniée, comme après un `layer.arrange` déjà joué dans la même session QGIS :
    /// couches du projet en ordre inverse, couches posées en haut.
    rearranged: bool,
    /// Couche du projet absente de l'inventaire : un vrai changement du projet.
    dropped: Option<&'static str>,
    /// Couche ajoutée au projet par l'utilisateur, absente de la légende d'origine.
    extra: bool,
    /// Couche du projet gardée sous le même id, mais renommée (nom et fichier) : son thème change.
    renamed: Option<&'static str>,
}

impl StyleQgis {
    fn new(generation: u32) -> Self {
        StyleQgis {
            generation,
            created: Default::default(),
            seen: Default::default(),
            refuse_style: None,
            rearranged: false,
            dropped: None,
            extra: false,
            renamed: None,
        }
    }

    fn count(&self, id: &str) -> usize {
        self.seen.borrow().iter().filter(|(c, _)| c == id).count()
    }
}

impl Executor for StyleQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.seen.borrow_mut().push((id.to_string(), p.clone()));
        match id {
            "project.inventory" => {
                let mut inv = crate::commands::map_topographic_style::fixtures::villeneuve();
                // Les couches posées par le journal, de cette session et des précédentes.
                let layers = inv["layers"].as_array_mut().unwrap();
                if let Some(gone) = self.dropped {
                    layers.retain(|l| l["layer_id"] != gone);
                }
                if let Some(id) = self.renamed {
                    for l in layers.iter_mut().filter(|l| l["layer_id"] == id) {
                        l["name"] = json!("couche sans thème");
                        l["source"] = json!("couche_sans_theme.gpkg");
                    }
                }
                if self.extra {
                    layers.push(
                        json!({"layer_id": "ecoles_9", "name": "ecoles", "kind": "vector",
                        "geometry": "Point", "crs": "EPSG:2154", "fields": []}),
                    );
                }
                if self.rearranged {
                    layers.reverse();
                }
                for g in 1..=self.generation {
                    let id = format!("ajout_g{g}");
                    if g < self.generation || self.created.borrow().contains(&id) {
                        let layer = json!({"layer_id": id, "name": "points", "kind": "vector",
                            "geometry": "Point", "crs": "EPSG:2154", "fields": []});
                        if self.rearranged {
                            layers.insert(0, layer);
                        } else {
                            layers.push(layer);
                        }
                    }
                }
                Ok(inv)
            }
            "layer.add" => {
                let id = format!("ajout_g{}", self.generation);
                self.created.borrow_mut().push(id.clone());
                Ok(json!({"layer_id": id, "name": "points"}))
            }
            "layer.set_style" => {
                if self.refuse_style == Some(self.count("layer.set_style")) {
                    return Err(RegistryError::Delegated {
                        code: "style.field_missing".into(),
                        message: "champ absent".into(),
                    });
                }
                Ok(json!({"layer_id": p["layer_id"]}))
            }
            "layer.arrange" => {
                let groups = p["groups"].as_array().unwrap();
                let layers: usize = groups
                    .iter()
                    .map(|g| g["layers"].as_array().unwrap().len())
                    .sum();
                Ok(json!({"group_count": groups.len(), "layer_count": layers}))
            }
            other => panic!("commande inattendue : {other}"),
        }
    }
}

/// Les trois gestes de la mise en forme, tels que l'extension les annonce.
fn style_gestures() -> Vec<String> {
    ["project.inventory", "layer.set_style", "layer.arrange"]
        .map(String::from)
        .to_vec()
}

#[test]
fn the_topographic_style_dictates_its_orders_and_keeps_the_inventory_out_of_the_journal() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let qgis = StyleQgis::new(1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_gestures(style_gestures());
    // L'inventaire de l'appelant est ignoré : seul celui que le noyau lit compte.
    let out = s
        .call(
            "map.topographic_style",
            json!({"inventory": {"layers": []}}),
        )
        .unwrap();
    let mut keys: Vec<&String> = out.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["groups", "styled", "unchanged"]);
    assert_eq!(out["styled"].as_array().unwrap().len(), 5);

    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|x| x["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "map.topographic_style",
            "layer.set_style",
            "layer.set_style",
            "layer.set_style",
            "layer.set_style",
            "layer.set_style",
            "layer.arrange"
        ]
    );
    assert_eq!(l[0]["params"], json!({}));
    // Aucune ligne ne prend une couche existante pour une couche posée.
    assert!(l.iter().all(|x| x.get("layer_id").is_none()));
    let raw = fs::read_to_string(s.journal_path()).unwrap();
    for word in [
        "inventory",
        "RGEALTI",
        "project_crs",
        "orders",
        "contenance",
    ] {
        assert!(!raw.contains(word), "« {word} » dans le journal : {raw}");
    }
    assert_eq!(l[6]["result"], json!({"group_count": 4, "layer_count": 5}));
    // QGIS a reçu l'inventaire une fois, puis les ordres dans l'ordre du journal.
    let order: Vec<String> = seen.borrow().iter().map(|(c, _)| c.clone()).collect();
    assert_eq!(order[0], "project.inventory");
    assert_eq!(&order[1..], &ids[1..]);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_refused_style_order_stops_the_sequence() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut qgis = StyleQgis::new(1);
    qgis.refuse_style = Some(2);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_gestures(style_gestures());
    let err = s.call("map.topographic_style", json!({})).unwrap_err();
    assert_eq!(err.code, "style.field_missing");
    let l = lines(&s);
    assert_eq!(l.len(), 3, "{l:?}");
    assert!(l[0].get("result").is_some());
    assert!(l[1].get("result").is_some());
    assert_eq!(l[2]["error"], "style.field_missing");
    assert!(!seen.borrow().iter().any(|(c, _)| c == "layer.arrange"));
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_styling_without_its_gestures_or_without_qgis_writes_nothing() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    // Gestes non annoncés : rien n'est lu, rien n'est écrit.
    let qgis = StyleQgis::new(1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project).unwrap().with_qgis(qgis);
    for id in ["map.topographic_style", "map.graduated_style"] {
        let params = if id == "map.graduated_style" {
            json!({"layer_id": "c", "field": "f"})
        } else {
            json!({})
        };
        let err = s.call(id, params).unwrap_err();
        assert_eq!(err.code, "style.unavailable", "{id}");
    }
    assert!(seen.borrow().is_empty());
    assert!(lines(&s).is_empty());
    drop(s);
    // Sans QGIS (CLI `run`), même annoncés : le bouchon ne lit rien.
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_gestures(style_gestures());
    let err = s.call("map.topographic_style", json!({})).unwrap_err();
    assert_eq!(err.code, "style.unavailable");
    assert!(lines(&s).is_empty());
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_styling_journal_replays_identically_and_applies_each_style_once() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(StyleQgis::new(1))
        .with_gestures(style_gestures());
    // Une couche posée par le journal, non reconnue : elle va aux « Autres couches ».
    s.call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    let out = s.call("map.topographic_style", json!({})).unwrap();
    assert!(out["unchanged"].to_string().contains("ajout_g1"));
    s.call(
        "map.graduated_style",
        json!({"layer_id": "parcelles_2", "field": "contenance"}),
    )
    .unwrap();
    let l = lines(&s);
    assert_eq!(l.len(), 10);
    assert_eq!(l[0]["layer_id"], "ajout_g1");
    assert_eq!(l[8]["id"], "map.graduated_style");
    assert_eq!(l[9]["id"], "layer.set_style");
    assert_eq!(l[9]["params"]["style"]["kind"], "graduated");
    drop(s);

    // Seconde session : l'ancienne couche est encore au projet, la nouvelle s'appelle autrement.
    let qgis = StyleQgis::new(2);
    let seen = qgis.seen.clone();
    let mut replayed = HashSet::new();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .with_gestures(style_gestures())
        .run(&Router::new(&registry).with_qgis(qgis), &mut replayed)
        .unwrap();
    // Seule la couche posée par ce rejeu est retenue : un style ne pose aucune couche.
    assert_eq!(replayed, HashSet::from(["ajout_g2".to_string()]));
    assert_eq!(
        report,
        ReplayReport {
            steps: 10,
            identical: true
        }
    );
    let count = |id: &str| seen.borrow().iter().filter(|(c, _)| c == id).count();
    assert_eq!(count("project.inventory"), 1);
    assert_eq!(count("layer.set_style"), 6);
    assert_eq!(count("layer.arrange"), 1);
    // Le rangement rejoué porte la couche de ce rejeu, jamais celle de l'origine.
    let arranged = seen
        .borrow()
        .iter()
        .find(|(c, _)| c == "layer.arrange")
        .unwrap()
        .1
        .to_string();
    assert!(
        arranged.contains("ajout_g2") && !arranged.contains("ajout_g1"),
        "{arranged}"
    );
    // Un second rejeu, avec la mémoire du premier : toujours identique.
    let report = Replay::load(&registry, &project)
        .unwrap()
        .with_gestures(style_gestures())
        .run(
            &Router::new(&registry).with_qgis(StyleQgis::new(3)),
            &mut replayed,
        )
        .unwrap();
    assert!(report.identical);
    let _ = fs::remove_dir_all(dir);
}

/// Journal d'une couche posée puis de la mise en forme (entrée « rejeu de la mise en forme
/// après des couches créées », 2026-11-09). `legacy` : la ligne de la mise en forme perd sa clé
/// `replay`, comme un journal écrit avant que l'ordre de la légende y soit gardé.
fn styling_journal(project: &Path, legacy: bool) -> PathBuf {
    let registry = Registry::standard();
    let mut s = Session::open(&registry, project)
        .unwrap()
        .with_qgis(StyleQgis::new(1))
        .with_gestures(style_gestures());
    s.call("layer.add", json!({"path": "pts.csv", "crs": "EPSG:2154"}))
        .unwrap();
    s.call("map.topographic_style", json!({})).unwrap();
    let l = lines(&s);
    // Assez long pour être résumé : seules taille et empreinte sont gardées.
    assert_eq!(l[1]["result"]["truncated"], true);
    assert_eq!(
        l[1]["replay"],
        json!({"legend": ["mnt_1", "parcelles_2", "plu_3", "gares_4", "routes_5", "ajout_g1"]})
    );
    let path = s.journal_path();
    drop(s);
    if legacy {
        let text: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| {
                let mut v: Value = serde_json::from_str(line).unwrap();
                v.as_object_mut().unwrap().remove("replay");
                v.to_string()
            })
            .collect();
        fs::write(&path, text.join("\n") + "\n").unwrap();
    }
    path
}

fn replay_styling(legacy: bool, qgis: StyleQgis) -> Result<ReplayReport, ErrorReport> {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let _ = styling_journal(&project, legacy);
    let outcome = Replay::load(&registry, &project)
        .unwrap()
        .with_gestures(style_gestures())
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new());
    let _ = fs::remove_dir_all(dir);
    outcome
}

#[test]
fn a_styling_replays_identically_when_the_legend_was_rearranged() {
    for legacy in [false, true] {
        let mut qgis = StyleQgis::new(2);
        qgis.rearranged = true;
        assert_eq!(
            replay_styling(legacy, qgis).unwrap(),
            ReplayReport {
                steps: 8,
                identical: true
            },
            "journal ancien : {legacy}"
        );
    }
}

#[test]
fn a_styling_on_a_changed_project_still_diverges_with_a_rearranged_legend() {
    type Change = fn(&mut StyleQgis);
    let changes: [(&str, Change); 3] = [
        ("couche retirée", |q| q.dropped = Some("plu_3")),
        ("couche ajoutée", |q| q.extra = true),
        ("couche renommée", |q| q.renamed = Some("gares_4")),
    ];
    for (what, change) in changes {
        for legacy in [false, true] {
            let mut qgis = StyleQgis::new(2);
            qgis.rearranged = true;
            change(&mut qgis);
            let err = replay_styling(legacy, qgis)
                .expect_err(&format!("{what}, journal ancien : {legacy}"));
            assert_eq!(
                (err.code.as_str(), err.step),
                ("replay.diverged", Some(2)),
                "{what}, journal ancien : {legacy}"
            );
            assert!(err.message.contains("map.topographic_style"));
        }
    }
}

#[test]
fn a_legend_reorders_only_the_layers_it_names_at_their_own_places() {
    let inv = |ids: &[&str]| json!({"layers": ids.iter().map(|id| json!({"layer_id": id})).collect::<Vec<_>>()});
    let legend = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // Les couches nommées reprennent l'ordre de la légende ; « x » (non nommée) reste en 2e.
    assert_eq!(
        in_legend_order(inv(&["c", "x", "b", "a"]), &legend(&["a", "b", "c"])),
        inv(&["a", "x", "b", "c"])
    );
    // Légende vide, ids absents, doublons, inventaire sans couches : rien ne casse.
    assert_eq!(in_legend_order(inv(&["b", "a"]), &[]), inv(&["b", "a"]));
    assert_eq!(
        in_legend_order(inv(&["b", "a"]), &legend(&["z", "a", "a", "b"])),
        inv(&["a", "b"])
    );
    assert_eq!(in_legend_order(json!({}), &legend(&["a"])), json!({}));
}

#[test]
fn a_malformed_legend_falls_back_to_the_dictated_orders() {
    for bad in [json!(null), json!({}), json!("x"), json!([1, 2])] {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let path = styling_journal(&project, false);
        let text: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| {
                let mut v: Value = serde_json::from_str(line).unwrap();
                if v.get("replay").is_some() {
                    v["replay"]["legend"] = bad.clone();
                }
                v.to_string()
            })
            .collect();
        fs::write(&path, text.join("\n") + "\n").unwrap();
        let mut qgis = StyleQgis::new(2);
        qgis.rearranged = true;
        let report = Replay::load(&registry, &project)
            .unwrap()
            .with_gestures(style_gestures())
            .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new());
        assert!(report.unwrap().identical, "légende {bad}");
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn a_styling_line_replayed_without_its_gestures_stops_on_style_unavailable() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(StyleQgis::new(1))
        .with_gestures(style_gestures());
    s.call("system.about", json!({})).unwrap();
    s.call(
        "map.graduated_style",
        json!({"layer_id": "parcelles_2", "field": "contenance"}),
    )
    .unwrap();
    drop(s);
    let replay_with = |gestures: Vec<String>| {
        Replay::load(&registry, &project)
            .unwrap()
            .with_gestures(gestures)
            .run(
                &Router::new(&registry).with_qgis(StyleQgis::new(2)),
                &mut HashSet::new(),
            )
            .unwrap_err()
    };
    // Sans gestes annoncés (CLI `replay`, sans QGIS) : arrêt à la ligne mère, nommé.
    let err = replay_with(Vec::new());
    assert_eq!(
        (err.code.as_str(), err.step),
        ("style.unavailable", Some(2))
    );
    // La ligne dictée sans son geste : arrêt à la ligne 3, et non une divergence obscure.
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(StyleQgis::new(2)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, "style.unavailable");
    let err = replay_with(vec!["layer.arrange".to_string()]);
    assert_eq!(
        (err.code.as_str(), err.step),
        ("style.unavailable", Some(2))
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_dictated_style_line_replayed_without_its_gesture_stops_on_style_unavailable() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(StyleQgis::new(1))
        .with_gestures(style_gestures());
    s.call(
        "map.graduated_style",
        json!({"layer_id": "parcelles_2", "field": "contenance"}),
    )
    .unwrap();
    drop(s);
    // Le journal amputé de sa ligne mère : la ligne dictée reste seule.
    let journal = project
        .parent()
        .unwrap()
        .join("p.cassini")
        .join(JOURNAL_FILE);
    let text = fs::read_to_string(&journal).unwrap();
    let second = text.lines().nth(1).unwrap().to_string();
    fs::write(&journal, second + "\n").unwrap();
    let err = Replay::load(&registry, &project)
        .unwrap()
        .with_gestures(["map.graduated_style".to_string()])
        .run(
            &Router::new(&registry).with_qgis(StyleQgis::new(2)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!(
        (err.code.as_str(), err.step),
        ("style.unavailable", Some(1))
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn partial_gestures_refuse_the_topographic_style_but_not_the_graduated_one() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let qgis = StyleQgis::new(1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_gestures([
            "project.inventory".to_string(),
            "layer.set_style".to_string(),
        ]);
    let err = s.call("map.topographic_style", json!({})).unwrap_err();
    assert_eq!(err.code, "style.unavailable");
    assert!(seen.borrow().is_empty(), "rien n'est lu");
    assert!(lines(&s).is_empty(), "rien n'est écrit");
    let out = s
        .call(
            "map.graduated_style",
            json!({"layer_id": "parcelles_2", "field": "contenance"}),
        )
        .unwrap();
    assert!(out.get("orders").is_none());
    let ids: Vec<String> = lines(&s)
        .iter()
        .map(|l| l["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["map.graduated_style", "layer.set_style"]);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

/// Commande de test qui demande un ordre qu'elle ne déclare pas.
struct BadOrders;
impl crate::registry::Command for BadOrders {
    type Params = serde_json::Map<String, Value>;
    type Output = Value;
    fn spec() -> crate::registry::CommandSpec {
        crate::registry::CommandSpec::new::<serde_json::Map<String, Value>, Value>(
            "test.bad_orders",
            "Test.",
            "Test.",
        )
        .runs_in(crate::registry::RunsIn::Core)
        .cancellable(false)
        .dictates(&["layer.set_style"])
        .tests(&["t"])
    }
    fn run(
        &self,
        _p: serde_json::Map<String, Value>,
    ) -> Result<Value, crate::registry::CommandFailure> {
        Ok(json!({"ok": true, "orders": [
            {"command": "layer.set_style", "params": {"layer_id": "a", "style": {"kind": "hillshade"}}},
            {"command": "layer.arrange", "params": {"groups": []}}
        ]}))
    }
}

#[test]
fn an_undeclared_order_is_an_error_and_nothing_is_dictated() {
    let (dir, project, _) = temp_project();
    let mut registry = Registry::standard();
    registry.register(BadOrders);
    let qgis = StyleQgis::new(1);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_gestures(style_gestures());
    let err = s.call("test.bad_orders", json!({})).unwrap_err();
    assert_eq!(err.code, "registry.invalid_order");
    assert!(err.message.contains("layer.arrange"));
    assert!(seen.borrow().is_empty());
    let l = lines(&s);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0]["error"], "registry.invalid_order");
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

// --- Style des rasters de résultat (bogue du fond noir, 2026-11-09). ---

/// QGIS de test qui ouvre les rasters (ids `<prefix><n>`) et pose les styles (ou les refuse).
struct ResultStyleQgis {
    seen: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
    prefix: &'static str,
    refuse_style: bool,
}

impl ResultStyleQgis {
    fn new(prefix: &'static str) -> Self {
        ResultStyleQgis {
            seen: Default::default(),
            prefix,
            refuse_style: false,
        }
    }
}

impl Executor for ResultStyleQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.seen.borrow_mut().push((id.to_string(), p.clone()));
        match id {
            "layer.open_raster" => {
                let n = self.seen.borrow().len();
                let stem = Path::new(p["path"].as_str().unwrap())
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                Ok(json!({"layer_id": format!("{}{n}", self.prefix), "name": stem}))
            }
            "layer.set_style" if self.refuse_style => Err(RegistryError::Delegated {
                code: "style.invalid".into(),
                message: "Style refusé : forme inconnue « result_ramp ».".into(),
            }),
            "layer.set_style" => Ok(json!({"layer_id": p["layer_id"]})),
            other => panic!("commande inattendue : {other}"),
        }
    }
}

/// L'ouverture des rasters et le geste de style, tels que l'extension les annonce.
fn result_style_gestures() -> Vec<String> {
    ["layer.open_raster", "layer.set_style"]
        .map(String::from)
        .to_vec()
}

/// Les formes de style que la nouvelle extension annonce, dont `result_ramp`.
fn result_ramp_kind() -> Vec<String> {
    vec!["hillshade".to_string(), "result_ramp".to_string()]
}

fn viewshed_params() -> Value {
    json!({"dem": "colline.tif",
        "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
        "radius": {"value": 200, "unit": "m"}})
}

fn with_hill(dir: &Path) {
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-colline-2154.tif"),
        dir.join("colline.tif"),
    )
    .unwrap();
}

#[test]
fn a_result_raster_is_opened_then_styled_on_the_opened_layer() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    with_hill(&dir);
    let registry = Registry::standard();
    let qgis = ResultStyleQgis::new("r");
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_raster_opening(true)
        .with_gestures(result_style_gestures())
        .with_style_kinds(result_ramp_kind());
    let slope = s.call_in_plan("terrain.slope", slope_params(), 2).unwrap();
    // Le résultat rendu reste celui de la pente.
    assert!(slope.get("layer_id").is_none());
    assert_eq!(slope["unit"], "degrees");
    s.call_from("terrain.viewshed", viewshed_params(), Some("external"))
        .unwrap();
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "terrain.slope",
            "layer.open_raster",
            "layer.set_style",
            "terrain.viewshed",
            "layer.open_raster",
            "layer.set_style"
        ]
    );
    // Le style vise la couche que l'ouverture vient de rendre, avec l'origine de l'étape.
    for (open, style, origin, plan_step) in [
        (1, 2, Value::Null, json!(2)),
        (4, 5, json!("external"), Value::Null),
    ] {
        assert_eq!(l[style]["params"]["layer_id"], l[open]["layer_id"]);
        assert!(l[style].get("layer_id").is_none(), "{}", l[style]);
        for line in [&l[open], &l[style]] {
            assert_eq!(line.get("origin").cloned().unwrap_or(Value::Null), origin);
            assert_eq!(
                line.get("plan_step").cloned().unwrap_or(Value::Null),
                plan_step
            );
        }
    }
    let slope_style = &l[2]["params"]["style"];
    assert_eq!(slope_style["kind"], "result_ramp");
    // Un plan incliné : pente constante, une seule classe à sa valeur.
    assert_eq!(slope_style["interpolation"], "discrete");
    assert_eq!(slope_style["stops"][0]["value"], slope["min"]);
    assert_eq!(slope_style["unit"], "°");
    assert_eq!(slope_style["transparent"], json!([-9999.0]));
    assert_eq!(
        l[5]["params"]["style"],
        json!({"kind": "result_ramp", "interpolation": "discrete",
               "stops": [{"value": 1.0, "color": "#b2182b", "label": "Visible"}],
               "transparent": [0.0, -9999.0], "opacity": 0.5})
    );
    // L'extension reçoit l'ordre de style tel qu'il est journalisé.
    let seen = seen.borrow();
    assert_eq!(seen.len(), 4);
    assert_eq!(seen[1].0, "layer.set_style");
    assert_eq!(seen[1].1, l[2]["params"]);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_cumulative_viewshed_ramps_to_its_highest_count_and_a_hillshade_keeps_its_style() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    with_hill(&dir);
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(ResultStyleQgis::new("r"))
        .with_raster_opening(true)
        .with_gestures(result_style_gestures())
        .with_style_kinds(result_ramp_kind());
    let out = s
        .call(
            "terrain.viewshed_cumulative",
            json!({"dem": "colline.tif",
                "observers": [{"x": 652030, "y": 6850250, "epsg": 2154},
                              {"x": 652130, "y": 6850250, "epsg": 2154}],
                "radius": {"value": 200, "unit": "m"}}),
        )
        .unwrap();
    let top = out["max_count"].as_f64().unwrap();
    assert!(
        top > 1.0,
        "deux observateurs qui voient une même cellule : {out}"
    );
    s.call("terrain.hillshade", json!({"dem": "dem/plan.tif"}))
        .unwrap();
    let l = lines(&s);
    let cumulative = &l[2]["params"]["style"];
    assert_eq!(cumulative["interpolation"], "linear");
    assert_eq!(cumulative["transparent"], json!([0.0, -9999.0]));
    let stops = cumulative["stops"].as_array().unwrap();
    assert_eq!(stops.first().unwrap()["value"], json!(1.0));
    assert_eq!(stops.last().unwrap()["value"], json!(top));
    assert_eq!(l[5]["id"], "layer.set_style");
    assert_eq!(
        l[5]["params"]["style"],
        json!({"kind": "hillshade", "opacity": crate::style::table::HILLSHADE_OPACITY})
    );
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_extension_without_the_style_gesture_or_the_result_ramp_kind_opens_the_raster_without_style() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    // Sans le geste `layer.set_style`, formes annoncées ou non ; puis avec le geste mais sans la
    // forme `result_ramp` (extension plus ancienne que le changement du 2026-11-09).
    let cases: [(Vec<String>, Vec<String>); 2] = [
        (vec!["layer.open_raster".to_string()], result_ramp_kind()),
        (result_style_gestures(), vec!["hillshade".to_string()]),
    ];
    for (gestures, kinds) in cases {
        let qgis = ResultStyleQgis::new("r");
        let seen = qgis.seen.clone();
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(qgis)
            .with_raster_opening(true)
            .with_gestures(gestures.clone())
            .with_style_kinds(kinds);
        let before = lines(&s).len();
        s.call("terrain.slope", slope_params()).unwrap();
        let ids: Vec<Value> = lines(&s)[before..]
            .iter()
            .map(|l| l["id"].clone())
            .collect();
        assert_eq!(
            ids,
            [json!("terrain.slope"), json!("layer.open_raster")],
            "{gestures:?}"
        );
        let seen: Vec<String> = seen.borrow().iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(seen, ["layer.open_raster"], "{gestures:?}");
        drop(s);
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_refused_result_style_is_the_error_of_the_call_journaled_at_its_step() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let mut qgis = ResultStyleQgis::new("r");
    qgis.refuse_style = true;
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(qgis)
        .with_raster_opening(true)
        .with_gestures(result_style_gestures())
        .with_style_kinds(result_ramp_kind());
    let err = s.call("terrain.slope", slope_params()).unwrap_err();
    assert_eq!(err.code, "style.invalid");
    let l = lines(&s);
    assert_eq!(l.len(), 3);
    assert!(l[0]["result"].is_object());
    assert_eq!(l[1]["layer_id"], "r1");
    assert_eq!(l[2]["id"], "layer.set_style");
    assert_eq!(l[2]["error"], "style.invalid");
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_styled_result_raster_replays_identically_on_the_replayed_layer() {
    let (dir, project, _) = temp_project();
    with_dem(&dir);
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(ResultStyleQgis::new("r"))
        .with_raster_opening(true)
        .with_gestures(result_style_gestures())
        .with_style_kinds(result_ramp_kind());
    s.call("terrain.slope", slope_params()).unwrap();
    let logged = lines(&s)[2]["params"]["style"].clone();
    drop(s);
    // Le rejeu pose le style journalisé sur la couche qu'il vient d'ouvrir, pas sur l'ancienne.
    let qgis = ResultStyleQgis::new("neuf");
    let seen = qgis.seen.clone();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .with_gestures(result_style_gestures())
        .with_style_kinds(result_ramp_kind())
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
        .unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 3,
            identical: true
        }
    );
    let seen = seen.borrow();
    let ids: Vec<&str> = seen.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["layer.open_raster", "layer.set_style"]);
    assert_eq!(seen[1].1, json!({"layer_id": "neuf1", "style": logged}));
    drop(seen);
    // Un exécutant de rejeu qui n'annonce pas le geste, ou pas la forme `result_ramp`, s'arrête
    // à la ligne de style, sans la lui envoyer.
    for (gestures, kinds) in [
        (vec!["layer.open_raster".to_string()], result_ramp_kind()),
        (result_style_gestures(), vec!["hillshade".to_string()]),
    ] {
        let qgis = ResultStyleQgis::new("x");
        let seen = qgis.seen.clone();
        let err = Replay::load(&registry, &project)
            .unwrap()
            .with_gestures(gestures)
            .with_style_kinds(kinds)
            .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
            .unwrap_err();
        assert_eq!(
            (err.code.as_str(), err.step),
            ("style.unavailable", Some(3))
        );
        assert!(seen.borrow().iter().all(|(id, _)| id != "layer.set_style"));
    }
    let _ = fs::remove_dir_all(dir);
}

// -- data.convert : l'extension suit le format (epic 10, entrée 9) ---------------------------

/// QGIS de test pour `data.convert` : écrit le fichier dicté (et les compagnons d'un
/// Shapefile), puis rend le résultat de la fiche, ou l'annulation après avoir écrit.
struct ConvertQgis {
    cancel: bool,
}

impl Executor for ConvertQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        assert_eq!(id, "data.convert");
        let out = PathBuf::from(p["out"].as_str().unwrap());
        fs::write(&out, b"donnees").unwrap();
        if out.extension().and_then(|e| e.to_str()) == Some("shp") {
            for ext in ["shx", "dbf", "prj", "cpg", "qix"] {
                fs::write(out.with_extension(ext), b"x").unwrap();
            }
        }
        if self.cancel {
            return Err(RegistryError::Cancelled(id.to_string()));
        }
        Ok(json!({"file": out.to_string_lossy(), "format": p["format"], "feature_count": 3}))
    }
}

#[test]
fn a_conversion_is_named_by_its_format_and_replays_identically() {
    let (dir, project, token) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(ConvertQgis { cancel: false });
    for format in ["gpkg", "shp", "geojson"] {
        s.call(
            "data.convert",
            json!({"layer_id": "gares", "format": format}),
        )
        .unwrap();
    }
    // Un format hors de la fiche est refusé avant QGIS : rien n'est journalisé ni écrit.
    let err = s
        .call(
            "data.convert",
            json!({"layer_id": "gares", "format": "xlsx"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "registry.invalid_params");
    let l = lines(&s);
    assert_eq!(l.len(), 3);
    let names = [
        "1-data.convert.gpkg",
        "2-data.convert.shp",
        "3-data.convert.geojson",
    ];
    for (line, name) in l.iter().zip(names) {
        assert_eq!(line["params"]["out"], name);
        assert_eq!(line["result"]["file"], name);
        assert!(line.get("layer_id").is_none(), "aucune couche posée");
        assert!(s.work_dir().join(name).exists(), "{name}");
    }
    assert!(s.work_dir().join("2-data.convert.dbf").exists());
    let raw = fs::read_to_string(s.journal_path()).unwrap();
    assert!(
        !raw.contains(&token),
        "chemin absolu dans le journal : {raw}"
    );

    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(ConvertQgis { cancel: false }),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(
        report,
        ReplayReport {
            steps: 3,
            identical: true
        }
    );
    // Le rejeu écrit sous `replay/`, compagnons compris, dans un dossier qu'il vide d'abord.
    let replayed = s.work_dir().join("replay");
    for name in names {
        assert!(replayed.join(name).exists(), "{name}");
    }
    assert!(replayed.join("2-data.convert.shx").exists());
    // Une conversion n'est pas une couche : elle ne rend aucune sortie intermédiaire, et le
    // repère `out:<k>.<n>` ne la désigne pas (son extension dépend de ses paramètres).
    assert!(intermediate_files(&registry, &s.journal_path()).is_empty());
    assert_eq!(
        reference_file(&registry, "out:2.1", &vec!["data.convert".to_string(); 3]),
        None
    );
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_cancelled_shapefile_conversion_leaves_no_companion_behind() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(ConvertQgis { cancel: true });
    let err = s
        .call(
            "data.convert",
            json!({"layer_id": "gares", "format": "shp"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "registry.cancelled");
    assert_eq!(lines(&s)[0]["cancelled"], true);
    let left: Vec<String> = fs::read_dir(s.work_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("1-data.convert"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_shapefile_output_names_its_companions() {
    assert_eq!(
        output_family("2-data.convert.shp"),
        [
            "2-data.convert.shp",
            "2-data.convert.shx",
            "2-data.convert.dbf",
            "2-data.convert.prj",
            "2-data.convert.cpg",
            "2-data.convert.qix"
        ]
    );
    assert_eq!(
        output_family("1-analysis.buffer.gpkg"),
        ["1-analysis.buffer.gpkg"]
    );
}

#[test]
fn a_conversion_after_a_plan_layer_does_not_hide_it() {
    // Une conversion consomme la couche d'une étape antérieure du plan sans rendre de couche :
    // la couche convertie reste visible du modèle (règle de 5.34 : seule une étape qui rend une
    // couche cache ce qu'elle consomme).
    let (dir, _, _) = temp_project();
    let registry = Registry::standard();
    let journal = dir.join("journal.jsonl");
    let rows = [
        json!({"step": 1, "id": "analysis.buffer", "plan_step": 1, "layer_id": "tampon_1",
               "params": {"layer_id": "gares", "distance": {"value": 500, "unit": "m"},
                          "out": "1-analysis.buffer.gpkg"},
               "result": {"layer_id": "tampon_1"}}),
        json!({"step": 2, "id": "data.convert", "plan_step": 2,
               "params": {"layer_id": "tampon_1", "format": "shp", "out": "2-data.convert.shp"},
               "result": {"file": "2-data.convert.shp", "format": "shp", "feature_count": 3}}),
    ];
    let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
    fs::write(&journal, text).unwrap();
    assert!(intermediate_files(&registry, &journal).is_empty());
    let _ = fs::remove_dir_all(dir);
}

// -- layer.remove : retrait du projet, jamais du fichier (epic 10, entrée 27) ----------------

/// Couches d'un projet QGIS de test, partagées entre l'exécutant d'une session et celui d'un
/// rejeu dans la même session.
type ProjectLayers = std::rc::Rc<std::cell::RefCell<HashSet<String>>>;

/// Projet ouvert avec la seule couche `routes_1`, comme à la réouverture du fichier de projet.
fn project_with_roads() -> ProjectLayers {
    std::rc::Rc::new(std::cell::RefCell::new(HashSet::from([
        "routes_1".to_string()
    ])))
}

/// QGIS de test avec état : `analysis.buffer` pose une couche d'id neuf à chaque appel (préfixé
/// de la génération, comme une session QGIS neuve), `layer.remove` retire une couche du projet
/// sans erreur si elle est déjà absente, et le tampon d'une couche absente échoue par
/// `layer.not_found`, qui la nomme. Les couches de `unsaved` (en mémoire ou en édition
/// modifiée) sont refusées par `layer.unsaved` et restent au projet.
struct RemoveQgis {
    generation: u32,
    count: std::cell::Cell<u32>,
    layers: ProjectLayers,
    unsaved: HashSet<String>,
    seen: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
}

impl RemoveQgis {
    fn new(generation: u32, layers: &ProjectLayers) -> Self {
        RemoveQgis {
            generation,
            count: std::cell::Cell::new(0),
            layers: layers.clone(),
            unsaved: HashSet::new(),
            seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }

    fn with_unsaved(mut self, layer_id: &str) -> Self {
        self.unsaved.insert(layer_id.to_string());
        self
    }
}

impl Executor for RemoveQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.seen.borrow_mut().push((id.to_string(), p.clone()));
        let layer = p["layer_id"].as_str().unwrap().to_string();
        match id {
            "analysis.buffer" => {
                if !self.layers.borrow().contains(&layer) {
                    return Err(RegistryError::Delegated {
                        code: "layer.not_found".into(),
                        message: format!("la couche {layer} n'est pas dans le projet"),
                    });
                }
                let n = self.count.get() + 1;
                self.count.set(n);
                fs::write(
                    p["out"].as_str().unwrap(),
                    format!("gpkg-{}", self.generation),
                )
                .unwrap();
                let new = format!("tampon_g{}_{n}", self.generation);
                self.layers.borrow_mut().insert(new.clone());
                Ok(json!({"layer_id": new, "feature_count": 2}))
            }
            "layer.remove" => {
                if self.layers.borrow().contains(&layer) && self.unsaved.contains(&layer) {
                    return Err(RegistryError::Delegated {
                        code: "layer.unsaved".into(),
                        message: format!("la couche {layer} n'est pas enregistrée"),
                    });
                }
                // Idempotente : une couche absente rend le même résultat, sans erreur.
                self.layers.borrow_mut().remove(&layer);
                Ok(json!({"layer_id": layer}))
            }
            other => panic!("commande inattendue : {other}"),
        }
    }
}

fn identical(steps: usize) -> ReplayReport {
    ReplayReport {
        steps,
        identical: true,
    }
}

#[test]
fn a_removed_layer_leaves_the_project_and_replays_identically_in_the_same_and_a_new_session() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let layers = project_with_roads();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RemoveQgis::new(1, &layers));
    let out = s
        .call("layer.remove", json!({"layer_id": "routes_1"}))
        .unwrap();
    assert_eq!(out, json!({"layer_id": "routes_1"}));
    assert!(layers.borrow().is_empty());
    // Une couche absente rend le même résultat, sans erreur.
    let again = s
        .call("layer.remove", json!({"layer_id": "routes_1"}))
        .unwrap();
    assert_eq!(again, out);
    // Paramètres hors de la fiche : refus avant QGIS, rien n'est journalisé.
    for bad in [
        json!({}),
        json!({"layer_id": "routes_1", "delete_file": true}),
    ] {
        let err = s.call("layer.remove", bad).unwrap_err();
        assert_eq!(err.code, "registry.invalid_params");
    }
    let l = lines(&s);
    assert_eq!(l.len(), 2);
    for line in &l {
        assert_eq!(line["params"], json!({"layer_id": "routes_1"}));
        assert_eq!(line["result"], json!({"layer_id": "routes_1"}));
        // Sortie `none` : le `layer_id` du résultat n'est pas une couche posée.
        assert!(
            line.get("layer_id").is_none(),
            "aucune couche posée : {line}"
        );
    }
    assert!(intermediate_files(&registry, &s.journal_path()).is_empty());
    drop(s);

    // Rejeu dans la même session : la couche est déjà retirée, le rejeu reste identique.
    let qgis = RemoveQgis::new(1, &layers);
    let seen = qgis.seen.clone();
    let mut replayed = HashSet::new();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut replayed)
        .unwrap();
    assert_eq!(report, identical(2));
    // Rejouée, jamais sautée.
    assert_eq!(seen.borrow().len(), 2);
    assert!(replayed.is_empty(), "un retrait ne pose aucune couche");

    // Rejeu dans une session neuve : le projet rouvert porte la couche, le rejeu la retire.
    let reopened = project_with_roads();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(2, &reopened)),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(report, identical(2));
    assert!(reopened.borrow().is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn removing_an_analysis_result_replays_on_the_layer_the_replay_created() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let layers = project_with_roads();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RemoveQgis::new(1, &layers));
    let buffer = s
        .call(
            "analysis.buffer",
            json!({"layer_id": "routes_1", "distance": {"value": 50, "unit": "m"}}),
        )
        .unwrap();
    assert_eq!(buffer["layer_id"], "tampon_g1_1");
    s.call("layer.remove", json!({"layer_id": "tampon_g1_1"}))
        .unwrap();
    assert_eq!(*layers.borrow(), HashSet::from(["routes_1".to_string()]));
    let l = lines(&s);
    assert_eq!(l[0]["layer_id"], "tampon_g1_1");
    assert!(l[1].get("layer_id").is_none());
    drop(s);

    // Session neuve : le tampon rejoué a un id neuf, que le retrait reçoit par la liaison.
    let reopened = project_with_roads();
    let qgis = RemoveQgis::new(2, &reopened);
    let seen = qgis.seen.clone();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
        .unwrap();
    assert_eq!(report, identical(2));
    assert_eq!(seen.borrow()[1].1, json!({"layer_id": "tampon_g2_1"}));
    assert_eq!(*reopened.borrow(), HashSet::from(["routes_1".to_string()]));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_step_that_cites_a_removed_layer_fails_naming_it_and_replays_identically() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let layers = project_with_roads();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RemoveQgis::new(1, &layers));
    s.call("layer.remove", json!({"layer_id": "routes_1"}))
        .unwrap();
    let err = s
        .call(
            "analysis.buffer",
            json!({"layer_id": "routes_1", "distance": {"value": 50, "unit": "m"}}),
        )
        .unwrap_err();
    assert_eq!(err.code, "layer.not_found");
    assert!(err.message.contains("routes_1"), "{}", err.message);
    assert_eq!(lines(&s)[1]["error"], "layer.not_found");
    drop(s);

    // Même erreur au rejeu, en même session comme en session neuve : identique.
    for state in [layers.clone(), project_with_roads()] {
        let report = Replay::load(&registry, &project)
            .unwrap()
            .run(
                &Router::new(&registry).with_qgis(RemoveQgis::new(2, &state)),
                &mut HashSet::new(),
            )
            .unwrap();
        assert_eq!(report, identical(2));
        assert!(state.borrow().is_empty());
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn replaying_in_the_same_session_an_analysis_of_a_layer_removed_after_diverges_at_the_analysis() {
    // Limite connue (AD-11) : la couche analysée puis retirée n'est plus au projet de la même
    // session, et le rejeu s'arrête à l'analyse, nommée ; il ne saute aucune étape.
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let layers = project_with_roads();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RemoveQgis::new(1, &layers));
    s.call(
        "analysis.buffer",
        json!({"layer_id": "routes_1", "distance": {"value": 50, "unit": "m"}}),
    )
    .unwrap();
    s.call("layer.remove", json!({"layer_id": "routes_1"}))
        .unwrap();
    drop(s);
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(1, &layers)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!((err.code.as_str(), err.step), ("replay.diverged", Some(1)));
    // En session neuve, le même journal rejoue à l'identique.
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(2, &project_with_roads())),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(report, identical(2));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_removal_refused_for_an_unsaved_layer_keeps_it_and_replays_identically_while_unsaved() {
    // Changement de cap du 2026-11-10 : une couche en mémoire ou en édition modifiée n'est pas
    // retirée (`layer.unsaved`). Le rejeu passe par le chemin générique d'une étape QGIS
    // refusée : refusée de nouveau, identique ; retirée au rejeu, divergente, rien de sauté.
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let layers = project_with_roads();
    let mut s = Session::open(&registry, &project)
        .unwrap()
        .with_qgis(RemoveQgis::new(1, &layers).with_unsaved("routes_1"));
    let err = s
        .call("layer.remove", json!({"layer_id": "routes_1"}))
        .unwrap_err();
    assert_eq!(err.code, "layer.unsaved");
    assert!(
        layers.borrow().contains("routes_1"),
        "la couche reste au projet"
    );
    let l = lines(&s);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0]["error"], "layer.unsaved");
    assert!(l[0].get("result").is_none(), "{}", l[0]);
    drop(s);

    // Même session, couche toujours non enregistrée : même refus, rejeu identique.
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(1, &layers).with_unsaved("routes_1")),
            &mut HashSet::new(),
        )
        .unwrap();
    assert_eq!(report, identical(1));
    assert!(layers.borrow().contains("routes_1"));

    // Couche enregistrée entre-temps : le retrait réussit au rejeu, qui diverge à l'étape.
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(1, &layers)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!((err.code.as_str(), err.step), ("replay.diverged", Some(1)));

    // Session neuve sans la couche (une couche en mémoire ne survit pas) : même divergence.
    let empty: ProjectLayers = std::rc::Rc::new(std::cell::RefCell::new(HashSet::new()));
    let err = Replay::load(&registry, &project)
        .unwrap()
        .run(
            &Router::new(&registry).with_qgis(RemoveQgis::new(2, &empty)),
            &mut HashSet::new(),
        )
        .unwrap_err();
    assert_eq!((err.code.as_str(), err.step), ("replay.diverged", Some(1)));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_last_plan_is_read_from_the_last_first_step() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let rows = [
        // Plan 1 : une sélection ; il n'est plus le dernier.
        json!({"id": "analysis.select_by_attribute", "plan_step": 1,
            "params": {"layer_id": "ancien", "field": "f", "operator": "eq", "value": "U"},
            "layer_id": "r1", "result": {}}),
        // Plan 2 : pente, ouverture dictée (même place, ne commence pas de plan), statistiques.
        json!({"id": "terrain.slope", "plan_step": 1,
            "params": {"dem": "dem/plan.tif", "unit": "degrees", "out": "2-terrain.slope.tif"},
            "result": {}}),
        json!({"id": "layer.open_raster", "plan_step": 1, "params": {"path": "out:2.1"},
            "layer_id": "R", "result": {}}),
        json!({"id": "analysis.zonal_statistics", "plan_step": 2,
            "params": {"layer_id": "R", "points_layer_id": {"layer_id": "gares", "n": 3}},
            "layer_id": "S", "result": {}}),
        // Une étape en échec : elle cite, mais ne laisse aucun fichier.
        json!({"id": "analysis.buffer", "plan_step": 3,
            "params": {"layer_id": "communes"}, "error": {"code": "x"}}),
        // Hors plan : ignorée.
        json!({"id": "analysis.buffer", "params": {"layer_id": "hors_plan"}, "result": {}}),
    ];
    hand_journal(&project, &rows);
    let journal = dir.join("p.cassini").join(JOURNAL_FILE);
    let plan = last_plan(&registry, &journal);
    let mut results: Vec<&str> = plan.results.iter().map(String::as_str).collect();
    results.sort_unstable();
    assert_eq!(
        results,
        ["2-terrain.slope.tif", "4-analysis.zonal_statistics.gpkg"]
    );
    // Les couches produites par le plan (R, S) n'y sont pas ; la sortie entière cite sa couche.
    for id in ["gares", "communes", "dem/plan.tif"] {
        assert!(plan.cited.contains(id), "{id} : {:?}", plan.cited);
    }
    for id in ["R", "S", "ancien", "hors_plan"] {
        assert!(!plan.cited.contains(id), "{id} : {:?}", plan.cited);
    }
    // Journal absent : plan vide.
    assert_eq!(
        last_plan(&registry, &dir.join("absent.jsonl")),
        crate::agent_inventory::LastPlan::default()
    );
    let _ = fs::remove_dir_all(dir);
}
// ---- Epic 10, entrée 30 : `data.convert_raster`, puis l'ouverture dictée et la pente ----

/// Session d'origine : un ASCII Grid converti par QGIS, ouvert par l'étape dictée, puis la
/// pente calculée par le noyau sur la sortie de la conversion (`out:1.1`, le repère d'un
/// `step:1` de plan).
fn convert_raster_journal(project_file: &Path, project: &std::rc::Rc<OpenProject>) -> String {
    let dir = project_file.parent().unwrap();
    fs::create_dir_all(dir.join("mnt")).unwrap();
    let source = dir.join("mnt/dalle.asc");
    fs::write(&source, "ncols 2\nnrows 2\n").unwrap();
    let registry = Registry::standard();
    let mut s = Session::open(&registry, project_file)
        .unwrap()
        .with_qgis(ProjectQgis::new(project, 1))
        .with_raster_opening(true);
    let converted = s
        .call(
            "data.convert_raster",
            json!({"raster": "mnt/dalle.asc", "assign_epsg": 2154}),
        )
        .unwrap();
    assert!(converted["file"]
        .as_str()
        .unwrap()
        .ends_with("1-data.convert_raster.tif"));
    s.call(
        "terrain.slope",
        json!({"dem": "out:1.1", "unit": "degrees"}),
    )
    .unwrap();
    let l = lines(&s);
    let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "data.convert_raster",
            "layer.open_raster",
            "terrain.slope",
            "layer.open_raster"
        ]
    );
    // La source est journalisée relative, avec son empreinte ; la sortie par son repère.
    assert_eq!(l[0]["params"]["raster"], json!("mnt/dalle.asc"));
    assert_eq!(l[0]["inputs"].as_array().unwrap().len(), 1);
    assert_eq!(l[1]["params"], json!({"path": "out:1.1"}));
    assert_eq!(l[1]["layer_id"], json!("raster_g1_2"));
    assert_eq!(l[2]["params"]["dem"], json!("out:1.1"));
    assert_eq!(l[3]["params"], json!({"path": "out:3.1"}));
    // L'original n'est pas touché.
    assert_eq!(fs::read_to_string(&source).unwrap(), "ncols 2\nnrows 2\n");
    fs::read_to_string(s.journal_path()).unwrap()
}

#[test]
fn a_converted_raster_is_opened_by_the_core_and_feeds_a_slope() {
    let (dir, project_file, token) = temp_project();
    let journal = convert_raster_journal(&project_file, &OpenProject::with_base());
    assert!(!journal.contains(&token), "aucun chemin absolu au journal");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_converted_raster_replays_identically_in_a_new_session_and_twice_in_the_same() {
    let (dir, project_file, _) = temp_project();
    let project = OpenProject::with_base();
    let before = convert_raster_journal(&project_file, &project);
    let identical = ReplayReport {
        steps: 4,
        identical: true,
    };
    let mut replayed = HashSet::new();
    let (outcome, _) = replay_in(&project_file, &project, 2, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    let (outcome, _) = replay_in(&project_file, &project, 3, &mut replayed);
    assert_eq!(outcome.unwrap(), identical);
    // Projet rouvert sans les rasters : la conversion est rejouée, puis rouverte, puis la pente.
    let reopened = OpenProject::with_base();
    let (outcome, _) = replay_in(&project_file, &reopened, 4, &mut HashSet::new());
    assert_eq!(outcome.unwrap(), identical);
    let files: Vec<String> = reopened
        .layers
        .borrow()
        .iter()
        .map(|(_, f)| f.clone())
        .collect();
    assert_eq!(
        files,
        [
            "communes.gpkg",
            "1-data.convert_raster.tif",
            "3-terrain.slope.tif"
        ]
    );
    let work = project_file.parent().unwrap().join("p.cassini");
    assert!(work.join("replay/1-data.convert_raster.tif").is_file());
    assert_eq!(fs::read_to_string(work.join(JOURNAL_FILE)).unwrap(), before);
    let _ = fs::remove_dir_all(dir);
}

// ---- Epic 10, entrée 31 : carroyage MGRS complété par le noyau, sources détaillées ----

/// QGIS de test de la mise en page : un inventaire d'emprise donnée (`None` : aucune), un export
/// qui note ses paramètres et rend, ou non, `source_details` ; `sources` longues ou courtes.
struct ExportQgis {
    extent: Option<Value>,
    details: bool,
    many_sources: bool,
    seen: std::rc::Rc<std::cell::RefCell<Vec<(String, Value)>>>,
}

impl ExportQgis {
    fn new(extent: Option<Value>, details: bool, many_sources: bool) -> Self {
        ExportQgis {
            extent,
            details,
            many_sources,
            seen: Default::default(),
        }
    }
}

impl Executor for ExportQgis {
    fn execute(&self, id: &str, p: Value) -> Result<Value, RegistryError> {
        self.seen.borrow_mut().push((id.to_string(), p.clone()));
        match id {
            INVENTORY => Ok(match &self.extent {
                Some(lonlat) => json!({"layers": [], "extent": {"lonlat": lonlat}}),
                None => json!({"layers": []}),
            }),
            "layout.export_pdf" => {
                let out = p["out"].as_str().unwrap().to_string();
                fs::write(&out, "pdf").unwrap();
                let sources: Vec<String> = if self.many_sources {
                    (1..=12).map(|n| format!("couche_{n:02}.gpkg")).collect()
                } else {
                    vec!["gares.gpkg".into()]
                };
                let mut result = json!({"file": out, "page_count": 1, "layer_count": sources.len(),
                    "sources": sources, "truncated": false});
                if self.details {
                    result["source_details"] = json!(sources
                        .iter()
                        .map(|s| json!({"source": s, "resolution": null,
                            "date": "2026-10-15", "date_origin": "file"}))
                        .collect::<Vec<_>>());
                }
                Ok(result)
            }
            other => panic!("commande inattendue : {other}"),
        }
    }
}

fn annecy_lonlat() -> Value {
    json!({"west": 6.05, "south": 45.78, "east": 6.25, "north": 45.95, "crs": "EPSG:4326"})
}

#[test]
fn an_mgrs_grid_is_filled_by_the_core_journaled_and_replayed_as_is() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let qgis = ExportQgis::new(Some(annecy_lonlat()), true, false);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project).unwrap().with_qgis(qgis);
    // Ce que l'appelant écrit de la zone et des carrés est ôté : le noyau les pose.
    s.call(
        "layout.export_pdf",
        json!({"title": "Secteur", "header": "Document de travail",
            "grid": {"kind": "mgrs", "interval": {"value": 1, "unit": "km"},
                     "epsg": 32631, "squares": ["31T ZZ"]}}),
    )
    .unwrap();
    let (expected_epsg, expected_squares) = crate::commands::layout_export_pdf::mgrs_grid_of(
        &json!({"extent": {"lonlat": annecy_lonlat()}}),
    )
    .unwrap();
    assert_eq!(expected_epsg, 32632);
    {
        let seen = seen.borrow();
        let ids: Vec<&str> = seen.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, [INVENTORY, "layout.export_pdf"]);
        let grid = &seen[1].1["grid"];
        assert_eq!(grid["epsg"], json!(32632));
        assert_eq!(grid["squares"], json!(expected_squares));
        assert_eq!(seen[1].1["header"], json!("Document de travail"));
    }
    let l = lines(&s);
    assert_eq!(l.len(), 1, "l'inventaire n'est pas une étape du journal");
    assert_eq!(l[0]["params"]["grid"]["epsg"], json!(32632));
    assert_eq!(l[0]["params"]["grid"]["squares"], json!(expected_squares));
    drop(s);
    // Au rejeu, l'emprise a changé (ou manque) : la zone et les carrés journalisés sont
    // repassés tels quels, sans relire l'inventaire.
    let qgis = ExportQgis::new(None, true, false);
    let seen = qgis.seen.clone();
    let report = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(qgis), &mut HashSet::new())
        .unwrap();
    assert!(report.identical);
    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].1["grid"]["epsg"], json!(32632));
    assert_eq!(seen[0].1["grid"]["squares"], json!(expected_squares));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_mgrs_grid_without_a_readable_extent_is_refused_before_the_export() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    let qgis = ExportQgis::new(None, true, false);
    let seen = qgis.seen.clone();
    let mut s = Session::open(&registry, &project).unwrap().with_qgis(qgis);
    let err = s
        .call(
            "layout.export_pdf",
            json!({"title": "Secteur", "grid": {"kind": "mgrs", "interval": {"value": 10, "unit": "km"}}}),
        )
        .unwrap_err();
    assert_eq!(err.code, "layout.no_extent");
    assert!(lines(&s).is_empty());
    assert_eq!(seen.borrow().len(), 1, "seul l'inventaire est lu");
    // Un intervalle hors liste est refusé par le noyau avant toute lecture auprès de QGIS.
    let err = s
        .call(
            "layout.export_pdf",
            json!({"title": "Secteur", "grid": {"kind": "mgrs", "interval": {"value": 5, "unit": "km"}}}),
        )
        .unwrap_err();
    assert_eq!(err.code, "registry.invalid_params");
    assert_eq!(seen.borrow().len(), 1, "l'inventaire n'est pas relu");
    // Les autres carroyages ne lisent pas l'inventaire.
    s.call(
        "layout.export_pdf",
        json!({"title": "Carte", "grid": {"kind": "projected", "epsg": 32631,
            "interval": {"value": 1, "unit": "km"}}}),
    )
    .unwrap();
    assert_eq!(seen.borrow().last().unwrap().0, "layout.export_pdf");
    assert_eq!(seen.borrow().len(), 2);
    drop(s);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_export_journaled_before_source_details_replays_identically() {
    for many in [false, true] {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        // Journal d'avant l'entrée 31 : le résultat ne porte pas `source_details`.
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(ExportQgis::new(None, false, many));
        s.call("layout.export_pdf", json!({"title": "Carte"}))
            .unwrap();
        let kept = lines(&s)[0]["result"].clone();
        assert_eq!(kept.get("truncated") == Some(&json!(true)), many, "{kept}");
        drop(s);
        // Le geste d'aujourd'hui rend `source_details` : la ligne ancienne se rejoue à l'identique.
        let report = Replay::load(&registry, &project)
            .unwrap()
            .run(
                &Router::new(&registry).with_qgis(ExportQgis::new(None, true, many)),
                &mut HashSet::new(),
            )
            .unwrap();
        assert!(report.identical, "résumé : {many}");
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn an_export_journaled_with_source_details_compares_them() {
    for many in [false, true] {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(ExportQgis::new(None, true, many));
        s.call("layout.export_pdf", json!({"title": "Carte"}))
            .unwrap();
        drop(s);
        // Un rejeu qui ne les rend plus diverge : le champ, journalisé, est comparé.
        let err = Replay::load(&registry, &project)
            .unwrap()
            .run(
                &Router::new(&registry).with_qgis(ExportQgis::new(None, false, many)),
                &mut HashSet::new(),
            )
            .unwrap_err();
        assert_eq!(err.code, "replay.diverged", "résumé : {many}");
        let _ = fs::remove_dir_all(dir);
    }
}
