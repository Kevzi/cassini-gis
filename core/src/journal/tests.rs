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
        .run(&Router::new(registry))
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
        .run(&Router::new(&registry).with_qgis(InventoryQgis))
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
        .run(&Router::new(&registry).with_qgis(qgis));
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

#[test]
fn a_replay_binds_new_layer_ids_and_accepts_other_qgis_bytes() {
    let (dir, project, _) = temp_project();
    let registry = Registry::standard();
    qgis_journal(&project);
    let replay = Replay::load(&registry, &project).unwrap();
    let report = replay
        .run(&Router::new(&registry).with_qgis(SessionQgis::new(2)))
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
        .run(&Router::new(&registry).with_qgis(other))
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
        .run(&Router::new(&registry).with_qgis(other))
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

    let outcome = Replay::load(&registry, &project)
        .unwrap()
        .run(&Router::new(&registry).with_qgis(SessionQgis::new(2)));
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
