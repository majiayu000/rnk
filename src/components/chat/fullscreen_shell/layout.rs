//! Where each region of a fullscreen chat sits, in terminal rows.
//!
//! The arithmetic is small and the ordering is the whole design:
//!
//! > **The bottom regions are allocated first. The transcript takes what is
//! > left.**
//!
//! Doing it the other way — transcript first, composer with the remainder — is
//! what makes a composer vanish on a short terminal. The transcript can always
//! scroll; the composer cannot, and a user who cannot see what they are typing
//! has lost the application. So the composer and status bar are paid first, and
//! any shortfall lands on the region that can absorb it.
//!
//! When the shortfall is total — the bottom regions alone exceed the terminal —
//! the layout is [refused] rather than clamped. Clamping there means drawing two
//! regions over each other, which renders as one region containing garbage from
//! both, and there is no way for a caller to notice.
//!
//! [refused]: FullscreenLayoutError

use std::fmt;

/// A contiguous horizontal band of the terminal.
///
/// Rows are half-open: a region at `top = 3` with `rows = 2` owns rows 3 and 4.
/// An empty region has `rows = 0` and still carries a meaningful `top`, which is
/// where the region below it begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    top: u16,
    rows: u16,
}

impl Region {
    /// Creates a region.
    pub const fn new(top: u16, rows: u16) -> Self {
        Self { top, rows }
    }

    /// Returns the first row this region owns.
    pub const fn top(self) -> u16 {
        self.top
    }

    /// Returns how many rows it owns.
    pub const fn rows(self) -> u16 {
        self.rows
    }

    /// Returns the first row *past* this region.
    pub const fn bottom(self) -> u16 {
        self.top + self.rows
    }

    /// Reports whether the region owns no rows.
    pub const fn is_empty(self) -> bool {
        self.rows == 0
    }

    /// Reports whether two regions share any row.
    ///
    /// Empty regions never overlap anything, including each other: they occupy
    /// no rows, so there is nothing to collide.
    pub const fn overlaps(self, other: Self) -> bool {
        if self.rows == 0 || other.rows == 0 {
            return false;
        }
        self.top < other.bottom() && other.top < self.bottom()
    }
}

/// The row assignment for one frame of a fullscreen chat.
///
/// The three regions tile the terminal exactly: they are contiguous, ordered
/// top to bottom, and their rows sum to the terminal height. That invariant is
/// asserted at construction rather than documented and hoped for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullscreenLayout {
    transcript: Region,
    composer: Region,
    status: Region,
    width: u16,
}

impl FullscreenLayout {
    /// The rows a transcript must have for the layout to be worth drawing.
    ///
    /// One. A transcript with zero rows shows the user nothing of the
    /// conversation, which is not a degraded chat — it is a different, useless
    /// application, and it should be reported rather than rendered.
    pub const MIN_TRANSCRIPT_ROWS: u16 = 1;

    /// Assigns rows, paying the bottom regions first.
    ///
    /// `composer_rows` is the height the composer needs for its current draft,
    /// already clamped by the caller to the composer's own maximum. The layout
    /// does not clamp it further: silently shrinking the composer would hide the
    /// end of what the user is typing, and the caller is the only one that knows
    /// which lines could be dropped.
    pub const fn try_new(
        width: u16,
        height: u16,
        composer_rows: u16,
        status_rows: u16,
    ) -> Result<Self, FullscreenLayoutError> {
        if width == 0 {
            return Err(FullscreenLayoutError::ZeroWidth);
        }
        // Checked, because the fixed regions are caller-supplied: a wrapped sum
        // would report a tiny requirement and hand back overlapping regions,
        // which is the exact failure this whole ordering exists to prevent.
        let Some(fixed) = composer_rows.checked_add(status_rows) else {
            return Err(FullscreenLayoutError::TooShort {
                height,
                composer_rows,
                status_rows,
                required: u16::MAX,
            });
        };
        let Some(required) = fixed.checked_add(Self::MIN_TRANSCRIPT_ROWS) else {
            return Err(FullscreenLayoutError::TooShort {
                height,
                composer_rows,
                status_rows,
                required: u16::MAX,
            });
        };
        if height < required {
            return Err(FullscreenLayoutError::TooShort {
                height,
                composer_rows,
                status_rows,
                required,
            });
        }
        let transcript_rows = height - fixed;
        Ok(Self {
            transcript: Region::new(0, transcript_rows),
            composer: Region::new(transcript_rows, composer_rows),
            status: Region::new(transcript_rows + composer_rows, status_rows),
            width,
        })
    }

    /// Returns the transcript region, which owns every remaining row.
    pub const fn transcript(self) -> Region {
        self.transcript
    }

    /// Returns the composer region, pinned above the status bar.
    pub const fn composer(self) -> Region {
        self.composer
    }

    /// Returns the status region, pinned to the bottom.
    pub const fn status(self) -> Region {
        self.status
    }

    /// Returns the terminal width the layout was computed for.
    pub const fn width(self) -> u16 {
        self.width
    }

    /// Returns the terminal height the regions tile.
    pub const fn height(self) -> u16 {
        self.status.bottom()
    }

    /// Reports whether any two regions share a row.
    ///
    /// Always false by construction. Kept public because it is what the tests
    /// assert, and a caller composing its own regions alongside these can use
    /// the same check.
    pub const fn has_overlap(self) -> bool {
        self.transcript.overlaps(self.composer)
            || self.transcript.overlaps(self.status)
            || self.composer.overlaps(self.status)
    }
}

