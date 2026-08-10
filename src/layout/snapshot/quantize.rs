#![forbid(missing_docs)]

//! Checked absolute half-open cell quantization.

use super::error::{ArithmeticOperation, Axis, Edge, GeometryField, LayoutSnapshotError};
use super::{CellRect, SnapshotIdentity};

pub(crate) fn finite(
    identity: &SnapshotIdentity,
    field: GeometryField,
    value: f32,
) -> Result<f64, LayoutSnapshotError> {
    if value.is_finite() {
        Ok(f64::from(value))
    } else {
        Err(LayoutSnapshotError::NonFiniteGeometry {
            identity: identity.clone(),
            field,
            value_bits: value.to_bits(),
        })
    }
}

pub(crate) fn extent(
    identity: &SnapshotIdentity,
    axis: Axis,
    field: GeometryField,
    value: f32,
) -> Result<f64, LayoutSnapshotError> {
    let value64 = finite(identity, field, value)?;
    if value64 < 0.0 {
        return Err(LayoutSnapshotError::NegativeExtent {
            identity: identity.clone(),
            axis,
            value_bits: value.to_bits(),
        });
    }
    Ok(value64)
}

pub(crate) fn add(
    identity: &SnapshotIdentity,
    left: f64,
    right: f64,
) -> Result<f64, LayoutSnapshotError> {
    let result = left + right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(LayoutSnapshotError::EdgeArithmeticOverflow {
            identity: identity.clone(),
            operation: ArithmeticOperation::Add,
            lhs_bits: left.to_bits(),
            rhs_bits: right.to_bits(),
        })
    }
}

pub(crate) fn subtract(
    identity: &SnapshotIdentity,
    left: f64,
    right: f64,
) -> Result<f64, LayoutSnapshotError> {
    let result = left - right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(LayoutSnapshotError::EdgeArithmeticOverflow {
            identity: identity.clone(),
            operation: ArithmeticOperation::Subtract,
            lhs_bits: left.to_bits(),
            rhs_bits: right.to_bits(),
        })
    }
}

/// Floor an absolute half-open edge into the containing terminal cell.
pub(crate) fn edge(
    identity: &SnapshotIdentity,
    edge: Edge,
    value: f64,
) -> Result<i32, LayoutSnapshotError> {
    let floored = value.floor();
    if floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return Err(LayoutSnapshotError::CellCoordinateOverflow {
            identity: identity.clone(),
            edge,
            rounded_bits: floored.to_bits(),
        });
    }
    Ok(floored as i32)
}

pub(crate) fn rect(
    identity: &SnapshotIdentity,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
) -> Result<CellRect, LayoutSnapshotError> {
    let left = edge(identity, Edge::Left, left)?;
    let top = edge(identity, Edge::Top, top)?;
    let right = edge(identity, Edge::Right, right)?;
    let bottom = edge(identity, Edge::Bottom, bottom)?;
    if left > right || i64::from(right) - i64::from(left) > i64::from(i32::MAX) {
        return Err(LayoutSnapshotError::CellSpanOverflow {
            identity: identity.clone(),
            axis: Axis::X,
            start: left,
            end: right,
        });
    }
    if top > bottom || i64::from(bottom) - i64::from(top) > i64::from(i32::MAX) {
        return Err(LayoutSnapshotError::CellSpanOverflow {
            identity: identity.clone(),
            axis: Axis::Y,
            start: top,
            end: bottom,
        });
    }
    CellRect::checked(left, top, right, bottom).ok_or(LayoutSnapshotError::CellSpanOverflow {
        identity: identity.clone(),
        axis: Axis::X,
        start: left,
        end: right,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BorderStyle, Dimension, Element, FlexDirection, Overflow};
    use crate::layout::{LayoutEngine, SnapshotBuildStrategy};
    use crate::reconciler::ScopedNodeIdentity;

    fn identity() -> SnapshotIdentity {
        SnapshotIdentity::from_scoped(ScopedNodeIdentity::Root)
    }

    #[test]
    fn half_open_bounds_derive_extent_from_edges() {
        let identity = identity();
        let bounds = rect(&identity, -0.25, 0.0, 1.75, 2.99).unwrap();
        assert_eq!((bounds.left(), bounds.right(), bounds.width()), (-1, 1, 2));
        assert_eq!((bounds.top(), bounds.bottom(), bounds.height()), (0, 2, 2));
    }

    #[test]
    fn content_border_and_gap_error_are_bounded() {
        const RAW_GAP: f64 = 1.25;
        let bordered_child = |key: &str, text: &str| {
            let mut child = Element::box_element().with_key(key);
            child.style.width = Dimension::Points(7.5);
            child.style.height = Dimension::Points(4.0);
            child.style.padding.left = 0.75;
            child.style.padding.right = 0.75;
            child.style.border_style = BorderStyle::Single;
            child.style.border_top = true;
            child.style.border_right = true;
            child.style.border_bottom = true;
            child.style.border_left = true;
            child.style.overflow_x = Overflow::Hidden;
            child.add_child(Element::text(text));
            child
        };
        let mut target = Element::box_element().with_key("root");
        target.style.width = Dimension::Points(30.0);
        target.style.height = Dimension::Points(4.0);
        target.style.flex_direction = FlexDirection::Row;
        target.style.column_gap = Some(RAW_GAP as f32);
        target.add_child(bordered_child("left", "left-content"));
        target.add_child(bordered_child("right", "right-content"));

        let frame = LayoutEngine::new()
            .prepare_element_incremental(&target, None, 30, 6)
            .expect("real Element border/padding/gap path builds a checked snapshot");
        assert_eq!(
            frame.snapshot_report().strategy(),
            SnapshotBuildStrategy::InitialFull
        );
        let snapshot = frame.snapshot();
        assert_eq!(
            frame.snapshot_report().work_counters().snapshot_nodes(),
            snapshot.nodes().len() as u64
        );
        let children = snapshot.root().children();
        assert_eq!(children.len(), 2);
        let left = snapshot.node(children[0]);
        let right = snapshot.node(children[1]);
        for child in [left, right] {
            let border = child.border_bounds();
            let content = child.content_bounds();
            assert!(border.contains(content));
            assert!(content.left() > border.left());
            assert!(content.right() < border.right());
            assert!(!content.is_empty());
        }
        let cell_gap = right.border_bounds().left() - left.border_bounds().right();
        assert_eq!(cell_gap, 1);
        assert!((RAW_GAP - f64::from(cell_gap)).abs() < 1.0);
    }
}
