use crate::abort_now;
use proc_macro2::Span;
use proc_macro2::TokenStream;

use quote::{quote_spanned, ToTokens};

/// Represents a diagnostic level
///
/// # Warnings
///
/// Warnings are ignored on stable/beta
#[derive(Debug, PartialEq)]
pub enum Level {
    Error,
    Warning,
    #[doc(hidden)]
    NonExhaustive,
}

/// Represents a single diagnostic message
#[derive(Debug)]
pub struct Diagnostic {
    pub(crate) level: Level,
    pub(crate) start: Span,
    pub(crate) end: Span,
    pub(crate) msg: String,
    pub(crate) suggestions: Vec<(SuggestionKind, String, Option<Span>)>,
    pub(crate) children: Vec<(Span, Span, String)>,
}

impl Diagnostic {
    /// Create a new diagnostic message that points to `Span::call_site()`
    pub fn new(level: Level, message: String) -> Self {
        Diagnostic::spanned(Span::call_site(), level, message)
    }

    /// Create a new diagnostic message that points to the `span`
    pub fn spanned(span: Span, level: Level, message: String) -> Self {
        Diagnostic::double_spanned(span, span, level, message)
    }

    /// Add another error message to self such that it will be emitted right after
    /// the main message.
    pub fn span_error(self, span: Span, msg: String) -> Self {
        self.double_span_error(span, span, msg)
    }

    /// Attach a "help" note to your main message, the note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_help(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Help, msg, Some(span)));
        self
    }

    /// Attach a "help" note to your main message.
    pub fn help(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Help, msg, None));
        self
    }

    /// Attach a note to your main message, the note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_note(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Note, msg, Some(span)));
        self
    }

    /// Attach a note to your main message
    pub fn note(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Note, msg, None));
        self
    }

    /// The message of main warning/error (no notes attached)
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Abort the proc-macro's execution and display the diagnostic.
    ///
    /// # Warnings
    ///
    /// Warnings do not get emitted on stable/beta but this function will abort anyway.
    pub fn abort(self) -> ! {
        self.emit();
        abort_now()
    }

    /// Display the diagnostic while not aborting macro execution.
    ///
    /// # Warnings
    ///
    /// Warnings are ignored on stable/beta
    pub fn emit(self) {
        crate::imp::emit_diagnostic(self);
    }
}

/// **NOT PUBLIC API! NOTHING TO SEE HERE!!!**
#[doc(hidden)]
impl Diagnostic {
    pub fn double_spanned(start: Span, end: Span, level: Level, message: String) -> Self {
        Diagnostic {
            level,
            start,
            end,
            msg: message,
            suggestions: vec![],
            children: vec![],
        }
    }

    pub fn double_span_error(mut self, start: Span, end: Span, msg: String) -> Self {
        self.children.push((start, end, msg));
        self
    }

    pub fn span_suggestion(self, span: Span, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.span_help(span, msg),
            _ => self.span_note(span, msg),
        }
    }

    pub fn suggestion(self, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.help(msg),
            _ => self.note(msg),
        }
    }
}

impl ToTokens for Diagnostic {
    fn to_tokens(&self, ts: &mut TokenStream) {
        use std::borrow::Cow;

        fn ensure_lf(buf: &mut String, s: &str) {
            if s.ends_with('\n') {
                buf.push_str(s);
            } else {
                buf.push_str(s);
                buf.push('\n');
            }
        }

        fn diag_to_tokens(
            start: Span,
            end: Span,
            level: &Level,
            msg: &str,
            suggestions: &[(SuggestionKind, String, Option<Span>)],
        ) -> TokenStream {
            if *level == Level::Warning {
                return TokenStream::new();
            }

            let message = if suggestions.is_empty() {
                Cow::Borrowed(msg)
            } else {
                let mut message = String::new();
                ensure_lf(&mut message, msg);
                message.push('\n');

                for (kind, note, _span) in suggestions {
                    message.push_str("  = ");
                    message.push_str(kind.name());
                    message.push_str(": ");
                    ensure_lf(&mut message, note);
                }
                message.push('\n');

                Cow::Owned(message)
            };

            let msg = syn::LitStr::new(&*message, end);
            let group = quote_spanned!(end=> { #msg } );
            quote_spanned!(start=> compile_error!#group)
        }

        ts.extend(diag_to_tokens(
            self.start,
            self.end,
            &self.level,
            &self.msg,
            &self.suggestions,
        ));
        ts.extend(
            self.children
                .iter()
                .map(|(start, end, msg)| diag_to_tokens(*start, *end, &Level::Error, &msg, &[])),
        );
    }
}

#[derive(Debug)]
pub(crate) enum SuggestionKind {
    Help,
    Note,
}

impl SuggestionKind {
    fn name(&self) -> &'static str {
        match self {
            SuggestionKind::Note => "note",
            SuggestionKind::Help => "help",
        }
    }
}

impl From<syn::Error> for Diagnostic {
    fn from(err: syn::Error) -> Self {
        use proc_macro2::TokenTree;

        fn gut_error(ts: &mut impl Iterator<Item = TokenTree>) -> Option<(Span, Span, String)> {
            let start = match ts.next() {
                // compile_error
                None => return None,
                Some(tt) => tt.span(),
            };
            ts.next().unwrap(); // !

            let lit = match ts.next().unwrap() {
                TokenTree::Group(group) => match group.stream().into_iter().next().unwrap() {
                    TokenTree::Literal(lit) => lit,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            };

            let end = lit.span();
            let mut msg = lit.to_string();
            msg.pop();
            msg.remove(0);

            Some((start, end, msg))
        }

        let mut ts = err.to_compile_error().into_iter();

        let (start, end, msg) = gut_error(&mut ts).unwrap();
        let mut res = Diagnostic::double_spanned(start, end, Level::Error, msg);

        while let Some((start, end, msg)) = gut_error(&mut ts) {
            res = res.double_span_error(start, end, msg);
        }

        res
    }
}
