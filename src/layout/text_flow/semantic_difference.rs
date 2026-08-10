//! Collision-free, control-safe TextFlow semantic difference diagnostics.

use std::fmt::Debug;

use crate::core::{Dimension, Edges, Style};

use super::{
    StyledTextRange, TextFlow, TextFlowCacheIdentity, TextFlowOptions, TextFlowRow, TextFlowRun,
    TextFlowToken,
};

pub(super) fn first_difference(full: &TextFlow, incremental: &TextFlow) -> Option<String> {
    if let Some(difference) = string_sequence("text_flow.rows", &full.rows, &incremental.rows) {
        return Some(difference);
    }
    if let Some(difference) = row_sequence(
        "text_flow.logical_rows",
        &full.logical_rows,
        &incremental.logical_rows,
    ) {
        return Some(difference);
    }
    if let Some(difference) = token_sequence("text_flow.tokens", &full.tokens, &incremental.tokens)
    {
        return Some(difference);
    }
    if let Some(difference) = scalar_sequence(
        "text_flow.position_map",
        &full.position_map,
        &incremental.position_map,
    ) {
        return Some(difference);
    }
    if let Some(difference) = scalar_sequence(
        "text_flow.diagnostics",
        &full.diagnostics,
        &incremental.diagnostics,
    ) {
        return Some(difference);
    }
    cache_identity_difference(
        "text_flow.cache_identity",
        &full.cache_identity,
        &incremental.cache_identity,
    )
}

fn scalar_difference<T: Debug + PartialEq>(
    path: &str,
    full: &T,
    incremental: &T,
) -> Option<String> {
    (full != incremental).then(|| format!("path={path} full={full:?} incremental={incremental:?}"))
}

fn floats_are_semantically_equal(full: f32, incremental: f32) -> bool {
    full == incremental || (full.is_nan() && incremental.is_nan())
}

fn float_difference(path: &str, full: f32, incremental: f32) -> Option<String> {
    (!floats_are_semantically_equal(full, incremental)).then(|| {
        format!(
            "path={path} full={full:?}/bits:0x{:08x} incremental={incremental:?}/bits:0x{:08x}",
            full.to_bits(),
            incremental.to_bits()
        )
    })
}

fn optional_float_difference(
    path: &str,
    full: Option<f32>,
    incremental: Option<f32>,
) -> Option<String> {
    match (full, incremental) {
        (Some(full), Some(incremental)) => float_difference(path, full, incremental),
        (None, None) => None,
        (Some(_), None) => Some(format!(
            "path={path}.presence full=present incremental=missing"
        )),
        (None, Some(_)) => Some(format!(
            "path={path}.presence full=missing incremental=present"
        )),
    }
}

fn dimension_difference(path: &str, full: Dimension, incremental: Dimension) -> Option<String> {
    match (full, incremental) {
        (Dimension::Auto, Dimension::Auto) => None,
        (Dimension::Points(full), Dimension::Points(incremental)) => {
            float_difference(&format!("{path}.points"), full, incremental)
        }
        (Dimension::Percent(full), Dimension::Percent(incremental)) => {
            float_difference(&format!("{path}.percent"), full, incremental)
        }
        _ => Some(format!(
            "path={path}.variant full={} incremental={}",
            dimension_variant(full),
            dimension_variant(incremental)
        )),
    }
}

fn dimension_variant(value: Dimension) -> &'static str {
    match value {
        Dimension::Auto => "Auto",
        Dimension::Points(_) => "Points",
        Dimension::Percent(_) => "Percent",
    }
}

fn edges_difference(path: &str, full: Edges, incremental: Edges) -> Option<String> {
    float_difference(&format!("{path}.top"), full.top, incremental.top)
        .or_else(|| float_difference(&format!("{path}.right"), full.right, incremental.right))
        .or_else(|| float_difference(&format!("{path}.bottom"), full.bottom, incremental.bottom))
        .or_else(|| float_difference(&format!("{path}.left"), full.left, incremental.left))
}

fn sequence_length_difference(path: &str, full: usize, incremental: usize) -> Option<String> {
    if full == incremental {
        return None;
    }
    let index = full.min(incremental);
    let (full_value, incremental_value) = if full < incremental {
        ("missing", "present")
    } else {
        ("present", "missing")
    };
    Some(format!(
        "path={path}[{index}] full_len={full} incremental_len={incremental} \
         full={full_value} incremental={incremental_value}"
    ))
}

fn byte_value(value: Option<u8>) -> String {
    value.map_or_else(|| "missing".to_owned(), |byte| format!("0x{byte:02x}"))
}

