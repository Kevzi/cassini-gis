//! Couche IA locale : un modèle GGUF chargé **dans le processus** (AD-15).
//! Aucun serveur, aucune socket, aucun réseau.

use std::fmt;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;

/// Taille du contexte, en jetons.
const N_CTX: u32 = 8192;
/// Borne de la réponse, en jetons.
const MAX_NEW_TOKENS: usize = 256;

/// Échecs de `complete`, tous typés : aucun ne fait paniquer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// La phrase est vide ; refusée avant tout chargement.
    EmptyPrompt,
    /// Le fichier du modèle n'existe pas.
    ModelNotFound(PathBuf),
    /// Le moteur n'a pas pu démarrer.
    Backend(String),
    /// Le fichier n'est pas un modèle GGUF lisible.
    Load(String),
    /// Le contexte d'inférence n'a pas pu être créé.
    Context(String),
    /// La phrase ne tient pas dans le contexte.
    PromptTooLong,
    /// Le décodage a échoué.
    Decode(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::EmptyPrompt => write!(f, "La phrase est vide."),
            AiError::ModelNotFound(p) => {
                write!(f, "Le fichier du modèle est introuvable : {}", p.display())
            }
            AiError::Backend(e) => write!(f, "Le moteur du modèle n'a pas démarré : {e}"),
            AiError::Load(e) => write!(f, "Ce fichier n'est pas un modèle GGUF lisible : {e}"),
            AiError::Context(e) => write!(f, "Le contexte du modèle n'a pas pu être créé : {e}"),
            AiError::PromptTooLong => write!(f, "La phrase est trop longue pour le modèle."),
            AiError::Decode(e) => write!(f, "Le modèle n'a pas pu répondre : {e}"),
        }
    }
}

impl std::error::Error for AiError {}

/// Code stable de l'erreur, pour les sorties machine.
impl AiError {
    pub fn code(&self) -> &'static str {
        match self {
            AiError::EmptyPrompt => "ai.empty_prompt",
            AiError::ModelNotFound(_) => "ai.model_not_found",
            AiError::Backend(_) => "ai.backend",
            AiError::Load(_) => "ai.load",
            AiError::Context(_) => "ai.context",
            AiError::PromptTooLong => "ai.prompt_too_long",
            AiError::Decode(_) => "ai.decode",
        }
    }
}

/// Le moteur ne s'initialise qu'une fois par processus ; ses journaux sont coupés.
fn backend() -> Result<&'static Mutex<LlamaBackend>, AiError> {
    static BACKEND: OnceLock<Result<Mutex<LlamaBackend>, String>> = OnceLock::new();
    BACKEND
        .get_or_init(|| {
            let mut b = LlamaBackend::init().map_err(|e| e.to_string())?;
            b.void_logs();
            Ok(Mutex::new(b))
        })
        .as_ref()
        .map_err(|e| AiError::Backend(e.clone()))
}

/// Invite au format chatml de Qwen, réflexion coupée (comme `harness.py`).
fn build_prompt(user: &str) -> String {
    format!("<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n")
}

/// Charge le modèle `model` et rend le texte de sa réponse à `prompt`.
/// Décodage glouton (température 0), contexte 8192, réponse bornée.
pub fn complete(model: &Path, prompt: &str) -> Result<String, AiError> {
    if prompt.trim().is_empty() {
        return Err(AiError::EmptyPrompt);
    }
    if !model.is_file() {
        return Err(AiError::ModelNotFound(model.to_path_buf()));
    }
    let backend = backend()?.lock().unwrap_or_else(|e| e.into_inner());
    let llama = LlamaModel::load_from_file(&backend, model, &LlamaModelParams::default())
        .map_err(|e| AiError::Load(e.to_string()))?;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(N_CTX))
        .with_n_batch(N_CTX);
    let mut ctx = llama
        .new_context(&backend, ctx_params)
        .map_err(|e| AiError::Context(e.to_string()))?;

    let vocab = llama.vocab();
    let tokens = vocab.tokenize(build_prompt(prompt).as_bytes(), false, true);
    if tokens.is_empty() || tokens.len() + MAX_NEW_TOKENS > N_CTX as usize {
        return Err(AiError::PromptTooLong);
    }

    let decode_err = |e: &dyn fmt::Display| AiError::Decode(e.to_string());
    let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
    let last = tokens.len() - 1;
    for (i, t) in tokens.iter().enumerate() {
        batch
            .add(*t, i as i32, &[0], i == last)
            .map_err(|e| decode_err(&e))?;
    }
    ctx.decode(&mut batch).map_err(|e| decode_err(&e))?;

    let mut sampler = LlamaSampler::greedy();
    let mut bytes: Vec<u8> = Vec::new();
    let mut pos = tokens.len() as i32;
    for _ in 0..MAX_NEW_TOKENS {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if vocab.is_eog(token) {
            break;
        }
        bytes.extend(vocab.token_to_piece(token, false, None));
        batch.clear();
        batch
            .add(token, pos, &[0], true)
            .map_err(|e| decode_err(&e))?;
        pos += 1;
        ctx.decode(&mut batch).map_err(|e| decode_err(&e))?;
    }
    Ok(String::from_utf8_lossy(&bytes).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_prompt_is_refused_before_loading() {
        let e = complete(Path::new("absent.gguf"), "  ").unwrap_err();
        assert_eq!(e, AiError::EmptyPrompt);
    }

    #[test]
    fn missing_model_is_typed() {
        let e = complete(Path::new("absent.gguf"), "Dis bonjour.").unwrap_err();
        assert!(matches!(e, AiError::ModelNotFound(_)));
        assert_eq!(e.code(), "ai.model_not_found");
    }

    #[test]
    fn a_file_that_is_not_gguf_is_a_load_error() {
        let f = std::env::temp_dir().join(format!("cassini-ai-{}.gguf", std::process::id()));
        std::fs::write(&f, b"ceci n'est pas un GGUF").unwrap();
        let e = complete(&f, "Dis bonjour.").unwrap_err();
        let _ = std::fs::remove_file(&f);
        assert!(matches!(e, AiError::Load(_)), "{e:?}");
    }

    #[test]
    fn prompt_ends_with_closed_thinking() {
        assert!(build_prompt("x").ends_with("<think>\n\n</think>\n\n"));
    }
}
