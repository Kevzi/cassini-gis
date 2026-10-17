//! Forme canonique et empreintes : clés triées, FNV-1a, version du registre.

use serde_json::{Map, Value};

use crate::registry::Registry;

/// Copie dont les clés d'objet sont triées, à tous les niveaux.
pub fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Map::new();
            for (k, v) in entries {
                out.insert(k.clone(), canonical(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

/// Empreinte (FNV-1a 64 bits, sans dépendance) des ids et schémas du registre,
/// dans l'ordre des ids : elle change dès qu'une commande ou un schéma change.
pub fn registry_version(registry: &Registry) -> String {
    let mut hash = Fnv::new();
    let mut feed = |bytes: &[u8]| hash.feed(bytes);
    for (spec, _) in registry.implementations() {
        feed(spec.id.as_bytes());
        feed(canonical(&spec.params_schema).to_string().as_bytes());
        feed(canonical(&spec.output_schema).to_string().as_bytes());
    }
    hash.hex()
}

/// Empreinte du contenu d'une valeur JSON : FNV-1a de sa forme canonique.
pub(super) fn fingerprint(value: &Value) -> String {
    let mut hash = Fnv::new();
    hash.feed(canonical(value).to_string().as_bytes());
    hash.hex()
}

/// FNV-1a 64 bits, sans dépendance ; chaque `feed` est séparé du suivant.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }

    fn feed(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }

    fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}
