use rnk::prelude::*;
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(hint) = build_usage_hint(&args) {
        eprintln!("{hint}");
        return Ok(());
    }

    render(app).run()
}

fn build_usage_hint(args: &[String]) -> Option<String> {
    if args.is_empty() {
        return None;
    }

    if args[0] == "example" {
        let suggested = args
            .get(1)
            .map(|name| format!("cargo run --example {name}"))
            .unwrap_or_else(|| "cargo run --example <name>".to_string());

        return Some(format!(
            "Detected subcommand-style example invocation.\nRun examples with `--example`:\n  {suggested}\nTry: `cargo run --example hello`"
        ));
    }

    Some(format!(
        "Unexpected arguments: {}\nThe `rnk` binary has no subcommands.\nTo run demos, use:\n  cargo run --example <name>",
        args.join(" ")
    ))
}

fn app() -> Element {
    Box::new()
        .padding(1)
        .child(
            Text::new("Hello, rnk!")
                .color(Color::Green)
                .bold()
                .into_element(),
        )
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::build_usage_hint;

    #[test]
    fn no_hint_when_no_args() {
        let args: Vec<String> = vec![];
        assert!(build_usage_hint(&args).is_none());
    }

    #[test]
    fn hint_for_example_without_name() {
        let args = vec!["example".to_string()];
        let hint = build_usage_hint(&args).unwrap();
        assert!(hint.contains("cargo run --example <name>"));
    }

    #[test]
    fn hint_for_example_with_name() {
        let args = vec!["example".to_string(), "aria".to_string()];
        let hint = build_usage_hint(&args).unwrap();
        assert!(hint.contains("cargo run --example aria"));
    }

    #[test]
    fn hint_for_unexpected_args() {
        let args = vec!["foo".to_string(), "bar".to_string()];
        let hint = build_usage_hint(&args).unwrap();
        assert!(hint.contains("Unexpected arguments: foo bar"));
        assert!(hint.contains("has no subcommands"));
    }
}
