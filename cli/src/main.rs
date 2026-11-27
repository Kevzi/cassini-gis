//! Binaire `cassini` : le programme officiel. Toute la logique est dans la bibliothèque
//! (`cassini::run_program`), que les programmes composés (entrée 2.14) reprennent.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cassini::run(
        &args,
        &mut std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(code as u8)
}
