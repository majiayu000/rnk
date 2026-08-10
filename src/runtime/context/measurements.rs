//! Checked preparation and infallible publication of runtime measurements.

use std::collections::HashMap;

use crate::{
    core::ElementId,
    layout::{Axis, CellOutputError, CheckedMeasurementSnapshot, Layout},
    reconciler::{ScopedNodeIdentity, SiblingIdentity},
};

use super::RuntimeContext;

pub(crate) struct PreparedMeasurementPublication {
    element: HashMap<ElementId, (u16, u16)>,
    node: HashMap<SiblingIdentity, (u16, u16)>,
    scoped: HashMap<ScopedNodeIdentity, (u16, u16)>,
    node_candidates: HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>>,
    key_aliases: HashMap<String, Vec<(ScopedNodeIdentity, SiblingIdentity)>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeasurementPublicationError {
    pub(crate) identity: ScopedNodeIdentity,
    pub(crate) source: CellOutputError,
}

fn checked_extent(axis: Axis, value: i32) -> Result<u16, CellOutputError> {
    if value < 0 {
        return Err(CellOutputError::NegativeAfterClip { axis, value });
    }
    u16::try_from(value).map_err(|_| CellOutputError::ExtentOutOfRange {
        axis,
        start: 0,
        end: value,
    })
}

fn checked_measurement(width: i32, height: i32) -> Result<(u16, u16), CellOutputError> {
    Ok((
        checked_extent(Axis::X, width)?,
        checked_extent(Axis::Y, height)?,
    ))
}

fn checked_legacy_extent(axis: Axis, value: f32) -> Result<u16, CellOutputError> {
    if !value.is_finite() || value > i32::MAX as f32 {
        return Err(CellOutputError::ExtentOutOfRange {
            axis,
            start: 0,
            end: i32::MAX,
        });
    }
    if value < 0.0 {
        return Err(CellOutputError::NegativeAfterClip {
            axis,
            value: value.floor() as i32,
        });
    }
    checked_extent(axis, value.trunc() as i32)
}

fn checked_legacy(layout: Layout) -> Result<(u16, u16), CellOutputError> {
    Ok((
        checked_legacy_extent(Axis::X, layout.width)?,
        checked_legacy_extent(Axis::Y, layout.height)?,
    ))
}

impl RuntimeContext {
    pub(crate) fn prepare_measurement_publication(
        measurements: CheckedMeasurementSnapshot,
        node_candidates: HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>>,
        key_aliases: HashMap<String, Vec<(ScopedNodeIdentity, SiblingIdentity)>>,
    ) -> Result<PreparedMeasurementPublication, MeasurementPublicationError> {
        let mut scoped = HashMap::with_capacity(measurements.scoped_vnode.len());
        for (identity, (width, height)) in measurements.scoped_vnode {
            let value = checked_measurement(width, height).map_err(|source| {
                MeasurementPublicationError {
                    identity: identity.clone(),
                    source,
                }
            })?;
            scoped.insert(identity, value);
        }
        let mut element = HashMap::with_capacity(measurements.element.len());
        for (element_id, (identity, width, height)) in measurements.element {
            let value = checked_measurement(width, height)
                .map_err(|source| MeasurementPublicationError { identity, source })?;
            element.insert(element_id, value);
        }
        let mut node = HashMap::with_capacity(measurements.vnode.len());
        for (identity, (scoped_identity, width, height)) in measurements.vnode {
            let value = checked_measurement(width, height).map_err(|source| {
                MeasurementPublicationError {
                    identity: scoped_identity,
                    source,
                }
            })?;
            node.insert(identity, value);
        }
        Ok(PreparedMeasurementPublication {
            element,
            node,
            scoped,
            node_candidates,
            key_aliases,
        })
    }

    pub(crate) fn publish_measurements(&mut self, prepared: PreparedMeasurementPublication) {
        self.measurements = prepared.element;
        self.measurements_by_node_key = prepared.node;
        self.measurements_by_scoped_node = prepared.scoped;
        self.measurement_node_candidates = prepared.node_candidates;
        self.measurements_by_key.clear();
        self.measurement_key_aliases.clear();
        self.scoped_measurement_aliases = prepared.key_aliases;
    }

    /// Replace all measurements after checked legacy conversion.
    pub fn set_measure_layouts(&mut self, layouts: HashMap<ElementId, Layout>) {
        let element = layouts
            .into_iter()
            .map(|(id, layout)| {
                checked_legacy(layout)
                    .map(|measurement| (id, measurement))
                    .unwrap_or_else(|error| panic!("measurement conversion failed: {error}"))
            })
            .collect();
        self.clear_and_set_legacy(element, HashMap::new(), HashMap::new(), HashMap::new());
    }

    /// Replace all measurements with compatibility string-keyed measurements.
    pub fn set_measure_layouts_with_keys(
        &mut self,
        layouts: HashMap<ElementId, Layout>,
        keyed_layouts: HashMap<String, Layout>,
    ) {
        let element = checked_legacy_map(layouts);
        let keyed = checked_legacy_map(keyed_layouts);
        self.clear_and_set_legacy(element, HashMap::new(), keyed, HashMap::new());
    }

    /// Replace all measurements with stable node-keyed measurements plus aliases.
    pub fn set_measure_layouts_with_node_keys(
        &mut self,
        layouts: HashMap<ElementId, Layout>,
        node_keyed_layouts: HashMap<SiblingIdentity, Layout>,
        key_aliases: HashMap<String, SiblingIdentity>,
    ) {
        self.clear_and_set_legacy(
            checked_legacy_map(layouts),
            checked_legacy_map(node_keyed_layouts),
            HashMap::new(),
            key_aliases,
        );
    }

    fn clear_and_set_legacy(
        &mut self,
        element: HashMap<ElementId, (u16, u16)>,
        node: HashMap<SiblingIdentity, (u16, u16)>,
        keyed: HashMap<String, (u16, u16)>,
        aliases: HashMap<String, SiblingIdentity>,
    ) {
        self.measurements = element;
        self.measurements_by_node_key = node;
        self.measurements_by_scoped_node.clear();
        self.measurement_node_candidates.clear();
        self.measurements_by_key = keyed;
        self.measurement_key_aliases = aliases;
        self.scoped_measurement_aliases.clear();
    }
}

fn checked_legacy_map<K: Eq + std::hash::Hash>(
    layouts: HashMap<K, Layout>,
) -> HashMap<K, (u16, u16)> {
    layouts
        .into_iter()
        .map(|(key, layout)| {
            checked_legacy(layout)
                .map(|measurement| (key, measurement))
                .unwrap_or_else(|error| panic!("measurement conversion failed: {error}"))
        })
        .collect()
}
