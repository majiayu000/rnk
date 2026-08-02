//! Pure, checked, parent-scoped reconciliation planning.
mod patch_schedule;

use super::identity::{
    CanonicalKey, ResolvedNodeIdentity, ScopedIdentityArena, ScopedNodeIdentity, SiblingIdentity,
    SiblingMatchKey, compatibility_token_for_exact, resolve_child_identity,
};
use super::{Patch, ReconcilePlanError};
use crate::core::{NodeKey, VNode, VNodeType};
use patch_schedule::schedule_structural_patches;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct ValidatedNode<'a> {
    vnode: &'a VNode,
    identity: ResolvedNodeIdentity,
    children: Vec<ValidatedNode<'a>>,
}

impl ValidatedNode<'_> {
    fn match_key(&self) -> SiblingMatchKey {
        self.identity.match_key()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedNodeAction {
    Reuse,
    Update,
    Create,
    Replace,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlannedNodeMutations {
    pub(crate) style: bool,
    pub(crate) text_context: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct PlannedNode {
    pub(crate) identity: ScopedNodeIdentity,
    pub(crate) old_identity: Option<ScopedNodeIdentity>,
    pub(crate) legacy_key: NodeKey,
    pub(crate) vnode: VNode,
    pub(crate) action: PlannedNodeAction,
    pub(crate) mutations: PlannedNodeMutations,
    pub(crate) children: Vec<PlannedNode>,
}
#[derive(Debug, Clone)]
pub(crate) struct ParentPlan {
    pub(crate) parent: ScopedNodeIdentity,
    pub(crate) final_children: Vec<ScopedNodeIdentity>,
    pub(crate) survivors: Vec<ScopedNodeIdentity>,
    pub(crate) creates: Vec<ScopedNodeIdentity>,
    pub(crate) removals: Vec<ScopedNodeIdentity>,
}
impl ParentPlan {
    fn validate(&self) -> Result<(), ReconcilePlanError> {
        let parent_scope = self.parent.diagnostic();
        let mut final_indices = HashMap::new();
        for (index, identity) in self.final_children.iter().enumerate() {
            if let Some(first_index) = final_indices.insert(identity.clone(), index) {
                return Err(ReconcilePlanError::DuplicateFinalIdentity {
                    parent_scope,
                    identity: identity.diagnostic(),
                    first_index,
                    second_index: index,
                });
            }
        }
        let mut source_counts = HashMap::new();
        for identity in self.survivors.iter().chain(&self.creates) {
            *source_counts.entry(identity.clone()).or_insert(0usize) += 1;
        }
        for identity in &self.final_children {
            match source_counts.get(identity).copied().unwrap_or(0) {
                0 => {
                    return Err(ReconcilePlanError::MissingFinalIdentity {
                        parent_scope,
                        identity: identity.diagnostic(),
                    });
                }
                1 => {}
                _ => {
                    return Err(ReconcilePlanError::DuplicateFinalIdentitySource {
                        parent_scope,
                        identity: identity.diagnostic(),
                    });
                }
            }
        }
        let final_set: HashSet<_> = self.final_children.iter().cloned().collect();
        for identity in source_counts.keys() {
            if !final_set.contains(identity) {
                return Err(ReconcilePlanError::ExtraPlannedIdentity {
                    parent_scope,
                    identity: identity.diagnostic(),
                });
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub(crate) struct ReconcilePlan {
    pub(crate) root: PlannedNode,
    pub(crate) parents: Vec<ParentPlan>,
    patches: Vec<Patch>,
}
impl ReconcilePlan {
    pub(crate) fn patches(&self) -> &[Patch] {
        &self.patches
    }
    pub(crate) fn into_patches(self) -> Vec<Patch> {
        self.patches
    }
    pub(crate) fn validate_final_orders(&self) -> Result<(), ReconcilePlanError> {
        let mut parent_indices = HashMap::new();
        for parent in &self.parents {
            parent.validate()?;
            let next_index = parent_indices.len();
            if parent_indices
                .insert(parent.parent.clone(), next_index)
                .is_some()
            {
                return Err(ReconcilePlanError::DuplicateParentPlan {
                    parent_scope: parent.parent.diagnostic(),
                });
            }
        }
        let mut planned_identities = HashSet::new();
        let mut used_parent_plans = HashSet::new();
        self.validate_planned_children(
            &self.root,
            &parent_indices,
            &mut planned_identities,
            &mut used_parent_plans,
        )?;
        for parent in &self.parents {
            if !used_parent_plans.contains(&parent.parent) {
                return Err(ReconcilePlanError::ExtraParentPlan {
                    parent_scope: parent.parent.diagnostic(),
                });
            }
        }
        self.validate_composite_projections_with(&composite_projection)?;
        Ok(())
    }
    fn validate_planned_children(
        &self,
        planned: &PlannedNode,
        parent_indices: &HashMap<ScopedNodeIdentity, usize>,
        planned_identities: &mut HashSet<ScopedNodeIdentity>,
        used_parent_plans: &mut HashSet<ScopedNodeIdentity>,
    ) -> Result<(), ReconcilePlanError> {
        if !planned_identities.insert(planned.identity.clone()) {
            return Err(ReconcilePlanError::DuplicatePlannedIdentity {
                identity: planned.identity.diagnostic(),
            });
        }
        let Some(parent_index) = parent_indices.get(&planned.identity).copied() else {
            return Err(ReconcilePlanError::MissingParentPlan {
                parent_scope: planned.identity.diagnostic(),
            });
        };
        used_parent_plans.insert(planned.identity.clone());
        let planned_order: Vec<_> = planned
            .children
            .iter()
            .map(|child| child.identity.clone())
            .collect();
        if planned_order != self.parents[parent_index].final_children {
            return Err(ReconcilePlanError::PlannedChildrenMismatch {
                parent_scope: planned.identity.diagnostic(),
            });
        }
        for child in &planned.children {
            self.validate_planned_children(
                child,
                parent_indices,
                planned_identities,
                used_parent_plans,
            )?;
        }
        Ok(())
    }
    pub(crate) fn validate_composite_projections_with(
        &self,
        projection: &dyn Fn(&PlannedNode) -> SiblingIdentity,
    ) -> Result<(), ReconcilePlanError> {
        Self::validate_composite_projection_roots_with(&[&self.root], projection)
    }
    pub(crate) fn validate_composite_projection_union_with(
        &self,
        other: &Self,
        projection: &dyn Fn(&PlannedNode) -> SiblingIdentity,
    ) -> Result<(), ReconcilePlanError> {
        Self::validate_composite_projection_roots_with(&[&self.root, &other.root], projection)
    }
    fn validate_composite_projection_roots_with(
        roots: &[&PlannedNode],
        projection: &dyn Fn(&PlannedNode) -> SiblingIdentity,
    ) -> Result<(), ReconcilePlanError> {
        let mut projections = HashMap::new();
        let mut pending = roots.to_vec();
        while let Some(planned) = pending.pop() {
            let projected = projection(planned);
            if let Some(first_scope) = projections.insert(projected, planned.identity.clone())
                && first_scope != planned.identity
            {
                return Err(ReconcilePlanError::CompositeIdentityCollision {
                    identity: projected,
                    first_scope: first_scope.diagnostic(),
                    second_scope: planned.identity.diagnostic(),
                });
            }
            pending.extend(&planned.children);
        }
        Ok(())
    }
}

fn composite_projection(planned: &PlannedNode) -> SiblingIdentity {
    planned.identity.composite_identity(planned.legacy_key)
}

pub(crate) fn plan_initial_tree(target: &VNode) -> Result<ReconcilePlan, ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::default();
    plan_initial_tree_in(target, &mut arena)
}

pub(crate) fn plan_initial_tree_in(
    target: &VNode,
    arena: &mut ScopedIdentityArena,
) -> Result<ReconcilePlan, ReconcilePlanError> {
    plan_initial_tree_with_token_source(target, &compatibility_token_for_exact, arena)
}

fn plan_initial_tree_with_token_source(
    target: &VNode,
    token_source: &dyn Fn(&str) -> u64,
    arena: &mut ScopedIdentityArena,
) -> Result<ReconcilePlan, ReconcilePlanError> {
    let validated_children = validate_tree_with_token_source(target, token_source, arena)?;
    let mut parents = Vec::new();
    let root = create_subtree(
        target,
        ScopedNodeIdentity::Root,
        target.key,
        PlannedNodeAction::Create,
        &validated_children,
        &mut parents,
    );
    let plan = ReconcilePlan {
        root,
        parents,
        patches: Vec::new(),
    };
    plan.validate_final_orders()?;
    Ok(plan)
}

pub(crate) fn plan_diff(old: &VNode, new: &VNode) -> Result<ReconcilePlan, ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::default();
    plan_diff_in(old, new, &mut arena)
}

pub(crate) fn plan_diff_in(
    old: &VNode,
    new: &VNode,
    arena: &mut ScopedIdentityArena,
) -> Result<ReconcilePlan, ReconcilePlanError> {
    plan_diff_with_token_source_in(old, new, &compatibility_token_for_exact, arena)
}

#[cfg(test)]
pub(crate) fn constant_token(_: &str) -> u64 {
    7
}

#[cfg(test)]
pub(crate) fn plan_diff_with_token_source(
    old: &VNode,
    new: &VNode,
    token_source: &dyn Fn(&str) -> u64,
) -> Result<ReconcilePlan, ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::default();
    plan_diff_with_token_source_in(old, new, token_source, &mut arena)
}

fn plan_diff_with_token_source_in(
    old: &VNode,
    new: &VNode,
    token_source: &dyn Fn(&str) -> u64,
    arena: &mut ScopedIdentityArena,
) -> Result<ReconcilePlan, ReconcilePlanError> {
    let old_children = validate_tree_with_token_source(old, token_source, arena)?;
    let new_children = validate_tree_with_token_source(new, token_source, arena)?;

    let mut planner = Planner {
        patches: Vec::new(),
        parents: Vec::new(),
        scoped_patch_addresses: true,
    };
    let root = planner.plan_root(old, new, &old_children, &new_children)?;
    let mut old_parents = Vec::new();
    let old_plan = ReconcilePlan {
        root: create_subtree(
            old,
            ScopedNodeIdentity::Root,
            old.key,
            PlannedNodeAction::Create,
            &old_children,
            &mut old_parents,
        ),
        parents: old_parents,
        patches: Vec::new(),
    };
    let plan = ReconcilePlan {
        root,
        parents: planner.parents,
        patches: planner.patches,
    };
    plan.validate_final_orders()?;
    plan.validate_composite_projection_union_with(&old_plan, &composite_projection)?;
    Ok(plan)
}

pub(crate) fn plan_child_patches(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
) -> Result<Vec<Patch>, ReconcilePlanError> {
    plan_child_patches_with_token_source(
        old_children,
        new_children,
        parent_key,
        &compatibility_token_for_exact,
    )
}

pub(crate) fn plan_child_patches_with_token_source(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
    token_source: &dyn Fn(&str) -> u64,
) -> Result<Vec<Patch>, ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::default();
    let root = ScopedNodeIdentity::Root;
    let old_children = validate_siblings(old_children, &root, token_source, &mut arena)?;
    let new_children = validate_siblings(new_children, &root, token_source, &mut arena)?;
    let mut planner = Planner {
        patches: Vec::new(),
        parents: Vec::new(),
        scoped_patch_addresses: false,
    };
    planner.plan_children(&old_children, &new_children, &root, parent_key)?;
    for parent in &planner.parents {
        parent.validate()?;
    }
    Ok(planner.patches)
}

fn validate_tree_with_token_source<'a>(
    target: &'a VNode,
    token_source: &dyn Fn(&str) -> u64,
    arena: &mut ScopedIdentityArena,
) -> Result<Vec<ValidatedNode<'a>>, ReconcilePlanError> {
    validate_siblings(
        &target.children,
        &ScopedNodeIdentity::Root,
        token_source,
        arena,
    )
}

fn validate_siblings<'a>(
    children: &'a [VNode],
    parent: &ScopedNodeIdentity,
    token_source: &dyn Fn(&str) -> u64,
    arena: &mut ScopedIdentityArena,
) -> Result<Vec<ValidatedNode<'a>>, ReconcilePlanError> {
    let mut canonical_indices: HashMap<SiblingMatchKey, usize> = HashMap::new();
    let mut token_sources: HashMap<u64, (CanonicalKey, usize)> = HashMap::new();
    let projected_token = |value: &str| token_source(value);

    let mut validated = Vec::with_capacity(children.len());
    for (index, child) in children.iter().enumerate() {
        let resolved = resolve_child_identity(child, index, parent, &projected_token, arena)?;
        if let Some((token, canonical_key)) = resolved.canonical_projection() {
            if let Some((first_source, first_index)) = token_sources.get(&token)
                && first_source != canonical_key
            {
                return Err(ReconcilePlanError::KeyTokenCollision {
                    parent_scope: parent.diagnostic(),
                    token,
                    first_index: *first_index,
                    second_index: index,
                });
            }
            if let Some(first_index) = canonical_indices.insert(resolved.match_key(), index) {
                return Err(ReconcilePlanError::DuplicateSiblingKey {
                    parent_scope: parent.diagnostic(),
                    key_kind: canonical_key.diagnostic_kind(),
                    token,
                    first_index,
                    second_index: index,
                });
            }
            token_sources.insert(token, (canonical_key.clone(), index));
        }

        let validated_children =
            validate_siblings(&child.children, &resolved.scoped, token_source, arena)?;
        validated.push(ValidatedNode {
            vnode: child,
            identity: resolved,
            children: validated_children,
        });
    }
    Ok(validated)
}

