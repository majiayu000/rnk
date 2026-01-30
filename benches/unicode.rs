//! Unicode width calculation benchmarks

use rnk::layout::{
    TextAlign, display_width, measure_text, measure_text_width, pad_text, truncate_middle,
    truncate_start, truncate_text, wrap_text,
};

fn main() {
    divan::main();
}

// Test strings
const ASCII_SHORT: &str = "Hello, World!";
const ASCII_LONG: &str = "The quick brown fox jumps over the lazy dog. This is a longer string for benchmarking purposes.";
const CJK_SHORT: &str = "你好世界";
const CJK_LONG: &str = "这是一段较长的中文文本，用于测试Unicode宽度计算的性能。包含各种常用汉字。";
const MIXED: &str = "Hello 你好 World 世界 Test 测试 Mixed 混合文本";
const EMOJI: &str = "Hello 👋 World 🌍 Test 🧪 Emoji 😀";
const EMOJI_ZWJ: &str = "Family: 👨‍👩‍👧‍👦 Flag: 🇺🇸 Skin: 👋🏽";

#[divan::bench]
fn measure_width_ascii_short() {
    divan::black_box(measure_text_width(ASCII_SHORT));
}

#[divan::bench]
fn measure_width_ascii_long() {
    divan::black_box(measure_text_width(ASCII_LONG));
}

#[divan::bench]
fn measure_width_cjk_short() {
    divan::black_box(measure_text_width(CJK_SHORT));
}

#[divan::bench]
fn measure_width_cjk_long() {
    divan::black_box(measure_text_width(CJK_LONG));
}

#[divan::bench]
fn measure_width_mixed() {
    divan::black_box(measure_text_width(MIXED));
}

#[divan::bench]
fn measure_width_emoji() {
    divan::black_box(measure_text_width(EMOJI));
}

#[divan::bench]
fn measure_width_emoji_zwj() {
    divan::black_box(measure_text_width(EMOJI_ZWJ));
}

#[divan::bench]
fn display_width_mixed() {
    divan::black_box(display_width(MIXED));
}

#[divan::bench]
fn measure_text_dimensions() {
    let multiline = "Line 1\nLine 2 longer\nLine 3\nLine 4 even longer text";
    divan::black_box(measure_text(multiline));
}

#[divan::bench(args = [20, 40, 60, 80])]
fn wrap_text_ascii(width: usize) {
    divan::black_box(wrap_text(ASCII_LONG, width));
}

#[divan::bench(args = [20, 40, 60])]
fn wrap_text_cjk(width: usize) {
    divan::black_box(wrap_text(CJK_LONG, width));
}

#[divan::bench(args = [30, 50, 70])]
fn wrap_text_mixed(width: usize) {
    divan::black_box(wrap_text(MIXED, width));
}

#[divan::bench(args = [20, 40, 60])]
fn truncate_text_ascii(width: usize) {
    divan::black_box(truncate_text(ASCII_LONG, width, "..."));
}

#[divan::bench(args = [20, 40, 60])]
fn truncate_text_cjk(width: usize) {
    divan::black_box(truncate_text(CJK_LONG, width, "…"));
}

#[divan::bench(args = [30, 50])]
fn truncate_start_mixed(width: usize) {
    divan::black_box(truncate_start(MIXED, width, "..."));
}

#[divan::bench(args = [30, 50])]
fn truncate_middle_mixed(width: usize) {
    divan::black_box(truncate_middle(MIXED, width, "..."));
}

#[divan::bench]
fn pad_text_left() {
    divan::black_box(pad_text("Hello", 20, TextAlign::Left));
}

#[divan::bench]
fn pad_text_right() {
    divan::black_box(pad_text("Hello", 20, TextAlign::Right));
}

#[divan::bench]
fn pad_text_center() {
    divan::black_box(pad_text("Hello", 20, TextAlign::Center));
}

#[divan::bench]
fn pad_text_cjk() {
    divan::black_box(pad_text("你好", 20, TextAlign::Center));
}

// Stress test with very long strings
#[divan::bench]
fn measure_width_very_long() {
    let long_text = ASCII_LONG.repeat(100);
    divan::black_box(measure_text_width(&long_text));
}

#[divan::bench]
fn wrap_text_very_long() {
    let long_text = ASCII_LONG.repeat(50);
    divan::black_box(wrap_text(&long_text, 80));
}

// Box drawing characters (common in TUI)
#[divan::bench]
fn measure_width_box_drawing() {
    let box_chars = "┌─────────────────┐│                 │└─────────────────┘";
    divan::black_box(measure_text_width(box_chars));
}