fn string_difference(path: &str, full: &str, incremental: &str) -> Option<String> {
    if full == incremental {
        return None;
    }
    let full_bytes = full.as_bytes();
    let incremental_bytes = incremental.as_bytes();
    let index = full_bytes
        .iter()
        .zip(incremental_bytes)
        .position(|(full, incremental)| full != incremental)
        .unwrap_or_else(|| full_bytes.len().min(incremental_bytes.len()));
    Some(format!(
        "path={path}.byte[{index}] full_len={} incremental_len={} full={} incremental={}",
        full_bytes.len(),
        incremental_bytes.len(),
        byte_value(full_bytes.get(index).copied()),
        byte_value(incremental_bytes.get(index).copied())
    ))
}

fn scalar_sequence<T: Debug + PartialEq>(
    path: &str,
    full: &[T],
    incremental: &[T],
) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if let Some(difference) = scalar_difference(&format!("{path}[{index}]"), full, incremental)
        {
            return Some(difference);
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn string_sequence(path: &str, full: &[String], incremental: &[String]) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if let Some(difference) = string_difference(&format!("{path}[{index}]"), full, incremental)
        {
            return Some(difference);
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn row_sequence(path: &str, full: &[TextFlowRow], incremental: &[TextFlowRow]) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if full != incremental {
            return row_difference(&format!("{path}[{index}]"), full, incremental);
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn row_difference(path: &str, full: &TextFlowRow, incremental: &TextFlowRow) -> Option<String> {
    scalar_difference(&format!("{path}.index"), &full.index, &incremental.index)
        .or_else(|| scalar_difference(&format!("{path}.width"), &full.width, &incremental.width))
        .or_else(|| string_difference(&format!("{path}.text"), &full.text, &incremental.text))
        .or_else(|| run_sequence(&format!("{path}.runs"), &full.runs, &incremental.runs))
}

fn run_sequence(path: &str, full: &[TextFlowRun], incremental: &[TextFlowRun]) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if full != incremental {
            return run_difference(&format!("{path}[{index}]"), full, incremental);
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn run_difference(path: &str, full: &TextFlowRun, incremental: &TextFlowRun) -> Option<String> {
    scalar_difference(
        &format!("{path}.token_index"),
        &full.token_index,
        &incremental.token_index,
    )
    .or_else(|| scalar_difference(&format!("{path}.row"), &full.row, &incremental.row))
    .or_else(|| scalar_difference(&format!("{path}.column"), &full.column, &incremental.column))
    .or_else(|| scalar_difference(&format!("{path}.width"), &full.width, &incremental.width))
    .or_else(|| string_difference(&format!("{path}.text"), &full.text, &incremental.text))
    .or_else(|| style_difference(&format!("{path}.style"), &full.style, &incremental.style))
}

fn token_sequence(
    path: &str,
    full: &[TextFlowToken],
    incremental: &[TextFlowToken],
) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if full != incremental {
            return token_difference(&format!("{path}[{index}]"), full, incremental);
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn token_difference(
    path: &str,
    full: &TextFlowToken,
    incremental: &TextFlowToken,
) -> Option<String> {
    scalar_difference(&format!("{path}.source"), &full.source, &incremental.source)
        .or_else(|| {
            string_difference(
                &format!("{path}.safe_text"),
                &full.safe_text,
                &incremental.safe_text,
            )
        })
        .or_else(|| style_difference(&format!("{path}.style"), &full.style, &incremental.style))
        .or_else(|| {
            scalar_difference(
                &format!("{path}.display_width"),
                &full.display_width,
                &incremental.display_width,
            )
        })
        .or_else(|| {
            scalar_difference(
                &format!("{path}.placement"),
                &full.placement,
                &incremental.placement,
            )
        })
        .or_else(|| scalar_difference(&format!("{path}.class"), &full.class, &incremental.class))
}

fn cache_identity_difference(
    path: &str,
    full: &TextFlowCacheIdentity,
    incremental: &TextFlowCacheIdentity,
) -> Option<String> {
    string_difference(
        &format!("{path}.input.source"),
        &full.input.source,
        &incremental.input.source,
    )
    .or_else(|| {
        scalar_difference(
            &format!("{path}.input.source_kind"),
            &full.input.source_kind,
            &incremental.input.source_kind,
        )
    })
    .or_else(|| {
        style_difference(
            &format!("{path}.input.default_style"),
            &full.input.default_style,
            &incremental.input.default_style,
        )
    })
    .or_else(|| {
        styled_range_sequence(
            &format!("{path}.input.styled_ranges"),
            &full.input.styled_ranges,
            &incremental.input.styled_ranges,
        )
    })
    .or_else(|| {
        options_difference(
            &format!("{path}.options"),
            &full.options,
            &incremental.options,
        )
    })
}

fn styled_range_sequence(
    path: &str,
    full: &[StyledTextRange],
    incremental: &[StyledTextRange],
) -> Option<String> {
    for (index, (full, incremental)) in full.iter().zip(incremental).enumerate() {
        if full != incremental {
            let path = format!("{path}[{index}]");
            return scalar_difference(&format!("{path}.range"), &full.range, &incremental.range)
                .or_else(|| {
                    style_difference(&format!("{path}.style"), &full.style, &incremental.style)
                });
        }
    }
    sequence_length_difference(path, full.len(), incremental.len())
}

fn options_difference(
    path: &str,
    full: &TextFlowOptions,
    incremental: &TextFlowOptions,
) -> Option<String> {
    scalar_difference(
        &format!("{path}.max_width"),
        &full.max_width,
        &incremental.max_width,
    )
    .or_else(|| {
        scalar_difference(
            &format!("{path}.text_wrap"),
            &full.text_wrap,
            &incremental.text_wrap,
        )
    })
    .or_else(|| {
        scalar_difference(
            &format!("{path}.overflow_x"),
            &full.overflow_x,
            &incremental.overflow_x,
        )
    })
    .or_else(|| {
        scalar_difference(
            &format!("{path}.overflow_y"),
            &full.overflow_y,
            &incremental.overflow_y,
        )
    })
    .or_else(|| {
        scalar_difference(
            &format!("{path}.tab_stop"),
            &full.tab_stop,
            &incremental.tab_stop,
        )
    })
    .or_else(|| {
        string_difference(
            &format!("{path}.ellipsis"),
            &full.ellipsis,
            &incremental.ellipsis,
        )
    })
    .or_else(|| {
        scalar_difference(
            &format!("{path}.width_policy.revision"),
            &full.width_policy.revision,
            &incremental.width_policy.revision,
        )
    })
}

fn style_difference(path: &str, full: &Style, incremental: &Style) -> Option<String> {
    macro_rules! first_scalar_difference {
        ($($field:ident),+ $(,)?) => {
            $(
                if let Some(difference) = scalar_difference(
                    &format!("{path}.{}", stringify!($field)),
                    &full.$field,
                    &incremental.$field,
                ) {
                    return Some(difference);
                }
            )+
        };
    }
    macro_rules! return_difference {
        ($difference:expr) => {
            if let Some(difference) = $difference {
                return Some(difference);
            }
        };
    }

    first_scalar_difference!(display, position);
    return_difference!(optional_float_difference(
        &format!("{path}.top"),
        full.top,
        incremental.top,
    ));
    return_difference!(optional_float_difference(
        &format!("{path}.right"),
        full.right,
        incremental.right,
    ));
    return_difference!(optional_float_difference(
        &format!("{path}.bottom"),
        full.bottom,
        incremental.bottom,
    ));
    return_difference!(optional_float_difference(
        &format!("{path}.left"),
        full.left,
        incremental.left,
    ));
    first_scalar_difference!(flex_direction, flex_wrap);
    return_difference!(float_difference(
        &format!("{path}.flex_grow"),
        full.flex_grow,
        incremental.flex_grow,
    ));
    return_difference!(float_difference(
        &format!("{path}.flex_shrink"),
        full.flex_shrink,
        incremental.flex_shrink,
    ));
    return_difference!(dimension_difference(
        &format!("{path}.flex_basis"),
        full.flex_basis,
        incremental.flex_basis,
    ));
    first_scalar_difference!(align_items, align_self, justify_content);
    return_difference!(edges_difference(
        &format!("{path}.padding"),
        full.padding,
        incremental.padding,
    ));
    return_difference!(edges_difference(
        &format!("{path}.margin"),
        full.margin,
        incremental.margin,
    ));
    return_difference!(float_difference(
        &format!("{path}.gap"),
        full.gap,
        incremental.gap,
    ));
    return_difference!(optional_float_difference(
        &format!("{path}.row_gap"),
        full.row_gap,
        incremental.row_gap,
    ));
    return_difference!(optional_float_difference(
        &format!("{path}.column_gap"),
        full.column_gap,
        incremental.column_gap,
    ));
    macro_rules! first_dimension_difference {
        ($($field:ident),+ $(,)?) => {
            $(
                return_difference!(dimension_difference(
                    &format!("{path}.{}", stringify!($field)),
                    full.$field,
                    incremental.$field,
                ));
            )+
        };
    }
    first_dimension_difference!(width, height, min_width, min_height, max_width, max_height,);
    first_scalar_difference!(
        border_style,
        border_color,
        border_top_color,
        border_right_color,
        border_bottom_color,
        border_left_color,
        border_dim,
        border_top,
        border_bottom,
        border_left,
        border_right,
        color,
        background_color,
        bold,
        italic,
        underline,
        strikethrough,
        dim,
        inverse,
        text_wrap,
        overflow_x,
        overflow_y,
        is_static,
    );
    None
}
