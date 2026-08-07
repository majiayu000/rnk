//! Validated scrollback content and its terminal transport encoding.
//!
//! Content crosses two representations and the distinction is load-bearing:
//!
//! * **Canonical** text uses `\n` alone, is what the identity digest is taken
//!   over, and is what a durable store persists.
//! * **Transport** bytes are what the terminal receives: every canonical `\n`
//!   becomes `\r\n`, because a terminal in raw mode does not return the carriage
//!   on a bare line feed and the transcript would stair-step.
//!
//! Counting them separately is what lets a partial write be classified honestly.
//! "Twelve bytes accepted" means nothing without knowing which stream it counted.

use std::fmt;

use super::identity::{ProjectionContext, ScrollbackContentIdentity};

/// The SGR reset written after every commit body.
///
/// Written unconditionally rather than only after styled content: the cost is
/// four bytes, and the alternative is a style leaking from a committed line into
/// the live region below it, which is invisible until it is wrong.
const SGR_RESET: &[u8] = b"\x1b[0m";

/// The line delimiter closing a commit.
const COMMIT_DELIMITER: &[u8] = b"\r\n";

/// Text that has been proven safe to write into the terminal's own scrollback.
///
/// Once these bytes are written the library can no longer address them, so the
/// checks here are the last place a malformed transcript can be stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackContent {
    canonical: String,
    identity: ScrollbackContentIdentity,
    context: ProjectionContext,
}

impl ScrollbackContent {
    /// Validates canonical text against the scrollback safety boundary.
    ///
    /// Accepted: printable Unicode and `\n`. Rejected: every other C0 control,
    /// `DEL`, the C1 range, and any content without a printable character —
    /// a commit that renders as nothing would occupy an identity and a ledger
    /// slot while proving nothing about the terminal.
    ///
    /// `ESC` is rejected along with the rest, so this layer carries no styling.
    /// The transport still emits its reset unconditionally, which means adding a
    /// styled-content allowlist later cannot change what already-committed
    /// content does to the region below it.
    pub fn try_new(
        canonical: impl Into<String>,
        context: ProjectionContext,
    ) -> Result<Self, ScrollbackContentError> {
        let canonical = canonical.into();
        let mut has_printable = false;
        for (index, character) in canonical.char_indices() {
            match classify(character) {
                CharacterClass::Printable => has_printable = true,
                CharacterClass::LineFeed => {}
                CharacterClass::Whitespace => {}
                CharacterClass::Forbidden(kind) => {
                    return Err(ScrollbackContentError::ForbiddenControl {
                        kind,
                        byte_offset: index,
                    });
                }
            }
        }
        if !has_printable {
            return Err(ScrollbackContentError::NothingToCommit);
        }
        let identity = ScrollbackContentIdentity::derive(&canonical, context);
        Ok(Self {
            canonical,
            identity,
            context,
        })
    }

    /// Returns the canonical text.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Returns the content identity derived at construction.
    pub const fn identity(&self) -> ScrollbackContentIdentity {
        self.identity
    }

    /// Returns the projection context this content was validated under.
    pub const fn context(&self) -> ProjectionContext {
        self.context
    }

    /// Encodes the content into the three ordered transport stages.
    pub fn encode(&self) -> TransportEncoding {
        let mut body = Vec::with_capacity(self.canonical.len() + 8);
        for byte in self.canonical.bytes() {
            if byte == b'\n' {
                body.push(b'\r');
            }
            body.push(byte);
        }
        TransportEncoding { body }
    }
}

/// The ordered byte stages of one commit.
///
/// A commit is body, then reset, then delimiter. Each stage is reported
/// separately so a write that stops between them can say where it stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEncoding {
    body: Vec<u8>,
}

