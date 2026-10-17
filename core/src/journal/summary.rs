//! Résumé d'un résultat journalisé et comparaison au rejeu.

use serde_json::{json, Value};

use super::canonical::{canonical, fingerprint};

/// Au-delà de cette taille (en octets), un résultat est résumé.
const SUMMARY_LIMIT: usize = 200;

/// Copie sans le `layer_id` de premier niveau : valeur de session, jamais comparée.
fn without_layer_id(value: &Value) -> Value {
    let mut copy = value.clone();
    if let Some(map) = copy.as_object_mut() {
        map.remove("layer_id");
    }
    copy
}

/// Le résultat rejoué redonne le résultat gardé, au `layer_id` près. Un résumé
/// se compare par taille et par empreinte du contenu ; un journal plus ancien,
/// dont le résumé n'a pas d'empreinte, ne se compare que par la taille.
pub(super) fn same_result(value: &Value, kept: &Value) -> bool {
    let again = summarize(value);
    if again.get("truncated") == Some(&Value::Bool(true)) {
        kept.get("truncated") == Some(&Value::Bool(true))
            && again.get("bytes") == kept.get("bytes")
            && kept
                .get("fingerprint")
                .is_none_or(|f| Some(f) == again.get("fingerprint"))
    } else {
        without_layer_id(&again) == without_layer_id(kept)
    }
}

/// Résultat gardé s'il tient en 200 octets, sinon son résumé : taille et
/// empreinte du contenu, sans le `layer_id` (valeur de session).
pub(super) fn summarize(value: &Value) -> Value {
    let text = value.to_string();
    if text.len() <= SUMMARY_LIMIT {
        canonical(value)
    } else {
        json!({
            "truncated": true,
            "bytes": text.len(),
            "fingerprint": fingerprint(&without_layer_id(value)),
        })
    }
}
