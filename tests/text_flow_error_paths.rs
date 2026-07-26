use std::error::Error;
use std::io;

use rnk::testing::TestRenderer;
use rnk::{Element, TextCoordinateError, TextRenderError};

#[test]
fn typed_error_reaches_remaining_callers() {
    let mut invalid = Element::text("invalid");
    invalid.style.padding.left = f32::NAN;
    let renderer = TestRenderer::new(20, 4);

    assert!(matches!(
        renderer.try_render_to_ansi(&invalid),
        Err(TextRenderError::Coordinate {
            source: TextCoordinateError::NonFinite,
            ..
        })
    ));
    assert!(matches!(
        renderer.try_render_to_plain(&invalid),
        Err(TextRenderError::Coordinate {
            source: TextCoordinateError::NonFinite,
            ..
        })
    ));

    let io_error = TextRenderError::io(
        "writing terminal frame",
        io::Error::new(io::ErrorKind::BrokenPipe, "closed"),
    );
    assert!(matches!(io_error, TextRenderError::Io { .. }));
    assert_eq!(
        io_error
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .map(io::Error::kind),
        Some(io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn caller_failure_commits_no_partial_output() {
    let mut invalid = Element::text("partial");
    invalid.style.padding.left = f32::NAN;
    let renderer = TestRenderer::new(20, 4);

    assert!(renderer.try_render_to_ansi(&invalid).is_err());
    assert!(std::panic::catch_unwind(|| renderer.render_to_ansi(&invalid)).is_err());

    let valid = Element::text("complete");
    assert_eq!(
        renderer.try_render_to_plain(&valid).unwrap().trim(),
        "complete"
    );
}
