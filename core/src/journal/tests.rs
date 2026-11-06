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
