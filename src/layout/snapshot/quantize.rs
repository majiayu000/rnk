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
    Ok(CellRect::checked(
        edge(identity, Edge::Left, left)?,
        edge(identity, Edge::Top, top)?,
        edge(identity, Edge::Right, right)?,
        edge(identity, Edge::Bottom, bottom)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let left = rect(&identity, 0.1, 0.0, 3.9, 1.0).unwrap();
        let right = rect(&identity, 3.9, 0.0, 8.2, 1.0).unwrap();
        assert!(left.right() <= right.left());
        assert_eq!(left.width(), 3);
        assert_eq!(right.width(), 5);
    }
}
