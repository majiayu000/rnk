use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

fn app() -> Element {
    Box::new()
        .padding(1)
        .child(Text::new("Hello, rnk!").color(Color::Green).bold().into_element())
        .into_element()
}
