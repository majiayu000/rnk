//! Scoped identity indexes and checked compatibility projections.

use std::collections::HashMap;

use crate::core::NodeKey;
use crate::layout::Layout;
use crate::reconciler::{ScopedNodeIdentity, SiblingIdentity};

use super::LayoutEngine;
use super::patch_error::LayoutLookupError;

impl LayoutEngine {
    pub(super) fn resolve_legacy_scope(
        &self,
        key: NodeKey,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError> {
        let matches: Vec<_> = self
            .vnode_legacy_keys
            .iter()
            .filter(|(_, candidate)| candidate.identity() == key.identity())
            .map(|(identity, _)| identity.clone())
            .collect();
        match matches.as_slice() {
            [] => Ok(None),
            [identity] => Ok(Some(identity.clone())),
            _ => Err(LayoutLookupError::AmbiguousLegacyNodeKey {
                key,
                scoped_match_count: matches.len(),
            }),
        }
    }

    pub(super) fn resolve_child_legacy_scope(
        &self,
        parent: &ScopedNodeIdentity,
        key: NodeKey,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError> {
        let matches: Vec<_> = self
            .vnode_legacy_keys
            .iter()
            .filter(|(identity, candidate)| {
                identity.parent() == Some(parent) && candidate.identity() == key.identity()
            })
            .map(|(identity, _)| identity.clone())
            .collect();
        match matches.as_slice() {
            [] => Ok(None),
            [identity] => Ok(Some(identity.clone())),
            _ => Err(LayoutLookupError::AmbiguousLegacyNodeKey {
                key,
                scoped_match_count: matches.len(),
            }),
        }
    }

    /// Checked layout lookup for a legacy sibling-local key.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutLookupError::AmbiguousLegacyNodeKey`] when the raw key
    /// matches nodes in more than one parent scope.
    pub fn try_get_vnode_layout(&self, key: NodeKey) -> Result<Option<Layout>, LayoutLookupError> {
        let Some(identity) = self.resolve_legacy_scope(key)? else {
            return Ok(None);
        };
        let Some(node_id) = self.vnode_map.get(&identity) else {
            return Ok(None);
        };
        let Some(layout) = self.taffy.layout(*node_id).ok() else {
            return Ok(None);
        };
        Ok(Some(Layout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        }))
    }

    /// Compatibility layout lookup.
    ///
    /// # Panics
    ///
    /// Panics when the raw key is ambiguous. Use
    /// [`try_get_vnode_layout`](Self::try_get_vnode_layout) for a checked result.
    pub fn get_vnode_layout(&self, key: NodeKey) -> Option<Layout> {
        self.try_get_vnode_layout(key)
            .unwrap_or_else(|error| panic!("VNode layout lookup failed: {error}"))
    }

    /// Checked composite compatibility projection of all scoped layouts.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutLookupError::CompositeIdentityCollision`] if two exact
    /// scopes project to the same compatibility identity.
    pub fn try_get_all_vnode_layouts(
        &self,
    ) -> Result<HashMap<SiblingIdentity, Layout>, LayoutLookupError> {
        let mut layouts = HashMap::with_capacity(self.vnode_map.len());
        let mut projected_scopes = HashMap::with_capacity(self.vnode_map.len());
        for (identity, node_id) in &self.vnode_map {
            let Some(legacy_key) = self.vnode_legacy_keys.get(identity).copied() else {
                continue;
            };
            let projected = identity.composite_identity(legacy_key);
            if let Some(existing) = projected_scopes.insert(projected, identity)
                && existing != identity
            {
                return Err(LayoutLookupError::CompositeIdentityCollision {
                    identity: projected,
                });
            }
            let Some(layout) = self.taffy.layout(*node_id).ok() else {
                continue;
            };
            layouts.insert(
                projected,
                Layout {
                    x: layout.location.x,
                    y: layout.location.y,
                    width: layout.size.width,
                    height: layout.size.height,
                },
            );
        }
        Ok(layouts)
    }

    /// Composite compatibility projection of all scoped layouts.
    ///
    /// # Panics
    ///
    /// Panics on a composite projection collision. Use
    /// [`try_get_all_vnode_layouts`](Self::try_get_all_vnode_layouts) for a
    /// checked result.
    pub fn get_all_vnode_layouts(&self) -> HashMap<SiblingIdentity, Layout> {
        self.try_get_all_vnode_layouts()
            .unwrap_or_else(|error| panic!("VNode layout projection failed: {error}"))
    }

    pub(crate) fn get_all_scoped_vnode_layouts(&self) -> HashMap<ScopedNodeIdentity, Layout> {
        self.vnode_map
            .iter()
            .filter_map(|(identity, node_id)| {
                let layout = self.taffy.layout(*node_id).ok()?;
                Some((
                    identity.clone(),
                    Layout {
                        x: layout.location.x,
                        y: layout.location.y,
                        width: layout.size.width,
                        height: layout.size.height,
                    },
                ))
            })
            .collect()
    }

    pub(crate) fn scoped_projection_for_element(
        &self,
        element_id: crate::core::ElementId,
    ) -> Option<(ScopedNodeIdentity, SiblingIdentity)> {
        let identity = self.element_scopes.get(&element_id)?.clone();
        let legacy_key = self.vnode_legacy_keys.get(&identity).copied()?;
        let projection = identity.composite_identity(legacy_key);
        Some((identity, projection))
    }

    pub(crate) fn raw_vnode_identity_candidates(
        &self,
    ) -> HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>> {
        let mut candidates = HashMap::new();
        for (identity, key) in &self.vnode_legacy_keys {
            candidates
                .entry(key.identity())
                .or_insert_with(Vec::new)
                .push(identity.clone());
        }
        candidates
    }
}
