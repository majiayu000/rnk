//! The digest behind every scrollback content identity.
//!
//! # Why this is not SHA-256
//!
//! A scrollback digest answers exactly one question: *have I already committed
//! this exact content under this exact identity?* The adversary in that question
//! is a duplicated terminal event or a message re-rendered at a new width — not
//! someone searching for a preimage. A 128-bit non-cryptographic digest settles
//! it, and settling it in-tree keeps a terminal UI library free of a hash
//! dependency it would otherwise carry forever.
//!
//! # What makes it safe to rely on
//!
//! Two properties, and the first matters more than the hash:
//!
//! 1. **Length framing.** Every field is absorbed as `len(u64 le) || bytes`, so
//!    distinct field tuples always produce distinct byte streams. Without it,
//!    `("ab", "c")` and `("a", "bc")` concatenate identically and collide no
//!    matter how strong the hash is. With it, collision resistance is purely a
//!    property of the digest, which is the only place it can be reasoned about.
//! 2. **A versioned domain tag.** Every digest starts from a constant naming its
//!    scheme. Replacing the scheme changes the tag, so an identity computed
//!    under a future scheme can never compare equal to one computed under this
//!    one — a mismatch surfaces as a typed conflict rather than as a silent
//!    false match.
//!
//! FNV-1a-128 is specified outside this crate and has no internal state to drift,
//! so a digest computed here is reproducible across processes, architectures and
//! crate versions. [`std::hash::DefaultHasher`] is explicitly not, which is why
//! it is unusable for an identity that a durable sink may persist.
//!
//! A sink that genuinely needs preimage resistance — one publishing identities
//! across a trust boundary — should carry its own digest and treat this one as
//! the framework's internal dedup key.

use std::fmt;

/// The domain tag absorbed before any caller-supplied field.
///
/// Bump the trailing version whenever the framing or the digest changes.
const DOMAIN_TAG: &[u8] = b"rnk.scrollback.content.v1";

/// FNV-1a-128 offset basis, per the FNV specification.
const OFFSET_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;

/// FNV-1a-128 prime, per the FNV specification.
const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// An unambiguous 128-bit digest over an ordered sequence of byte fields.
///
/// Construct one with [`ContentDigest::builder`], absorb each field in a fixed
/// order, then [`DigestBuilder::finish`].
///
/// ```
/// use rnk::components::chat::ContentDigest;
///
/// let split = ContentDigest::builder().field(b"ab").field(b"c").finish();
/// let joined = ContentDigest::builder().field(b"a").field(b"bc").finish();
/// assert_ne!(split, joined, "length framing keeps field boundaries significant");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentDigest([u8; 16]);

impl ContentDigest {
    /// Starts a digest seeded with the scheme's domain tag.
    pub fn builder() -> DigestBuilder {
        let mut builder = DigestBuilder {
            state: OFFSET_BASIS,
        };
        builder.absorb_framed(DOMAIN_TAG);
        builder
    }

    /// Returns the digest as raw big-endian bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Returns the lowercase hexadecimal rendering used in logs and receipts.
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(32);
        for byte in self.0 {
            // Two lowercase hex digits per byte, fixed width, no separators.
            out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
            out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
        }
        out
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Accumulates length-framed fields into a [`ContentDigest`].
#[derive(Debug, Clone)]
pub struct DigestBuilder {
    state: u128,
}

impl DigestBuilder {
    /// Absorbs one field, keeping its boundary significant.
    #[must_use]
    pub fn field(mut self, bytes: &[u8]) -> Self {
        self.absorb_framed(bytes);
        self
    }

    /// Absorbs one `u64` field, encoded little-endian.
    #[must_use]
    pub fn field_u64(self, value: u64) -> Self {
        self.field(&value.to_le_bytes())
    }

    /// Finishes the digest.
    pub fn finish(self) -> ContentDigest {
        ContentDigest(self.state.to_be_bytes())
    }

    fn absorb_framed(&mut self, bytes: &[u8]) {
        // The length prefix is what makes the field boundary recoverable, so it
        // is absorbed before the payload and is never omitted for empty fields.
        let length = bytes.len() as u64;
        for byte in length.to_le_bytes() {
            self.absorb_byte(byte);
        }
        for &byte in bytes {
            self.absorb_byte(byte);
        }
    }

    fn absorb_byte(&mut self, byte: u8) {
        self.state ^= u128::from(byte);
        self.state = self.state.wrapping_mul(PRIME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_fields_in_the_same_order_digest_identically() {
        let first = ContentDigest::builder()
            .field(b"hello")
            .field_u64(7)
            .finish();
        let second = ContentDigest::builder()
            .field(b"hello")
            .field_u64(7)
            .finish();
        assert_eq!(first, second);
    }

    #[test]
    fn reordering_fields_changes_the_digest() {
        let forward = ContentDigest::builder().field(b"a").field(b"b").finish();
        let backward = ContentDigest::builder().field(b"b").field(b"a").finish();
        assert_ne!(forward, backward);
    }

    #[test]
    fn a_field_boundary_cannot_be_moved_without_changing_the_digest() {
        // The classic concatenation ambiguity: without framing these are the
        // same byte stream and would be indistinguishable.
        let split = ContentDigest::builder().field(b"ab").field(b"c").finish();
        let joined = ContentDigest::builder().field(b"a").field(b"bc").finish();
        assert_ne!(split, joined);
    }

    #[test]
    fn an_empty_field_is_still_a_field() {
        let with_empty = ContentDigest::builder().field(b"").field(b"x").finish();
        let without = ContentDigest::builder().field(b"x").finish();
        assert_ne!(with_empty, without);
    }

    #[test]
    fn an_empty_field_differs_from_no_field_at_the_end_too() {
        let trailing_empty = ContentDigest::builder().field(b"x").field(b"").finish();
        let without = ContentDigest::builder().field(b"x").finish();
        assert_ne!(trailing_empty, without);
    }

    #[test]
    fn the_domain_tag_is_absorbed_before_any_caller_field() {
        // A builder with no fields still carries the tag, so it is not the bare
        // FNV offset basis.
        let empty = ContentDigest::builder().finish();
        assert_ne!(empty.0, OFFSET_BASIS.to_be_bytes());
    }

    #[test]
    fn hex_rendering_is_fixed_width_lowercase() {
        let digest = ContentDigest::builder().field(b"anything").finish();
        let hex = digest.to_hex();
        assert_eq!(hex.len(), 32);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(hex, digest.to_string());
    }

    #[test]
    fn digests_are_stable_across_builder_clones() {
        let base = ContentDigest::builder().field(b"prefix");
        let left = base.clone().field(b"left").finish();
        let right = base.field(b"left").finish();
        assert_eq!(left, right);
    }
}
