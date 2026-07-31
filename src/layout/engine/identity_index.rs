//! Scoped identity indexes and checked compatibility projections.

use std::collections::{HashMap, HashSet};

use crate::core::{NodeKey, VNode};
use crate::layout::Layout;
use crate::reconciler::{
    Patch, ReconcilePlanError, ScopedIdentityArena, ScopedNodeIdentity, SiblingIdentity,
    compatibility_token_for_exact, insert_composite_projection, plan_initial_tree,
    resolve_child_identity,
};

use super::LayoutEngine;
use super::patch_error::{
    DirectPatchError, LayoutLookupError, PatchError, PatchFailure, PatchKind,
};

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

    pub(super) fn preflight_patch_identities(
        &self,
        patches: &[Patch],
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), DirectPatchError> {
        let mut planned_identities = HashSet::new();
        let mut planned_keyed_tokens = HashSet::new();
        let mut prospective_projections = HashMap::new();
        for (identity, legacy_key) in &self.vnode_legacy_keys {
            insert_composite_projection(&mut prospective_projections, identity, *legacy_key)
                .map_err(|(projection, first_scope)| {
                    ReconcilePlanError::CompositeIdentityCollision {
                        identity: projection,
                        first_scope: first_scope.diagnostic(),
                        second_scope: identity.diagnostic(),
                    }
                })?;
        }
        for patch in patches {
            let (kind, node) = match patch {
                Patch::Create { node, .. } => (PatchKind::Create, node),
                Patch::Replace { node, .. } => (PatchKind::Replace, node),
                _ => continue,
            };
            plan_initial_tree(node)?;
            let (parent_scope, resolved, excluded, excluded_nodes) =
                match patch {
                    Patch::Create { parent, .. } => {
                        let parent_scope = self.resolve_patch_scope(*parent, kind)?;
                        let resolved = resolve_child_identity(
                            node,
                            node.key.index,
                            &parent_scope,
                            &compatibility_token_for_exact,
                            arena,
                        )?;
                        (parent_scope, resolved, None, None)
                    }
                    Patch::Replace { key, .. } => {
                        let old_identity = self.resolve_patch_scope(*key, kind)?;
                        let old_node =
                            self.vnode_map.get(&old_identity).copied().ok_or_else(|| {
                                PatchError::new(kind, *key, PatchFailure::UnknownNode)
                            })?;
                        let parent_scope = old_identity.parent().cloned().ok_or_else(|| {
                            PatchError::new(kind, *key, PatchFailure::MissingParent)
                        })?;
                        let parent_node = self.taffy.parent(old_node).ok_or_else(|| {
                            PatchError::new(kind, *key, PatchFailure::MissingParent)
                        })?;
                        let actual_index = self
                            .taffy
                            .children(parent_node)
                            .map_err(|_| PatchError::new(kind, *key, PatchFailure::TreeRejected))?
                            .iter()
                            .position(|candidate| *candidate == old_node)
                            .ok_or_else(|| {
                                PatchError::new(kind, *key, PatchFailure::PostconditionViolated)
                            })?;
                        let resolved = resolve_child_identity(
                            node,
                            actual_index,
                            &parent_scope,
                            &compatibility_token_for_exact,
                            arena,
                        )?;
                        (
                            parent_scope,
                            resolved,
                            Some(old_identity),
                            Some(self.vnode_subtree_nodes(old_node)),
                        )
                    }
                    _ => unreachable!("only create and replace reach identity preflight"),
                };
            self.preflight_new_sibling_identity(
                &parent_scope,
                &resolved.scoped,
                resolved.legacy_key,
                excluded.as_ref(),
            )?;
            if let Some(excluded_nodes) = excluded_nodes {
                for (identity, legacy_key) in &self.vnode_legacy_keys {
                    if self
                        .vnode_map
                        .get(identity)
                        .is_some_and(|node_id| excluded_nodes.contains(node_id))
                    {
                        prospective_projections.remove(&identity.composite_identity(*legacy_key));
                    }
                }
            }
            let mut subtree_identities = Vec::new();
            Self::collect_scoped_subtree_identities(
                node,
                resolved.scoped.clone(),
                resolved.legacy_key,
                &mut subtree_identities,
                arena,
            )?;
            for (identity, legacy_key) in subtree_identities {
                insert_composite_projection(&mut prospective_projections, &identity, legacy_key)
                    .map_err(|(projection, first_scope)| {
                        ReconcilePlanError::CompositeIdentityCollision {
                            identity: projection,
                            first_scope: first_scope.diagnostic(),
                            second_scope: identity.diagnostic(),
                        }
                    })?;
            }
            let duplicate_planned_identity = !planned_identities.insert(resolved.scoped.clone());
            let duplicate_planned_token = resolved
                .legacy_key
                .user_key
                .is_some_and(|token| !planned_keyed_tokens.insert((parent_scope.clone(), token)));
            if duplicate_planned_identity || duplicate_planned_token {
                return Err(ReconcilePlanError::DuplicatePlannedIdentity {
                    identity: resolved.scoped.diagnostic(),
                }
                .into());
            }
        }
        Ok(())
    }

    fn collect_scoped_subtree_identities(
        vnode: &VNode,
        identity: ScopedNodeIdentity,
        legacy_key: NodeKey,
        identities: &mut Vec<(ScopedNodeIdentity, NodeKey)>,
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), DirectPatchError> {
        identities.push((identity.clone(), legacy_key));
        for (index, child) in vnode.children.iter().enumerate() {
            let resolved = resolve_child_identity(
                child,
                index,
                &identity,
                &compatibility_token_for_exact,
                arena,
            )?;
            Self::collect_scoped_subtree_identities(
                child,
                resolved.scoped,
                resolved.legacy_key,
                identities,
                arena,
            )?;
        }
        Ok(())
    }

    fn preflight_new_sibling_identity(
        &self,
        parent_scope: &ScopedNodeIdentity,
        new_identity: &ScopedNodeIdentity,
        new_key: NodeKey,
        excluded: Option<&ScopedNodeIdentity>,
    ) -> Result<(), DirectPatchError> {
        let collides = self.vnode_legacy_keys.iter().any(|(identity, key)| {
            identity.parent() == Some(parent_scope)
                && excluded != Some(identity)
                && match new_key.user_key {
                    Some(token) => key.user_key == Some(token),
                    None => identity == new_identity,
                }
        });
        if collides {
            return Err(ReconcilePlanError::DuplicatePlannedIdentity {
                identity: new_identity.diagnostic(),
            }
            .into());
        }
        Ok(())
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
