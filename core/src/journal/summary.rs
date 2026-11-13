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

/// Champs de résultat ajoutés à une commande déjà faite : une ligne écrite avant leur ajout ne
/// les porte pas, et le rejeu ne les compare pas pour elle (`source_details` de
/// `layout.export_pdf`, epic 10, entrée 31). Une ligne qui les porte les compare comme le reste.
const LATE_FIELDS: [&str; 1] = ["source_details"];

/// Le résultat rejoué redonne le résultat gardé, au `layer_id` près. Un résumé
/// se compare par taille et par empreinte du contenu ; un journal plus ancien,
/// dont le résumé n'a pas d'empreinte, ne se compare que par la taille. Une ligne d'avant un
/// champ ajouté (`LATE_FIELDS`) se compare au résultat rejoué sans ce champ : en clair, quand
/// la ligne ne le porte pas ; résumée, quand le résultat sans lui redonne taille et empreinte
/// (le résumé d'une ligne qui le porte a été pris avec lui, et ne se retrouve pas sans lui).
pub(super) fn same_result(value: &Value, kept: &Value) -> bool {
    if same_as_kept(value, kept) {
        return true;
    }
    let mut older = value.clone();
    let Some(map) = older.as_object_mut() else {
        return false;
    };
    let mut removed = false;
    for field in LATE_FIELDS {
        let in_kept = kept.get(field).is_some();
        if !in_kept && map.remove(field).is_some() {
            removed = true;
        }
    }
    removed && same_as_kept(&older, kept)
}

fn same_as_kept(value: &Value, kept: &Value) -> bool {
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
