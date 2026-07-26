use std::error::Error;

use rnk::{Element, TextRenderError, layout::TextFlowError, try_render_to_string_with_tab_stop};

#[test]
fn try_render_to_string_preserves_source_and_returns_no_partial_string() {
    let element = Element::text("\tmust-not-be-returned");

    let error = try_render_to_string_with_tab_stop(&element, 20, 0).unwrap_err();
    assert!(matches!(
        error,
        TextRenderError::Flow {
            source: TextFlowError::InvalidTabStop,
            ..
        }
    ));
    assert!(matches!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<TextFlowError>()),
        Some(TextFlowError::InvalidTabStop)
    ));

    assert_eq!(
        try_render_to_string_with_tab_stop(&element, 20, 4)
            .unwrap()
            .trim(),
        "must-not-be-returned"
    );
}
