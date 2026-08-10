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
    use crate::core::Element;
    use crate::layout::snapshot::{
        AxisClip, CellVector, CheckedSnapshotNodeInput, LayoutSnapshotBuilder,
        SnapshotBuildStrategy,
    };
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
        let identity = identity();
        let border = rect(&identity, 0.1, 0.1, 8.2, 4.8).unwrap();
        let raw_content = rect(&identity, 1.2, 1.1, 7.9, 3.7).unwrap();
        let content = raw_content.intersect(border);
        assert_eq!(
            (border.left(), border.top(), border.right(), border.bottom()),
            (0, 0, 8, 4)
        );
        assert_eq!(
            (
                content.left(),
                content.top(),
                content.right(),
                content.bottom()
            ),
            (1, 1, 7, 3)
        );
        assert!(border.contains(content));

        let outside = rect(&identity, 30.0, 2.0, 35.0, 3.0).unwrap();
        let canonical_empty = outside.intersect(border);
        assert_eq!(
            (
                canonical_empty.left(),
                canonical_empty.top(),
                canonical_empty.right(),
                canonical_empty.bottom()
            ),
            (8, 2, 8, 3)
        );
        assert!(border.contains(canonical_empty));

        let root = Element::root();
        let mut builder = LayoutSnapshotBuilder::new(8, 4, 1);
        builder
            .push_ordered(CheckedSnapshotNodeInput {
                element_id: root.id,
                identity: identity.clone(),
                parent: None,
                border_bounds: border,
                content_bounds: content,
                text_origin: raw_content.origin(),
                effective_clip: AxisClip::from_rect(content),
                scroll_transform: CellVector::checked(0, 0),
                text_flow: None,
            })
            .unwrap();
        let (published, _) = builder
            .finish(SnapshotBuildStrategy::InitialFull, 0, None)
            .unwrap();
        assert_eq!(published.snapshot().root().border_bounds(), border);
        assert_eq!(published.snapshot().root().content_bounds(), content);

        let left = rect(&identity, 0.1, 0.0, 3.9, 1.0).unwrap();
        let right = rect(&identity, 3.9, 0.0, 8.2, 1.0).unwrap();
        assert_eq!(left.right(), right.left());
        assert!(left.right() <= right.left());
        assert_eq!(left.width(), 3);
        assert_eq!(right.width(), 5);
        let shared_edge_error = (3.9_f64 - f64::from(left.right())).abs();
        assert!(shared_edge_error < 1.0);
    }
}
