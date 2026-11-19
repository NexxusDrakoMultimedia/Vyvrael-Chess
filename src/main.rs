// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl engine binary: a thin wrapper around [`vyvrael::cli::run`].

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