impl TransportEncoding {
    /// Returns the CRLF-encoded content body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Returns the SGR reset stage.
    pub const fn reset(&self) -> &'static [u8] {
        SGR_RESET
    }

    /// Returns the commit delimiter stage.
    pub const fn delimiter(&self) -> &'static [u8] {
        COMMIT_DELIMITER
    }

    /// Returns every stage in write order.
    pub fn stages(&self) -> [(TransportStage, &[u8]); 3] {
        [
            (TransportStage::Body, &self.body),
            (TransportStage::Reset, SGR_RESET),
            (TransportStage::Delimiter, COMMIT_DELIMITER),
        ]
    }

    /// Returns the total transport byte count across all stages.
    pub fn total_len(&self) -> usize {
        self.body.len() + SGR_RESET.len() + COMMIT_DELIMITER.len()
    }
}

/// Which stage of a commit's transport a byte count refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportStage {
    /// The CRLF-encoded content itself.
    Body,
    /// The trailing SGR reset.
    Reset,
    /// The commit delimiter.
    Delimiter,
}

impl fmt::Display for TransportStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Body => "content body",
            Self::Reset => "style reset",
            Self::Delimiter => "commit delimiter",
        })
    }
}

/// The safety category of a rejected character.
///
/// Only the category and its offset are reported. The character itself is
/// withheld so a rejection can be logged next to untrusted transcript text
/// without echoing back the sequence that was being smuggled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ForbiddenControlKind {
    /// `ESC`, which begins cursor movement, OSC, title and clipboard sequences.
    Escape,
    /// A bare `CR`, which would move the cursor without advancing a line.
    CarriageReturn,
    /// Any other C0 control.
    C0Control,
    /// `DEL`.
    Delete,
    /// A C1 control, reachable through UTF-8 as well as through 8-bit input.
    C1Control,
}

impl fmt::Display for ForbiddenControlKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Escape => "an escape sequence introducer",
            Self::CarriageReturn => "a bare carriage return",
            Self::C0Control => "a C0 control character",
            Self::Delete => "a delete control character",
            Self::C1Control => "a C1 control character",
        })
    }
}

/// Every way content can fail the scrollback safety boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollbackContentError {
    /// A forbidden control character was found at a byte offset.
    ForbiddenControl {
        /// The safety category of the rejected character.
        kind: ForbiddenControlKind,
        /// Its byte offset within the canonical text.
        byte_offset: usize,
    },
    /// The content held no printable character.
    NothingToCommit,
}

impl fmt::Display for ScrollbackContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForbiddenControl { kind, byte_offset } => write!(
                f,
                "scrollback content contains {kind} at byte offset {byte_offset}"
            ),
            Self::NothingToCommit => f.write_str(
                "scrollback content has no printable character, so committing it would prove nothing",
            ),
        }
    }
}

impl std::error::Error for ScrollbackContentError {}

enum CharacterClass {
    Printable,
    LineFeed,
    Whitespace,
    Forbidden(ForbiddenControlKind),
}

fn classify(character: char) -> CharacterClass {
    match character {
        '\n' => CharacterClass::LineFeed,
        '\r' => CharacterClass::Forbidden(ForbiddenControlKind::CarriageReturn),
        '\x1b' => CharacterClass::Forbidden(ForbiddenControlKind::Escape),
        '\x7f' => CharacterClass::Forbidden(ForbiddenControlKind::Delete),
        ' ' | '\t' => CharacterClass::Whitespace,
        character if (character as u32) < 0x20 => {
            CharacterClass::Forbidden(ForbiddenControlKind::C0Control)
        }
        character if (0x80..=0x9f).contains(&(character as u32)) => {
            CharacterClass::Forbidden(ForbiddenControlKind::C1Control)
        }
        character if character.is_whitespace() => CharacterClass::Whitespace,
        _ => CharacterClass::Printable,
    }
}

#[cfg(test)]
mod tests {
    use super::super::identity::ThemeIdentity;
    use super::*;

    fn context() -> ProjectionContext {
        ProjectionContext::new(80, ThemeIdentity::new(0)).expect("valid")
    }

    fn content(text: &str) -> Result<ScrollbackContent, ScrollbackContentError> {
        ScrollbackContent::try_new(text, context())
    }

    #[test]
    fn printable_text_is_accepted() {
        let value = content("hello world").expect("accepted");
        assert_eq!(value.canonical(), "hello world");
    }

