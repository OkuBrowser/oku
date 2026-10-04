//! This implementation uses [`proc_macro::Diagnostic`], nightly only.

use std::cell::Cell;

use proc_macro::{Diagnostic as PDiag, Level as PLevel};

use crate::{
    abort_now, check_correctness,
    diagnostic::{Diagnostic, Level, SuggestionKind},
};

pub fn abort_if_dirty() {
    check_correctness();
    if IS_DIRTY.with(|c| c.get()) {
        abort_now()
    }
}

pub(crate) fn cleanup() -> Vec<Diagnostic> {
    IS_DIRTY.with(|c| c.set(false));
    vec![]
}

pub(crate) fn emit_diagnostic(diag: Diagnostic) {
    let Diagnostic {
        level,
        start,
        end,
        msg,
        suggestions,
        children,
    } = diag;

    let span = start.join(end).unwrap_or(start);

    let level = match level {
        Level::Warning => PLevel::Warning,
        Level::Error => {
            IS_DIRTY.with(|c| c.set(true));
            PLevel::Error
        }
        _ => unreachable!(),
    };

    let mut res = PDiag::spanned(span.unwrap(), level, msg);

    for (kind, msg, span) in suggestions {
        res = match (kind, span) {
            (SuggestionKind::Note, Some(span)) => res.span_note(span.unwrap(), msg),
            (SuggestionKind::Help, Some(span)) => res.span_help(span.unwrap(), msg),
            (SuggestionKind::Note, None) => res.note(msg),
            (SuggestionKind::Help, None) => res.help(msg),
        }
    }

    for (start, end, msg) in children {
        let span = start.join(end).unwrap_or(start).unwrap();
        res = res.span_error(span, msg);
    }

    res.emit()
}

thread_local! {
    static IS_DIRTY: Cell<bool> = Cell::new(false);
}
