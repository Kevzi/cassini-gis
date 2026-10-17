# Le modèle local sous Windows (ticket 5.1)

Le crate `cassini-ai` lie llama.cpp **dans le processus** (AD-15) : pas de serveur,
pas de socket. La première compilation construit llama.cpp avec CMake.

## Prérequis

- Visual Studio Build Tools 2022 (charge de travail C++) et CMake 4.x.
- libclang (utilisé par `bindgen`) : poser `LIBCLANG_PATH` sur le dossier qui
  contient `libclang.dll`.
- Le fichier `Qwen3.5-9B-Q4_K_M.gguf` (environ 5,4 Go) dans `ai/models/` (hors git).
  Cassini ne le télécharge jamais.

## Variables et chemin court

```powershell
$env:LIBCLANG_PATH = "C:\chemin\vers\libclang\clang\native"
$env:CARGO_TARGET_DIR = "C:\ct"      # chemin COURT, voir ci-dessous
```

CMake échoue (limite MAX_PATH de 260 caractères) si `target/` est sous un chemin
trop long : d'où un `CARGO_TARGET_DIR` court.

## Compiler et essayer

```powershell
cargo build --release -p cassini
cassini model-check --model ai/models/Qwen3.5-9B-Q4_K_M.gguf --prompt "Dis bonjour."
```

- Build à froid en release : environ 1 min 30. `target/` grossit de quelques Go.
- Toujours en `--release` : le profil `dev` rend l'inférence très lente.
- Réponse en environ 7 s sur CPU. Code de sortie 0 ; en cas d'échec, un message
  JSON `{"code":"ai.…"}` sur la sortie d'erreur et un code non nul.

## Essai avec le vrai modèle

```powershell
$env:CASSINI_TEST_MODEL = "$PWD\ai\models\Qwen3.5-9B-Q4_K_M.gguf"
cargo test --release -p cassini-ai
```

Sans cette variable, l'essai est ignoré. Le test `cli/tests/hors_ligne.rs` vérifie
que le binaire n'importe ni `ws2_32` ni `winhttp`. La garde AD-14 :
`cargo install cargo-deny --locked` puis `cargo deny check`.
