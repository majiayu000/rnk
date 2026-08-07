//! Stable identity for one committable scrollback message.
//!
//! A commit identity answers "is this the same commit I already made?" without
//! ever carrying the message's bytes. The bytes live in [`ScrollbackContent`];
//! what travels through receipts, ledgers, errors and `Debug` output is only the
//! identity, so a transcript's text cannot leak through an audit trail.
//!
//! [`ScrollbackContent`]: super::ScrollbackContent

use std::fmt;

use super::digest::ContentDigest;
use crate::components::chat::{MessageId, MessageRevision};

/// The store or conversation scope a commit belongs to.
///
/// Two commits in different namespaces never deduplicate against each other and
/// never conflict, even when every other field matches. This is what lets one
/// process host several conversations against a shared durable sink.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScrollbackNamespace(String);

impl ScrollbackNamespace {
    /// Creates a namespace from a non-empty caller-stable string.
    ///
    /// The value must survive a restart unchanged: a namespace derived from a
    /// process ID or a random per-run token silently disables durable dedup.
    pub fn new(value: impl Into<String>) -> Result<Self, ScrollbackIdentityError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ScrollbackIdentityError::EmptyNamespace);
        }
        Ok(Self(value))
    }

    /// Returns the namespace text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScrollbackNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The render environment a message was projected under, frozen at staging time.
///
/// Freezing matters because a committed line cannot be re-flowed: once bytes are
/// in the terminal's own scrollback the library can no longer address them. A
/// candidate that was staged at width 80 must stay comparable against its
/// original projection even after the terminal is resized, so the context is
/// part of the identity rather than read from the live environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionContext {
    width: u16,
    theme: ThemeIdentity,
}

impl ProjectionContext {
    /// Creates a projection context from a non-zero terminal width and a theme.
    pub fn new(width: u16, theme: ThemeIdentity) -> Result<Self, ScrollbackIdentityError> {
        if width == 0 {
            return Err(ScrollbackIdentityError::ZeroWidth);
        }
        Ok(Self { width, theme })
    }

    /// Returns the frozen width in terminal cells.
    pub const fn width(self) -> u16 {
        self.width
    }

    /// Returns the frozen theme identity.
    pub const fn theme(self) -> ThemeIdentity {
        self.theme
    }

    fn absorb(self, builder: super::digest::DigestBuilder) -> super::digest::DigestBuilder {
        builder
            .field_u64(u64::from(self.width))
            .field_u64(self.theme.0)
    }
}

/// An opaque handle for the theme a projection was rendered under.
///
/// The shell never interprets this value; it only requires that a caller which
/// changes visible styling also changes the identity, so a restyled message is
/// not mistaken for one already committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThemeIdentity(u64);

impl ThemeIdentity {
    /// Creates a theme identity from a caller-stable token.
    pub const fn new(token: u64) -> Self {
        Self(token)
    }

    /// Returns the underlying token.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The complete identity of one scrollback commit.
///
/// Equality over the whole value is what decides dedup. Equality over the
/// [`ScrollbackCommitKey`] alone — namespace, message and revision — is what
/// decides *conflict*: a second commit under the same key with a different
/// digest or context means the caller changed committed content, which no sink
/// can honour once the bytes have left.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScrollbackCommitId {
    key: ScrollbackCommitKey,
    content: ScrollbackContentIdentity,
    context: ProjectionContext,
}

impl ScrollbackCommitId {
    /// Assembles a commit identity from its parts.
    pub fn new(
        key: ScrollbackCommitKey,
        content: ScrollbackContentIdentity,
        context: ProjectionContext,
    ) -> Self {
        Self {
            key,
            content,
            context,
        }
    }

    /// Returns the dedup key: namespace, message and terminal revision.
    pub fn key(&self) -> &ScrollbackCommitKey {
        &self.key
    }

    /// Returns the content identity frozen at first staging.
    pub const fn content(&self) -> ScrollbackContentIdentity {
        self.content
    }

    /// Returns the frozen projection context.
    pub const fn context(&self) -> ProjectionContext {
        self.context
    }

    /// Reports whether `other` is the same commit observed again.
    pub fn is_same_commit(&self, other: &Self) -> bool {
        self == other
    }

    /// Reports whether `other` claims this key while disagreeing about content.
    ///
    /// This is the condition that must fail closed: the key names a line that
    /// may already be in the terminal, and the disagreement means the caller
    /// believes it holds different bytes for it.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.key == other.key && (self.content != other.content || self.context != other.context)
    }
}

/// The part of a commit identity that names *which line*, ignoring its content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScrollbackCommitKey {
    namespace: ScrollbackNamespace,
    message_id: MessageId,
    revision: MessageRevision,
}

impl ScrollbackCommitKey {
    /// Creates a commit key.
    pub const fn new(
        namespace: ScrollbackNamespace,
        message_id: MessageId,
        revision: MessageRevision,
    ) -> Self {
        Self {
            namespace,
            message_id,
            revision,
        }
    }

    /// Returns the namespace.
    pub const fn namespace(&self) -> &ScrollbackNamespace {
        &self.namespace
    }

    /// Returns the message identity.
    pub const fn message_id(&self) -> MessageId {
        self.message_id
    }

    /// Returns the terminal revision this commit was taken at.
    pub const fn revision(&self) -> MessageRevision {
        self.revision
    }
}

