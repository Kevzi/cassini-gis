//! Binaire du programme d'exemple : les quatre points d'entrée de `cassini`, avec le registre
//! du programme composé (`serve`, `mcp`, `run`, `replay`).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cassini::run_program(
        &programme_exemple::program(),
        &args,
        &mut std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(code as u8)
}
