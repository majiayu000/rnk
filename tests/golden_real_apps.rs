use rnk::components::{
    Badge, BadgeVariant, Box as RnkBox, Confirm, ConfirmState, Message, Progress, ProgressSymbols,
    SelectInput, SelectItem, Stat, Text, TextArea, TextAreaState,
};
use rnk::core::{Color, Element, FlexDirection};
use rnk::testing::GoldenTest;

fn chat_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Message::system("session: deterministic chat").into_element())
        .child(Message::user("Summarize the release gates").into_element())
        .child(Message::assistant("CI, docs, and examples are all checked.").into_element())
        .child(Text::new("> ready").color(Color::Yellow).into_element())
        .into_element()
}

fn git_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .gap(1.0)
                .child(Text::new("branch").bold().into_element())
                .child(
                    Badge::new("main")
                        .variant(BadgeVariant::Success)
                        .into_element(),
                )
                .child(
                    Badge::new("+2")
                        .variant(BadgeVariant::Warning)
                        .into_element(),
                )
                .into_element(),
        )
        .child(Text::new("M src/testing/harness.rs").into_element())
        .child(Text::new("A tests/golden_real_apps.rs").into_element())
        .child(
            Text::new("checks: clean")
                .color(Color::Green)
                .into_element(),
        )
        .into_element()
}

fn top_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .gap(2.0)
                .child(Stat::new("CPU", "42%").trend_down("3%").into_element())
                .child(Stat::new("Mem", "1.8G").trend_up("128M").into_element())
                .into_element(),
        )
        .child(
            Progress::new()
                .progress(0.42)
                .width(18)
                .symbols(ProgressSymbols::ascii())
                .show_percent(true)
                .label("load")
                .into_element(),
        )
        .child(Text::new("pid  command      cpu").dim().into_element())
        .child(Text::new("101  rnk-demo     12%").into_element())
        .into_element()
}

fn form_flow() -> Element {
    let mut confirm = ConfirmState::default_yes("Submit profile?");
    confirm.focus_yes();

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Name: Ada Lovelace").into_element())
        .child(
            SelectInput::new(vec![
                SelectItem::new("Engineer", "engineer"),
                SelectItem::new("Designer", "designer"),
                SelectItem::new("Researcher", "researcher"),
            ])
            .highlighted(2)
            .limit(3)
            .into_element(),
        )
        .child(Confirm::new(&confirm).into_element())
        .into_element()
}

fn textarea_flow() -> Element {
    let state = TextAreaState::with_content("fn main() {\n    println!(\"rnk\");\n}");

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("editor: src/main.rs").bold().into_element())
        .child(
            TextArea::new(&state)
                .width(36)
                .height(5)
                .line_numbers(true)
                .prompt("| ")
                .into_element(),
        )
        .into_element()
}

#[test]
fn chat_flow_plain_golden() {
    GoldenTest::new("real_app_chat")
        .with_size(80, 12)
        .assert_match(&chat_flow());
}

#[test]
fn chat_flow_ansi_golden() {
    GoldenTest::new("real_app_chat")
        .ansi()
        .with_size(80, 12)
        .assert_match(&chat_flow());
}

#[test]
fn git_flow_plain_golden() {
    GoldenTest::new("real_app_git")
        .with_size(80, 12)
        .assert_match(&git_flow());
}

#[test]
fn top_flow_plain_golden() {
    GoldenTest::new("real_app_top")
        .with_size(80, 12)
        .assert_match(&top_flow());
}

#[test]
fn form_flow_plain_golden() {
    GoldenTest::new("real_app_forms")
        .with_size(80, 18)
        .assert_match(&form_flow());
}

#[test]
fn textarea_flow_plain_golden() {
    GoldenTest::new("real_app_textarea")
        .with_size(80, 12)
        .assert_match(&textarea_flow());
}
