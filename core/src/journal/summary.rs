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

/// Clé réservée du journal : un résumé s'écrit `{"$summary": {truncated, bytes, fingerprint}}`.
/// Aucun résultat de commande ne peut la porter (`$` est exclu des champs Rust), donc un résultat
/// court qui porte lui-même `truncated: true` n'est jamais pris pour un résumé (AD-27, AD-11).
pub(super) const SUMMARY_KEY: &str = "$summary";

/// Marqueurs d'un résumé gardé : la clé réservée, ou la forme des journaux d'avant, exactement
/// `{truncated: true, bytes[, fingerprint]}` à la racine, qui se lit comme avant.
fn kept_summary(kept: &Value) -> Option<&Value> {
    let map = kept.as_object()?;
    // La clé réservée vaut seule, et son contenu est un objet : sinon ce n'est pas un résumé.
    if let Some(inner) = map.get(SUMMARY_KEY) {
        return (map.len() == 1 && inner.is_object()).then_some(inner);
    }
    let legacy = map.get("truncated") == Some(&Value::Bool(true))
        && map.get("bytes").is_some_and(Value::is_u64)
        && map
            .keys()
            .all(|k| matches!(k.as_str(), "truncated" | "bytes" | "fingerprint"));
    legacy.then_some(kept)
}

fn same_as_kept(value: &Value, kept: &Value) -> bool {
    let again = summarize(value);
    if let Some(now) = again.get(SUMMARY_KEY) {
        kept_summary(kept).is_some_and(|old| {
            now.get("bytes") == old.get("bytes")
                && old
                    .get("fingerprint")
                    .is_none_or(|f| Some(f) == now.get("fingerprint"))
        })
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
        json!({SUMMARY_KEY: {
            "truncated": true,
            "bytes": text.len(),
            "fingerprint": fingerprint(&without_layer_id(value)),
        }})
    }
}
