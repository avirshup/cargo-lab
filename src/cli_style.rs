#![allow(unused)]
/// Taken directly from cargo,
/// https://github.com/rust-lang/cargo/blob/a357df4/crates/cargo-util-terminal/src/style.rs#L4
/// MIT license.
use anstyle::*;

pub const NOP: Style = Style::new();
pub const HEADER: Style = AnsiColor::BrightGreen.on_default().effects(Effects::BOLD);
pub const USAGE: Style = AnsiColor::BrightGreen.on_default().effects(Effects::BOLD);
pub const LITERAL: Style = AnsiColor::BrightCyan.on_default().effects(Effects::BOLD);
pub const PLACEHOLDER: Style = AnsiColor::Cyan.on_default();
pub const ERROR: Style = annotate_snippets::renderer::DEFAULT_ERROR_STYLE;
pub const WARN: Style = annotate_snippets::renderer::DEFAULT_WARNING_STYLE;
pub const NOTE: Style = annotate_snippets::renderer::DEFAULT_NOTE_STYLE;
pub const GOOD: Style = AnsiColor::BrightGreen.on_default().effects(Effects::BOLD);
pub const VALID: Style = AnsiColor::BrightCyan.on_default().effects(Effects::BOLD);
pub const INVALID: Style = annotate_snippets::renderer::DEFAULT_WARNING_STYLE;
pub const TRANSIENT: Style = annotate_snippets::renderer::DEFAULT_HELP_STYLE;
pub const CONTEXT: Style = annotate_snippets::renderer::DEFAULT_CONTEXT_STYLE;

pub const UPDATE_ADDED: Style = NOTE;
pub const UPDATE_REMOVED: Style = ERROR;
pub const UPDATE_UPGRADED: Style = GOOD;
pub const UPDATE_DOWNGRADED: Style = WARN;
pub const UPDATE_UNCHANGED: Style = Style::new().bold();

pub const DEP_NORMAL: Style = Style::new().effects(Effects::DIMMED);
pub const DEP_BUILD: Style = AnsiColor::Blue.on_default().effects(Effects::BOLD);
pub const DEP_DEV: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
pub const DEP_FEATURE: Style = AnsiColor::Magenta.on_default().effects(Effects::DIMMED);
