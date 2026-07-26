use std::error::Error;

use rnk::{Element, TextCoordinateError, TextRenderError, try_render_to_string};

#[test]
fn try_render_to_string_preserves_source_and_returns_no_partial_string() {
    let mut invalid = Element::text("must-not-be-returned");
    invalid.style.padding.left = f32::NAN;

    let error = try_render_to_string(&invalid, 20).unwrap_err();
    assert!(matches!(
        error,
        TextRenderError::Coordinate {
            source: TextCoordinateError::NonFinite,
            ..
        }
    ));
    assert!(matches!(
        error
            .source()
            .and_then(|source| { source.downcast_ref::<TextCoordinateError>() }),
        Some(TextCoordinateError::NonFinite)
    ));
}