/// Every way a fullscreen layout can be refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FullscreenLayoutError {
    /// A width of zero cannot describe any rendered line.
    ZeroWidth,
    /// The terminal cannot hold the fixed regions plus a usable transcript.
    TooShort {
        /// The terminal height that was offered.
        height: u16,
        /// The rows the composer asked for.
        composer_rows: u16,
        /// The rows the status bar asked for.
        status_rows: u16,
        /// The height that would have been enough.
        required: u16,
    },
}

impl fmt::Display for FullscreenLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => {
                f.write_str("terminal width must be greater than zero to lay out a chat")
            }
            Self::TooShort {
                height,
                composer_rows,
                status_rows,
                required,
            } => write!(
                f,
                "a terminal {height} row(s) tall cannot hold a {composer_rows}-row composer, \
                 a {status_rows}-row status bar and a transcript: {required} row(s) are needed"
            ),
        }
    }
}

impl std::error::Error for FullscreenLayoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_regions_tile_the_terminal_exactly() {
        let layout = FullscreenLayout::try_new(80, 24, 3, 1).expect("tall enough");

        assert_eq!(layout.transcript(), Region::new(0, 20));
        assert_eq!(layout.composer(), Region::new(20, 3));
        assert_eq!(layout.status(), Region::new(23, 1));
        assert_eq!(
            layout.transcript().rows() + layout.composer().rows() + layout.status().rows(),
            24
        );
        assert!(!layout.has_overlap());
    }

    #[test]
    fn the_bottom_regions_stay_pinned_to_the_bottom() {
        let layout = FullscreenLayout::try_new(80, 24, 3, 1).expect("tall enough");
        assert_eq!(layout.status().bottom(), layout.height());
        assert_eq!(layout.composer().bottom(), layout.status().top());
        assert_eq!(layout.transcript().bottom(), layout.composer().top());
    }

    #[test]
    fn a_growing_composer_takes_rows_from_the_transcript_and_nothing_else() {
        let small = FullscreenLayout::try_new(80, 24, 1, 1).expect("tall enough");
        let grown = FullscreenLayout::try_new(80, 24, 5, 1).expect("tall enough");

        assert_eq!(small.transcript().rows(), 22);
        assert_eq!(grown.transcript().rows(), 18);
        // The status bar never moves off the bottom, however tall the composer.
        assert_eq!(small.status(), grown.status());
        assert!(!grown.has_overlap());
    }

    #[test]
    fn the_smallest_workable_terminal_still_lays_out_without_overlap() {
        let layout = FullscreenLayout::try_new(1, 3, 1, 1).expect("exactly enough");

        assert_eq!(
            layout.transcript().rows(),
            FullscreenLayout::MIN_TRANSCRIPT_ROWS
        );
        assert!(!layout.has_overlap());
    }

    #[test]
    fn a_terminal_one_row_too_short_is_refused_rather_than_overlapped() {
        let error = FullscreenLayout::try_new(80, 2, 1, 1).expect_err("one row short");

        assert_eq!(
            error,
            FullscreenLayoutError::TooShort {
                height: 2,
                composer_rows: 1,
                status_rows: 1,
                required: 3,
            }
        );
        // The message names the height that would have worked, so a caller can
        // act on it rather than guess.
        assert!(error.to_string().contains("3 row(s) are needed"), "{error}");
    }

    #[test]
    fn a_zero_width_terminal_is_refused() {
        assert_eq!(
            FullscreenLayout::try_new(0, 24, 1, 1),
            Err(FullscreenLayoutError::ZeroWidth)
        );
    }

    #[test]
    fn a_status_bar_may_be_absent_without_disturbing_the_others() {
        let layout = FullscreenLayout::try_new(80, 24, 2, 0).expect("tall enough");

        assert_eq!(layout.transcript().rows(), 22);
        assert_eq!(layout.composer(), Region::new(22, 2));
        assert!(layout.status().is_empty());
        // An empty region collides with nothing, which is why it is allowed to
        // sit exactly where the composer ends.
        assert!(!layout.has_overlap());
    }

    #[test]
    fn every_height_from_the_minimum_upwards_tiles_without_overlap() {
        for height in 3..=200u16 {
            for composer_rows in 1..=5u16 {
                let Ok(layout) = FullscreenLayout::try_new(80, height, composer_rows, 1) else {
                    continue;
                };
                assert!(!layout.has_overlap(), "overlap at height {height}");
                assert_eq!(layout.height(), height, "lost rows at height {height}");
                assert!(
                    layout.transcript().rows() >= FullscreenLayout::MIN_TRANSCRIPT_ROWS,
                    "empty transcript at height {height}"
                );
            }
        }
    }

    #[test]
    fn fixed_regions_that_would_overflow_are_refused_not_wrapped() {
        let error = FullscreenLayout::try_new(80, u16::MAX, u16::MAX, 1).expect_err("overflows");

        assert!(matches!(
            error,
            FullscreenLayoutError::TooShort {
                required: u16::MAX,
                ..
            }
        ));
    }

    #[test]
    fn empty_regions_never_overlap() {
        let empty = Region::new(10, 0);
        let occupied = Region::new(10, 5);
        assert!(!empty.overlaps(occupied));
        assert!(!occupied.overlaps(empty));
        assert!(!empty.overlaps(empty));
    }

    #[test]
    fn adjacent_regions_do_not_count_as_overlapping() {
        let upper = Region::new(0, 5);
        let lower = Region::new(5, 5);
        assert!(!upper.overlaps(lower));
    }

    #[test]
    fn regions_sharing_a_single_row_do_overlap() {
        let upper = Region::new(0, 6);
        let lower = Region::new(5, 5);
        assert!(upper.overlaps(lower));
    }
}
