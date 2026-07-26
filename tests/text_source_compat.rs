use rnk::components::{Line, Span, Text};
use rnk::core::{Children, ElementType, TextWrap};
use rnk::{Color, Element, ElementId, Style, render_to_string};

#[test]
fn exact_crlf_and_trailing_break_ranges() {
    let source = "a\r\nb\r\n";
    let element = Text::new(source).into_element();
    let stored = element
        .text_content
        .as_deref()
        .expect("Text::new must publish its exact source through text_content");

    assert_eq!(stored.as_bytes(), source.as_bytes());
    assert_eq!(&stored[1..3], "\r\n");
    assert_eq!(&stored[4..6], "\r\n");

    let cloned = element.clone();
    assert_ne!(cloned.id, element.id);
    assert_eq!(cloned.text_content, element.text_content);
    assert_eq!(cloned.spans.as_ref().map(Vec::len), Some(2));
    assert_eq!(
        cloned.spans.as_ref().map(Vec::len),
        element.spans.as_ref().map(Vec::len)
    );
}

#[test]
fn exact_literal_source_variants() {
    let sources = [
        "",
        "alpha",
        "a\nb",
        "a\r\nb\r\n",
        "a\rb\r",
        "a\r\n\nb\n",
        "\n\n",
        "tail\n",
        "\ttab\u{1b}[31m",
    ];

    for source in sources {
        let element = Text::new(source).into_element();
        assert_eq!(
            element.text_content.as_deref(),
            Some(source),
            "source bytes changed for {source:?}"
        );
    }
}

#[test]
fn structured_source_domain() {
    let spans_element = Text::spans(vec![
        Span::new("hello ").color(Color::Red),
        Span::new("world").bold(),
    ])
    .into_element();
    assert_eq!(spans_element.text_content.as_deref(), Some("hello world"));
    let spans = spans_element
        .spans
        .as_ref()
        .expect("Text::spans must preserve structured ranges");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].spans.len(), 2);
    assert_eq!(spans[0].spans[0].style.color, Some(Color::Red));
    assert!(spans[0].spans[1].style.bold);

    let line_element =
        Text::line(Line::from_spans(vec![Span::new("single").italic()])).into_element();
    assert_eq!(line_element.text_content.as_deref(), Some("single"));
    assert!(line_element.style.italic);
    assert!(line_element.spans.is_none());

    let first = Line::from_spans(vec![
        Span::new("left").color(Color::Red),
        Span::new(" side").bold(),
    ]);
    let second = Line::new();
    let third = Line::from_spans(vec![Span::new("right").color(Color::Blue)]);
    let multiline = Text::from_lines(vec![first, second, third]).into_element();

    assert_eq!(
        multiline.text_content.as_deref(),
        Some("left side\n\nright")
    );
    let lines = multiline
        .spans
        .as_ref()
        .expect("Text::from_lines must preserve structured ranges");
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].spans.len(), 2);
    assert!(lines[1].is_empty());
    assert_eq!(lines[2].spans[0].style.color, Some(Color::Blue));
}

#[test]
fn builder_updates_preserve_source_state() {
    let exact = Text::new("before\r\nafter\r\n")
        .color(Color::Green)
        .bold()
        .wrap(TextWrap::Truncate)
        .key("exact")
        .into_element();

    assert_eq!(exact.text_content.as_deref(), Some("before\r\nafter\r\n"));
    assert_eq!(exact.key.as_deref(), Some("exact"));
    assert_eq!(exact.style.text_wrap, TextWrap::Truncate);
    assert!(exact.style.bold);

    let structured = Text::spans(vec![Span::new("left"), Span::new("right")])
        .underline()
        .key("structured")
        .into_element();
    assert_eq!(structured.text_content.as_deref(), Some("leftright"));
    assert_eq!(structured.key.as_deref(), Some("structured"));
    assert!(
        structured
            .spans
            .as_ref()
            .expect("builder updates must not discard structured ranges")[0]
            .spans
            .iter()
            .all(|span| span.style.underline)
    );
}

#[test]
fn source_state_initializes_for_default_clone_and_empty_lines() {
    let default = Text::default().into_element();
    assert_eq!(default.text_content.as_deref(), Some(""));
    assert!(default.spans.is_none());

    let original = Text::new("clone\r\nsource\r\n").bold();
    let cloned = original.clone();
    let original_element = original.into_element();
    let cloned_element = cloned.into_element();
    assert_eq!(
        original_element.text_content.as_deref(),
        Some("clone\r\nsource\r\n")
    );
    assert_eq!(cloned_element.text_content, original_element.text_content);
    assert!(cloned_element.style.bold);

    let empty_lines = Text::from_lines(Vec::new()).into_element();
    assert_eq!(empty_lines.text_content.as_deref(), Some(""));
    assert!(
        empty_lines
            .spans
            .as_ref()
            .expect("empty structured lines must retain their source domain")
            .is_empty()
    );
}

#[test]
fn plain_multiline_compatibility() {
    let element = Text::new("alpha\nbeta").into_element();

    assert_eq!(element.text_content.as_deref(), Some("alpha\nbeta"));
    let lines = element
        .spans
        .as_ref()
        .expect("legacy renderer needs normalized multiline spans");
    assert_eq!(lines.len(), 2);
    assert_eq!(render_to_string(&element, 20), "alpha\nbeta");
}

#[test]
fn single_span_structured_constructors_keep_plain_wrap_path() {
    let spans = Text::spans(vec![Span::new("abcdef").bold()]).into_element();
    let plain_bold = Text::new("abcdef").bold().into_element();
    assert_eq!(
        render_to_string(&spans, 3),
        render_to_string(&plain_bold, 3)
    );
    assert!(spans.style.bold);
    assert!(spans.spans.is_none());

    let line = Text::line(Line::raw("abcdef")).italic().into_element();
    let plain_italic = Text::new("abcdef").italic().into_element();
    assert_eq!(
        render_to_string(&line, 3),
        render_to_string(&plain_italic, 3)
    );
    assert!(line.style.italic);
    assert!(line.spans.is_none());

    let from_lines = Text::from_lines(vec![Line::raw("abcdef")])
        .color(Color::Blue)
        .into_element();
    let plain_blue = Text::new("abcdef").color(Color::Blue).into_element();
    assert_eq!(
        render_to_string(&from_lines, 3),
        render_to_string(&plain_blue, 3)
    );
    assert_eq!(from_lines.style.color, Some(Color::Blue));
    assert!(from_lines.spans.is_none());
}

#[test]
fn external_element_struct_literal_compiles() {
    let element = Element {
        id: ElementId::new(),
        element_type: ElementType::Text,
        style: Style::new(),
        children: Children::new(),
        text_content: Some("literal".to_owned()),
        spans: None,
        key: None,
        accessibility: None,
        scroll_offset_x: None,
        scroll_offset_y: None,
    };

    assert_eq!(element.get_text(), Some("literal"));

    let span_only = Element {
        id: ElementId::new(),
        element_type: ElementType::Text,
        style: Style::new(),
        children: Children::new(),
        text_content: None,
        spans: Some(vec![Line::raw("span"), Line::raw("only")]),
        key: None,
        accessibility: None,
        scroll_offset_x: None,
        scroll_offset_y: None,
    };

    assert_eq!(render_to_string(&span_only, 20), "span\nonly");
}