struct Planner {
    patches: Vec<Patch>,
    parents: Vec<ParentPlan>,
    scoped_patch_addresses: bool,
}

impl Planner {
    fn plan_root(
        &mut self,
        old: &VNode,
        new: &VNode,
        old_children: &[ValidatedNode<'_>],
        new_children: &[ValidatedNode<'_>],
    ) -> Result<PlannedNode, ReconcilePlanError> {
        if node_requires_replacement(old, new) {
            self.patches.push(Patch::replace(
                self.patch_address(&ScopedNodeIdentity::Root, old.key),
                new.clone(),
            ));
            let mut replacement = create_subtree(
                new,
                ScopedNodeIdentity::Root,
                new.key,
                PlannedNodeAction::Replace,
                new_children,
                &mut self.parents,
            );
            replacement.old_identity = Some(ScopedNodeIdentity::Root);
            return Ok(replacement);
        }

        let action = if !old.props.semantically_eq(&new.props) {
            self.patches.push(Patch::update(
                self.patch_address(&ScopedNodeIdentity::Root, old.key),
                old.props.clone(),
                new.props.clone(),
            ));
            PlannedNodeAction::Update
        } else {
            PlannedNodeAction::Reuse
        };
        let children = self.plan_children(
            old_children,
            new_children,
            &ScopedNodeIdentity::Root,
            self.patch_address(&ScopedNodeIdentity::Root, old.key),
        )?;
        Ok(PlannedNode {
            identity: ScopedNodeIdentity::Root,
            old_identity: Some(ScopedNodeIdentity::Root),
            legacy_key: new.key,
            vnode: shallow_node_snapshot(new, None),
            action,
            mutations: planned_mutations(old, new),
            children,
        })
    }

    fn plan_children(
        &mut self,
        old_children: &[ValidatedNode<'_>],
        new_children: &[ValidatedNode<'_>],
        parent: &ScopedNodeIdentity,
        parent_address: NodeKey,
    ) -> Result<Vec<PlannedNode>, ReconcilePlanError> {
        let parent_patch_start = self.patches.len();
        validate_cross_frame_projections(parent, old_children, new_children)?;
        let old_by_match: HashMap<_, _> = old_children
            .iter()
            .enumerate()
            .map(|(index, child)| (child.match_key(), index))
            .collect();
        let old_by_token: HashMap<_, _> = old_children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| {
                child
                    .identity
                    .compatibility_token()
                    .map(|token| (token, index))
            })
            .collect();
        let mut matched_old = vec![false; old_children.len()];
        let mut planned_children = Vec::with_capacity(new_children.len());
        let mut final_children = Vec::with_capacity(new_children.len());
        let mut survivors = Vec::new();
        let mut creates = Vec::new();
        let mut create_patches = Vec::new();
        let mut already_in_order = true;
        let mut next_untouched_old = 0usize;
        let mut seen_create = false;

        for (new_index, new_child) in new_children.iter().enumerate() {
            let new_identity = &new_child.identity;
            let direct_old_index = old_by_match.get(&new_identity.match_key()).copied();
            let projected_old_index = new_identity.compatibility_token().and_then(|token| {
                old_by_token.get(&token).copied().filter(|old_index| {
                    is_source_domain_conversion(&old_children[*old_index].identity, new_identity)
                })
            });
            let old_index = direct_old_index.or(projected_old_index);
            let planned = if let Some(old_index) = old_index {
                if seen_create || old_index != next_untouched_old {
                    already_in_order = false;
                }
                next_untouched_old = old_index + 1;
                matched_old[old_index] = true;
                self.plan_matched_child(
                    &old_children[old_index],
                    new_child,
                    direct_old_index.is_none(),
                )?
            } else {
                seen_create = true;
                create_patches.push(Patch::Create {
                    key: new_identity.legacy_key,
                    parent: parent_address,
                    props: new_child.vnode.props.clone(),
                    node: normalized_child_clone(new_child.vnode, new_index),
                });
                create_subtree(
                    new_child.vnode,
                    new_identity.scoped.clone(),
                    new_identity.legacy_key,
                    PlannedNodeAction::Create,
                    &new_child.children,
                    &mut self.parents,
                )
            };

            match planned.action {
                PlannedNodeAction::Reuse | PlannedNodeAction::Update => {
                    survivors.push(planned.identity.clone())
                }
                PlannedNodeAction::Create | PlannedNodeAction::Replace => {
                    creates.push(planned.identity.clone())
                }
            }
            final_children.push(planned.identity.clone());
            planned_children.push(planned);
        }

        let mut removals = Vec::new();
        let mut removal_patches = Vec::new();
        for (old_index, matched) in matched_old.into_iter().enumerate() {
            if !matched {
                let old_identity = &old_children[old_index].identity;
                removals.push(old_identity.scoped.clone());
                removal_patches.push((
                    old_index,
                    Patch::remove(
                        self.patch_address(&old_identity.scoped, old_identity.legacy_key),
                    ),
                ));
            }
        }
        let structural_patches = schedule_structural_patches(
            parent,
            old_children,
            &planned_children,
            already_in_order,
            create_patches,
            removal_patches,
        )?;
        self.patches
            .splice(parent_patch_start..parent_patch_start, structural_patches);

        if !already_in_order {
            self.patches.push(Patch::reorder(
                parent_address,
                patch_schedule::reorder_keys(
                    new_children,
                    &planned_children,
                    self.scoped_patch_addresses,
                ),
            ));
        }
        self.parents.push(ParentPlan {
            parent: parent.clone(),
            final_children,
            survivors,
            creates,
            removals,
        });
        Ok(planned_children)
    }

    fn plan_matched_child(
        &mut self,
        old: &ValidatedNode<'_>,
        new: &ValidatedNode<'_>,
        force_replace: bool,
    ) -> Result<PlannedNode, ReconcilePlanError> {
        let (old_identity, new_identity) = (&old.identity, &new.identity);
        let (old_vnode, new_vnode) = (old.vnode, new.vnode);
        let new_index = new_identity.legacy_key.index;
        if force_replace || node_requires_replacement(old_vnode, new_vnode) {
            self.patches.push(Patch::replace(
                self.patch_address(&old_identity.scoped, old_identity.legacy_key),
                normalized_child_clone(new_vnode, new_index),
            ));
            let mut replacement = create_subtree(
                new_vnode,
                new_identity.scoped.clone(),
                new_identity.legacy_key,
                PlannedNodeAction::Replace,
                &new.children,
                &mut self.parents,
            );
            replacement.old_identity = Some(old_identity.scoped.clone());
            return Ok(replacement);
        }

        let action = if !old_vnode.props.semantically_eq(&new_vnode.props) {
            self.patches.push(Patch::update(
                self.patch_address(&old_identity.scoped, old_identity.legacy_key),
                old_vnode.props.clone(),
                new_vnode.props.clone(),
            ));
            PlannedNodeAction::Update
        } else {
            PlannedNodeAction::Reuse
        };
        let children = self.plan_children(
            &old.children,
            &new.children,
            &new_identity.scoped,
            self.patch_address(&old_identity.scoped, old_identity.legacy_key),
        )?;
        Ok(PlannedNode {
            identity: new_identity.scoped.clone(),
            old_identity: Some(old_identity.scoped.clone()),
            legacy_key: new_identity.legacy_key,
            vnode: shallow_node_snapshot(new_vnode, Some(new_index)),
            action,
            mutations: planned_mutations(old_vnode, new_vnode),
            children,
        })
    }

    fn patch_address(&self, identity: &ScopedNodeIdentity, legacy_key: NodeKey) -> NodeKey {
        if self.scoped_patch_addresses {
            identity.scoped_patch_address(legacy_key)
        } else {
            legacy_key
        }
    }
}

fn create_subtree(
    vnode: &VNode,
    identity: ScopedNodeIdentity,
    legacy_key: NodeKey,
    action: PlannedNodeAction,
    validated_children: &[ValidatedNode<'_>],
    parents: &mut Vec<ParentPlan>,
) -> PlannedNode {
    let mut children = Vec::with_capacity(vnode.children.len());
    let mut creates = Vec::with_capacity(vnode.children.len());
    for (index, child) in validated_children.iter().enumerate() {
        let planned = create_subtree(
            child.vnode,
            child.identity.scoped.clone(),
            child.identity.legacy_key,
            PlannedNodeAction::Create,
            &child.children,
            parents,
        );
        creates.push(planned.identity.clone());
        debug_assert_eq!(planned.vnode.key.index, index);
        children.push(planned);
    }
    parents.push(ParentPlan {
        parent: identity.clone(),
        final_children: creates.clone(),
        survivors: Vec::new(),
        creates,
        removals: Vec::new(),
    });
    let actual_index = (identity != ScopedNodeIdentity::Root).then_some(legacy_key.index);
    PlannedNode {
        identity,
        old_identity: None,
        legacy_key,
        vnode: shallow_node_snapshot(vnode, actual_index),
        action,
        mutations: PlannedNodeMutations::default(),
        children,
    }
}

fn validate_cross_frame_projections(
    parent: &ScopedNodeIdentity,
    old_children: &[ValidatedNode<'_>],
    new_children: &[ValidatedNode<'_>],
) -> Result<(), ReconcilePlanError> {
    let old_by_token: HashMap<_, _> = old_children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| {
            child
                .identity
                .canonical_projection()
                .map(|(token, source)| (token, (source, index)))
        })
        .collect();
    for (new_index, new_child) in new_children.iter().enumerate() {
        let Some((token, new_source)) = new_child.identity.canonical_projection() else {
            continue;
        };
        let Some((old_source, old_index)) = old_by_token.get(&token) else {
            continue;
        };
        if *old_source != new_source
            && !matches!(
                (*old_source, new_source),
                (CanonicalKey::Exact(_), CanonicalKey::Opaque(_))
                    | (CanonicalKey::Opaque(_), CanonicalKey::Exact(_))
            )
        {
            return Err(ReconcilePlanError::KeyTokenCollision {
                parent_scope: parent.diagnostic(),
                token,
                first_index: *old_index,
                second_index: new_index,
            });
        }
    }
    Ok(())
}

fn is_source_domain_conversion(old: &ResolvedNodeIdentity, new: &ResolvedNodeIdentity) -> bool {
    matches!(
        (old.canonical_key(), new.canonical_key()),
        (Some(CanonicalKey::Exact(_)), Some(CanonicalKey::Opaque(_)))
            | (Some(CanonicalKey::Opaque(_)), Some(CanonicalKey::Exact(_)))
    )
}

fn planned_mutations(old: &VNode, new: &VNode) -> PlannedNodeMutations {
    let style = !old.props.style.semantically_eq(&new.props.style);
    PlannedNodeMutations {
        style,
        text_context: old.is_text() && style,
    }
}

fn node_requires_replacement(old: &VNode, new: &VNode) -> bool {
    if old.node_type.type_id() != new.node_type.type_id() {
        return true;
    }
    matches!(
        (&old.node_type, &new.node_type),
        (VNodeType::Text(old_text), VNodeType::Text(new_text)) if old_text != new_text
    )
}

fn shallow_node_snapshot(vnode: &VNode, actual_index: Option<usize>) -> VNode {
    let mut key = vnode.key;
    if let Some(actual_index) = actual_index {
        key.index = actual_index;
    }
    VNode {
        key,
        node_type: vnode.node_type.clone(),
        props: vnode.props.clone(),
        children: Vec::new(),
    }
}

fn normalized_child_clone(vnode: &VNode, actual_index: usize) -> VNode {
    let mut normalized = vnode.clone();
    normalized.key.index = actual_index;
    normalized
}

#[cfg(test)]
mod tests;
