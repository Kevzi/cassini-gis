//! Mise en forme d'un exemple en prompt court (ticket 5.10, tâche 2).
//!
//! C'est le **seul endroit** qui décide du texte vu par le modèle ajusté : la chaîne Python
//! d'entraînement lit `prompt` et `completion` tels quels et ne met rien en forme. Si deux
//! langages formataient chacun de leur côté, un écart d'un espace suffirait à apprendre au
//! modèle un format qu'il ne verra jamais.
//!
//! Le format est le chatml de Qwen, réflexion coupée, à l'identique de `ai/src/lib.rs`
//! (`turn_head`, `TURN_END`, `ASSISTANT_OPEN`). Le message système est court : rôle, formes
//! de réponse, chaînage, question plutôt que deviner, refus seulement si aucune commande ne
//! sait faire. Ni catalogue ni exemples : le modèle ajusté les a appris.

use cassini_core::plan::Plan;
use serde_json::Value;

/// Consignes du prompt court. Elles condensent les règles de `ai/src/plan.rs` (`SYSTEM`).
pub const SYSTEM: &str = "Tu es l'agent d'un SIG hors ligne. Tu traduis la demande de l'utilisateur en un plan de commandes du registre. \
Réponds uniquement par un objet JSON compact, sans texte autour, de l'une de ces trois formes : \
{\"kind\":\"calls\",\"calls\":[{\"command\":id,\"params\":{...}}]}, \
{\"kind\":\"question\",\"question\":\"<question courte>\"} ou \
{\"kind\":\"refusal\",\"reason\":\"<raison courte>\"}.\n\
Une couche se désigne par son identifiant exact de l'inventaire, la couche produite par l'étape k par \"step:k.layer_id\". \
Un filtre sur un champ vient avant le tampon ou l'intersection. Une distance se traduit par un tampon sur la couche de référence, \
puis une intersection dont la couche cherchée est l'entrée ; plusieurs conditions se croisent par intersections successives. \
N'ajoute ni étape, ni option, ni chemin de sortie que la demande ne réclame pas.\n\
Si un mot de la demande ne désigne aucune couche ni aucun champ de l'inventaire, pose une question au lieu de deviner. \
Refuse seulement si aucune commande ne sait faire l'opération demandée.";

/// Début d'un tour chatml : balise, rôle, retour à la ligne (comme `ai/src/lib.rs`).
fn turn_head(role: &str) -> String {
    format!("<|im_start|>{role}\n")
}
/// Fin d'un tour, retour à la ligne compris.
pub const TURN_END: &str = "<|im_end|>\n";
/// Ouverture du tour assistant, réflexion coupée (`ASSISTANT_OPEN` de `ai/src/lib.rs`).
pub const ASSISTANT_OPEN: &str = "<|im_start|>assistant\n<think>\n\n</think>\n\n";
/// Marque de fin de la réponse attendue : la perte porte aussi sur elle, pour que le modèle
/// apprenne à s'arrêter.
pub const COMPLETION_END: &str = "<|im_end|>";

/// Le message système : consignes courtes, puis l'inventaire du projet, sous la même forme
/// que `inventory_text` de `ai/src/plan.rs`.
pub fn system_text(inventory: &Value) -> String {
    format!("{SYSTEM}\n\nINVENTAIRE DU PROJET :\n{inventory}\n")
}

/// Le texte exact vu par le modèle avant sa réponse : tour système, tour utilisateur, puis
/// ouverture du tour assistant.
pub fn render_prompt(inventory: &Value, request: &str) -> String {
    format!(
        "{}{}{TURN_END}{}{request}{TURN_END}{ASSISTANT_OPEN}",
        turn_head("system"),
        system_text(inventory),
        turn_head("user"),
    )
}

/// La réponse attendue : le plan en JSON compact, `kind` en tête, puis la fin de tour.
pub fn completion(plan: &Plan) -> String {
    let json = serde_json::to_string(plan).expect("un plan se sérialise toujours");
    format!("{json}{COMPLETION_END}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, DEFAULT_COUNT, DEFAULT_SEED};
    use serde_json::json;

    #[test]
    fn the_fixed_part_stays_short() {
        // Quelques centaines de jetons : un jeton fait au moins deux caractères en français.
        let empty = render_prompt(&json!({}), "");
        assert!(
            empty.chars().count() < 1500,
            "{} caractères",
            empty.chars().count()
        );
        // Ni catalogue ni exemples dans la partie fixe.
        assert!(!SYSTEM.contains("CATALOGUE") && !SYSTEM.contains("EXEMPLES"));
    }

    #[test]
    fn the_prompt_is_chatml_with_thinking_turned_off() {
        let inventory = json!({"layers": [{"layer_id": "gares_1"}]});
        let prompt = render_prompt(&inventory, "Tampon de 300 m autour des gares.");
        assert!(prompt.starts_with("<|im_start|>system\nTu es l'agent"));
        assert!(prompt.contains(
            "\nINVENTAIRE DU PROJET :\n{\"layers\":[{\"layer_id\":\"gares_1\"}]}\n<|im_end|>\n"
        ));
        assert!(prompt.ends_with(
            "<|im_start|>user\nTampon de 300 m autour des gares.<|im_end|>\n\
             <|im_start|>assistant\n<think>\n\n</think>\n\n"
        ));
        assert_eq!(prompt.matches("<|im_start|>").count(), 3);
    }

    #[test]
    fn the_output_is_identical_from_one_call_to_the_next() {
        let (examples, _) = generate(DEFAULT_SEED, 200);
        for e in &examples {
            let plan: Plan = serde_json::from_value(e.expected.clone()).unwrap();
            assert_eq!(
                render_prompt(&e.inventory, &e.request),
                render_prompt(&e.inventory, &e.request)
            );
            assert_eq!(completion(&plan), completion(&plan));
        }
    }

    #[test]
    fn every_completion_starts_with_kind_and_reads_back_as_the_expected_plan() {
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for e in &examples {
            let plan: Plan = serde_json::from_value(e.expected.clone()).unwrap();
            let text = completion(&plan);
            assert!(text.starts_with("{\"kind\":\""), "{text}");
            let body = text.strip_suffix(COMPLETION_END).expect("fin de tour");
            let back: Plan = serde_json::from_str(body).unwrap();
            assert_eq!(back, plan);
            let value: Value = serde_json::from_str(body).unwrap();
            assert_eq!(value, e.expected, "le plan relu est le plan attendu");
            // JSON compact, sur une ligne : la forme que la grammaire de `cassini-ai` impose.
            assert!(!body.contains('\n'), "{body}");
            assert!(!body.contains("\": ") && !body.contains("\", \""), "{body}");
        }
    }
}