impl fmt::Display for ScrollbackCommitKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/message {}@{}",
            self.namespace,
            self.message_id.get(),
            self.revision.get()
        )
    }
}

/// A content-derived value that is equal exactly when the content is unchanged.
///
/// Derived through [`ContentDigest`], so it never contains the content itself
/// and is safe to place in receipts, ledgers, error messages and audit records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScrollbackContentIdentity(ContentDigest);

impl ScrollbackContentIdentity {
    /// Derives an identity from canonical content bytes and its context.
    ///
    /// The context participates so that the same text projected at a different
    /// width is a different identity — which is exactly what a resize produces,
    /// and exactly what must not silently reuse an earlier commit.
    pub fn derive(canonical: &str, context: ProjectionContext) -> Self {
        let builder = context.absorb(ContentDigest::builder());
        Self(builder.field(canonical.as_bytes()).finish())
    }

    /// Returns the underlying digest.
    pub const fn digest(self) -> ContentDigest {
        self.0
    }
}

impl fmt::Display for ScrollbackContentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Every way a commit identity can fail to be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollbackIdentityError {
    /// A namespace was empty, which would collapse distinct stores together.
    EmptyNamespace,
    /// A projection width of zero cannot describe any rendered line.
    ZeroWidth,
}

impl fmt::Display for ScrollbackIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNamespace => {
                f.write_str("scrollback namespace must not be empty: distinct stores would merge")
            }
            Self::ZeroWidth => f.write_str(
                "projection width must be greater than zero to describe a rendered line",
            ),
        }
    }
}

impl std::error::Error for ScrollbackIdentityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(width: u16) -> ProjectionContext {
        ProjectionContext::new(width, ThemeIdentity::new(1)).expect("non-zero width")
    }

    fn key(namespace: &str, id: u64, revision: u64) -> ScrollbackCommitKey {
        ScrollbackCommitKey::new(
            ScrollbackNamespace::new(namespace).expect("non-empty"),
            MessageId::new(id),
            MessageRevision::new(revision).expect("non-zero"),
        )
    }

    fn commit(
        namespace: &str,
        id: u64,
        revision: u64,
        text: &str,
        width: u16,
    ) -> ScrollbackCommitId {
        let context = context(width);
        ScrollbackCommitId::new(
            key(namespace, id, revision),
            ScrollbackContentIdentity::derive(text, context),
            context,
        )
    }

    #[test]
    fn an_empty_namespace_is_rejected() {
        assert_eq!(
            ScrollbackNamespace::new(""),
            Err(ScrollbackIdentityError::EmptyNamespace)
        );
    }

    #[test]
    fn a_zero_width_projection_is_rejected() {
        assert_eq!(
            ProjectionContext::new(0, ThemeIdentity::new(0)),
            Err(ScrollbackIdentityError::ZeroWidth)
        );
    }

    #[test]
    fn the_same_message_observed_twice_is_the_same_commit() {
        let first = commit("store", 1, 1, "hello", 80);
        let second = commit("store", 1, 1, "hello", 80);
        assert!(first.is_same_commit(&second));
        assert!(!first.conflicts_with(&second));
    }

    #[test]
    fn the_same_key_with_different_content_conflicts() {
        let first = commit("store", 1, 1, "hello", 80);
        let second = commit("store", 1, 1, "goodbye", 80);
        assert!(!first.is_same_commit(&second));
        assert!(first.conflicts_with(&second));
    }

    #[test]
    fn the_same_key_and_text_at_a_different_width_conflicts() {
        // A resize reprojects; committing the reprojection under the old key
        // would claim to rewrite a line the terminal already owns.
        let first = commit("store", 1, 1, "hello", 80);
        let second = commit("store", 1, 1, "hello", 120);
        assert!(first.conflicts_with(&second));
    }

    #[test]
    fn a_different_theme_is_a_different_content_identity() {
        let narrow = ProjectionContext::new(80, ThemeIdentity::new(1)).expect("valid");
        let restyled = ProjectionContext::new(80, ThemeIdentity::new(2)).expect("valid");
        assert_ne!(
            ScrollbackContentIdentity::derive("hello", narrow),
            ScrollbackContentIdentity::derive("hello", restyled)
        );
    }

    #[test]
    fn different_namespaces_neither_dedupe_nor_conflict() {
        let left = commit("store-a", 1, 1, "hello", 80);
        let right = commit("store-b", 1, 1, "hello", 80);
        assert!(!left.is_same_commit(&right));
        assert!(!left.conflicts_with(&right));
    }

    #[test]
    fn different_namespaces_with_different_content_still_do_not_conflict() {
        let left = commit("store-a", 1, 1, "hello", 80);
        let right = commit("store-b", 1, 1, "goodbye", 80);
        assert!(!left.conflicts_with(&right));
    }

    #[test]
    fn a_later_revision_is_a_separate_commit_rather_than_a_conflict() {
        let first = commit("store", 1, 1, "hello", 80);
        let second = commit("store", 1, 2, "hello, again", 80);
        assert!(!first.is_same_commit(&second));
        assert!(!first.conflicts_with(&second));
    }

    #[test]
    fn identity_display_never_reveals_content() {
        let id = commit("store", 1, 1, "a secret token", 80);
        let rendered = format!("{:?} {} {}", id, id.key(), id.content());
        assert!(!rendered.contains("secret"), "rendered as: {rendered}");
    }
}
