// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl engine binary. Started without arguments it speaks UCI on standard
//! input and output ([`vyvrael::uci::run`]); with arguments it runs one of the
//! commands of [`vyvrael::cli::run`].

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        // The search thread writes too, so the UCI loop takes the output
        // itself instead of a lock held here.
        return match vyvrael::uci::run(io::stdin().lock(), Box::new(io::stdout())) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();
    let status = vyvrael::cli::run(&args, &mut out, &mut err);
    let flushed = out.flush();
    match (status, flushed) {
        (Ok(code), Ok(())) => ExitCode::from(code),
        // Output could not be written (for example a closed pipe).
        _ => ExitCode::FAILURE,
    }
}
