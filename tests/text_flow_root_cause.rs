use rnk::prelude::{Color, Span, Text, TextWrap};
use rnk::testing::TestRenderer;

#[test]
fn measure_rows_must_equal_rendered_rows() {
    const WIDTH: u16 = 4;
    const HEIGHT: u16 = 8;

    let fixtures = [
        (
            "plain",
            Text::new("abcdefgh").wrap(TextWrap::Wrap).into_element(),
            "abcdefgh",
        ),
        (
            "rich_spans",
            Text::spans(vec![
                Span::new("abcd").bold(),
                Span::new("efgh").color(Color::Green),
            ])
            .wrap(TextWrap::Wrap)
            .into_element(),
            "abcdefgh",
        ),
    ];

    let renderer = TestRenderer::new(WIDTH, HEIGHT);
    let mut mismatches = Vec::new();

    for (name, element, expected_content) in fixtures {
        let measured_rows = renderer
            .get_layout(&element)
            .expect("text fixture must have a computed layout")
            .height
            .ceil() as usize;
        let rendered = renderer.render_to_plain(&element);
        let rendered_rows = if rendered.is_empty() {
            0
        } else {
            rendered.split("\r\n").count()
        };
        let rendered_content = rendered.replace("\r\n", "");

        if measured_rows != rendered_rows || rendered_content != expected_content {
            mismatches.push(format!(
                "{name}: measured_rows={measured_rows}, rendered_rows={rendered_rows}, \
                 expected_content={expected_content:?}, rendered_content={rendered_content:?}"
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "layout measurement and rendering disagree:\n{}",
        mismatches.join("\n")
    );
}
