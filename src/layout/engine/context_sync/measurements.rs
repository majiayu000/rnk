//! Exact snapshot-derived measurement projection.

use std::collections::HashMap;

use crate::{
    core::ElementId,
    layout::{IncrementalInvariantError, LayoutLookupError, PreparedSnapshotFrame},
    reconciler::{ScopedNodeIdentity, SiblingIdentity},
};

use super::{LayoutEngine, LegacyLayoutSnapshotError};

pub(crate) struct CheckedMeasurementSnapshot {
    pub(crate) element: HashMap<ElementId, (ScopedNodeIdentity, i32, i32)>,
    pub(crate) scoped_vnode: HashMap<ScopedNodeIdentity, (i32, i32)>,
    pub(crate) vnode: HashMap<SiblingIdentity, (ScopedNodeIdentity, i32, i32)>,
}

impl LayoutEngine {
    pub(crate) fn try_get_snapshot_measurements(
        &self,
        frame: &PreparedSnapshotFrame,
    ) -> Result<CheckedMeasurementSnapshot, LegacyLayoutSnapshotError> {
        let mut element = HashMap::new();
        let mut scoped_vnode = HashMap::new();
        let mut vnode = HashMap::new();
        let mut projected_scopes = HashMap::new();

        for (element_id, node) in frame.element_nodes() {
            let bounds = node.border_bounds();
            let scoped = node.identity().scoped().clone();
            let measurement = (bounds.width(), bounds.height());
            element.insert(element_id, (scoped.clone(), measurement.0, measurement.1));

            let legacy_key = self.vnode_legacy_keys.get(&scoped).copied().ok_or(
                LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::CompatibilityMapMismatch,
                ),
            )?;
            let projected = scoped.composite_identity(legacy_key);
            if let Some(existing) = projected_scopes.insert(projected, scoped.clone())
                && existing != scoped
            {
                return Err(LegacyLayoutSnapshotError::Lookup(
                    LayoutLookupError::CompositeIdentityCollision {
                        identity: projected,
                    },
                ));
            }
            scoped_vnode.insert(scoped.clone(), measurement);
            vnode.insert(projected, (scoped, measurement.0, measurement.1));
        }

        Ok(CheckedMeasurementSnapshot {
            element,
            scoped_vnode,
            vnode,
        })
    }
}
