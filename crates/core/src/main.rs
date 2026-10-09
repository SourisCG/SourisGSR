//! Binary entry point. Mirrors `src/main.cpp` argument dispatch (v5.10.2).
//!
//! T10 scope: parse + `--help`/`--version` + localized errors (exit 1).
//! `Run`/`Info`/`List*` actions report `err_not_implemented` until their
//! backend phases land (T20-T41). See `.specify/spec-01-core.md`.

use gsr_core::cli::{render_help, render_usage, ParseErr, EXIT_USAGE_ERROR};
use gsr_i18n::{resolve_lang, Catalog};

fn catalog_for(argv: &[String]) -> Catalog {
    let lang_env = std::env::var("LANG").ok();
    Catalog::new(resolve_lang(argv, lang_env.as_deref()))
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    match gsr_core::cli::parse(&argv, std::env::var("LANG").ok().as_deref()) {
        Ok(ok) => {
            for warning in &ok.warnings {
                eprintln!("{warning}");
            }
            match ok.action {
                gsr_core::cli::Action::Help => {
                    print!("{}", render_help(&catalog_for(&argv)));
                }
                gsr_core::cli::Action::Version => {
                    println!("{}", env!("CARGO_PKG_VERSION"));
                }
                _ => {
                    let catalog = catalog_for(&argv);
                    eprintln!(
                        "{}: {}",
                        catalog.get("prefix_error"),
                        gsr_i18n::fill(
                            &catalog.get("err_not_implemented"),
                            &[("what", "recording")]
                        )
                    );
                    std::process::exit(EXIT_USAGE_ERROR);
                }
            }
        }
        Err(ParseErr { message, full_help }) => {
            let catalog = catalog_for(&argv);
            if !message.is_empty() {
                eprintln!("{message}");
            }
            if full_help {
                print!("{}", render_help(&catalog));
            } else {
                print!("{}", render_usage(&catalog));
            }
            std::process::exit(EXIT_USAGE_ERROR);
        }
    }
}