    #[test]
    fn line_feeds_are_the_only_accepted_control() {
        assert!(content("first\nsecond").is_ok());
    }

    #[test]
    fn an_escape_sequence_is_rejected_by_category_not_by_content() {
        let error = content("safe\x1b]0;title\x07").expect_err("rejected");
        assert_eq!(
            error,
            ScrollbackContentError::ForbiddenControl {
                kind: ForbiddenControlKind::Escape,
                byte_offset: 4,
            }
        );
        let rendered = error.to_string();
        assert!(!rendered.contains("title"), "rendered as: {rendered}");
    }

    #[test]
    fn a_bare_carriage_return_is_rejected() {
        let error = content("over\rwrite").expect_err("rejected");
        assert!(matches!(
            error,
            ScrollbackContentError::ForbiddenControl {
                kind: ForbiddenControlKind::CarriageReturn,
                ..
            }
        ));
    }

    #[test]
    fn other_c0_controls_are_rejected() {
        let error = content("bell\x07").expect_err("rejected");
        assert!(matches!(
            error,
            ScrollbackContentError::ForbiddenControl {
                kind: ForbiddenControlKind::C0Control,
                ..
            }
        ));
    }

    #[test]
    fn delete_is_rejected() {
        assert!(matches!(
            content("x\x7f").expect_err("rejected"),
            ScrollbackContentError::ForbiddenControl {
                kind: ForbiddenControlKind::Delete,
                ..
            }
        ));
    }

    #[test]
    fn c1_controls_are_rejected_even_when_they_arrive_as_utf8() {
        // U+009B is the 8-bit CSI introducer and is two bytes in UTF-8, so a
        // byte-oriented check that only looks below 0x20 would miss it.
        assert!(matches!(
            content("x\u{009b}").expect_err("rejected"),
            ScrollbackContentError::ForbiddenControl {
                kind: ForbiddenControlKind::C1Control,
                ..
            }
        ));
    }

    #[test]
    fn empty_content_is_rejected() {
        assert_eq!(content(""), Err(ScrollbackContentError::NothingToCommit));
    }

    #[test]
    fn whitespace_only_content_is_rejected() {
        assert_eq!(
            content("  \n\t \n"),
            Err(ScrollbackContentError::NothingToCommit)
        );
    }

    #[test]
    fn canonical_line_feeds_become_crlf_in_transport() {
        let encoded = content("a\nb").expect("accepted").encode();
        assert_eq!(encoded.body(), b"a\r\nb");
    }

    #[test]
    fn transport_stages_are_body_then_reset_then_delimiter() {
        let encoded = content("x").expect("accepted").encode();
        let stages = encoded.stages();
        assert_eq!(stages[0].0, TransportStage::Body);
        assert_eq!(stages[0].1, b"x");
        assert_eq!(stages[1].0, TransportStage::Reset);
        assert_eq!(stages[1].1, b"\x1b[0m");
        assert_eq!(stages[2].0, TransportStage::Delimiter);
        assert_eq!(stages[2].1, b"\r\n");
    }

    #[test]
    fn total_transport_length_counts_every_stage() {
        let encoded = content("a\nb").expect("accepted").encode();
        // "a\r\nb" is four bytes, reset is four, delimiter is two.
        assert_eq!(encoded.total_len(), 10);
        assert_eq!(
            encoded.total_len(),
            encoded
                .stages()
                .iter()
                .map(|(_, bytes)| bytes.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn transport_length_differs_from_canonical_length_when_lines_wrap() {
        let value = content("a\nb").expect("accepted");
        assert_eq!(value.canonical().len(), 3);
        assert_eq!(value.encode().body().len(), 4);
    }

    #[test]
    fn identity_is_derived_at_construction_and_tracks_the_context() {
        let narrow = ScrollbackContent::try_new("same", context()).expect("accepted");
        let wide_context = ProjectionContext::new(120, ThemeIdentity::new(0)).expect("valid");
        let wide = ScrollbackContent::try_new("same", wide_context).expect("accepted");
        assert_ne!(narrow.identity(), wide.identity());
    }
}
