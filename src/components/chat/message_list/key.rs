//! Measurement keys and the entries they are built from.

use std::sync::Arc;

use super::key_snapshot::KeySnapshot;
use super::types::{MessageCompositeMeasureConfig, MessageExpansionKey, MessageVariantKey};
use crate::components::chat::{MessageId, MessageRevision};

/// Everything a measurement is keyed on.
///
/// Equality and hashing go through a bit-pattern snapshot rather than the
/// config's own `PartialEq`, so a key is always equal to itself even when the
/// styles it carries contain NaN. Float `PartialEq` is not reflexive, and a key
/// that did not equal itself would miss its own cache entry on every lookup.
#[derive(Debug, Clone)]
pub struct MessageMeasureKey {
    message_id: MessageId,
    content_revision: MessageRevision,
    variant: MessageVariantKey,
    expansion: MessageExpansionKey,
    config: MessageCompositeMeasureConfig,
    snapshot: KeySnapshot,
}

impl MessageMeasureKey {
    fn new(
        message_id: MessageId,
        content_revision: MessageRevision,
        variant: MessageVariantKey,
        expansion: MessageExpansionKey,
        config: MessageCompositeMeasureConfig,
    ) -> Self {
        let snapshot = KeySnapshot::of(
            message_id,
            content_revision,
            variant.get(),
            expansion.get(),
            &config,
        );
        Self {
            message_id,
            content_revision,
            variant,
            expansion,
            config,
            snapshot,
        }
    }

    /// The message this key measures.
    pub const fn message_id(&self) -> MessageId {
        self.message_id
    }

    /// The content revision the measurement was taken at.
    pub const fn content_revision(&self) -> MessageRevision {
        self.content_revision
    }

    /// The rendering variant.
    pub const fn variant(&self) -> MessageVariantKey {
        self.variant
    }

    /// The expansion state.
    pub const fn expansion(&self) -> MessageExpansionKey {
        self.expansion
    }

    /// The full composite config.
    pub const fn config(&self) -> &MessageCompositeMeasureConfig {
        &self.config
    }
}

impl PartialEq for MessageMeasureKey {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot == other.snapshot
    }
}

impl Eq for MessageMeasureKey {}

impl std::hash::Hash for MessageMeasureKey {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.snapshot.hash(hasher);
    }
}

/// A shared, immutable handle to one measurement key.
///
/// Cloning bumps a refcount. Visible slices hand one of these to the renderer
/// every frame, so deep-copying the source text and styles per slice would make
/// the cost of scrolling scale with how long the messages are rather than with
/// how many are on screen.
#[derive(Debug, Clone)]
pub struct MessageMeasureKeyHandle(Arc<MessageMeasureKey>);

impl MessageMeasureKeyHandle {
    /// Builds a handle from the parts that identify a measurement.
    pub fn new(
        message_id: MessageId,
        content_revision: MessageRevision,
        variant: MessageVariantKey,
        expansion: MessageExpansionKey,
        config: MessageCompositeMeasureConfig,
    ) -> Self {
        Self(Arc::new(MessageMeasureKey::new(
            message_id,
            content_revision,
            variant,
            expansion,
            config,
        )))
    }

    /// Borrows the key. There is no way to obtain a mutable one.
    pub fn as_key(&self) -> &MessageMeasureKey {
        &self.0
    }

    /// Whether two handles point at the same allocation.
    ///
    /// Used to prove a slice's key is the one the state measured with, not an
    /// equal-looking key rebuilt from newer content.
    pub fn is_same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for MessageMeasureKeyHandle {
    fn eq(&self, other: &Self) -> bool {
        self.as_key() == other.as_key()
    }
}

impl Eq for MessageMeasureKeyHandle {}

impl std::hash::Hash for MessageMeasureKeyHandle {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.as_key().hash(hasher);
    }
}

/// One message as the list holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageListEntry {
    message_id: MessageId,
    content_revision: MessageRevision,
    variant: MessageVariantKey,
    expansion: MessageExpansionKey,
    measure_config: MessageCompositeMeasureConfig,
}

impl MessageListEntry {
    /// Builds an entry.
    pub const fn new(
        message_id: MessageId,
        content_revision: MessageRevision,
        variant: MessageVariantKey,
        expansion: MessageExpansionKey,
        measure_config: MessageCompositeMeasureConfig,
    ) -> Self {
        Self {
            message_id,
            content_revision,
            variant,
            expansion,
            measure_config,
        }
    }

    /// The message's identity.
    pub const fn message_id(&self) -> MessageId {
        self.message_id
    }

    /// The content revision this entry describes.
    pub const fn content_revision(&self) -> MessageRevision {
        self.content_revision
    }

    /// The rendering variant.
    pub const fn variant(&self) -> MessageVariantKey {
        self.variant
    }

    /// The expansion state.
    pub const fn expansion(&self) -> MessageExpansionKey {
        self.expansion
    }

    /// The composite measure config.
    pub const fn measure_config(&self) -> &MessageCompositeMeasureConfig {
        &self.measure_config
    }

    /// Builds the measurement key this entry is measured under.
    pub fn measure_key(&self) -> MessageMeasureKeyHandle {
        MessageMeasureKeyHandle::new(
            self.message_id,
            self.content_revision,
            self.variant,
            self.expansion,
            self.measure_config.clone(),
        )
    }
}
