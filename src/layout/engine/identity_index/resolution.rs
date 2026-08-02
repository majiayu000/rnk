//! Stable-address resolution and tombstones for direct patch preflight.

use std::collections::HashMap;

use crate::core::NodeKey;
use crate::reconciler::{ReconcilePlanError, ScopedNodeIdentity, SiblingIdentity};

use super::VirtualNodeEntry;
use super::direct_helpers::VirtualAliases;
use crate::layout::DirectPatchPreflightCause;

#[derive(Clone, Copy)]
pub(super) enum LookupRole {
    Target,
    Parent,
}

#[derive(Clone, Copy)]
pub(super) enum TombstoneKind {
    Removed,
    Replaced,
}

#[derive(Clone)]
pub(super) struct Tombstone {
    identity: ScopedNodeIdentity,
    legacy_key: NodeKey,
    raw_key: NodeKey,
    scoped_key: NodeKey,
    scoped_aliases: Vec<SiblingIdentity>,
    raw_aliases: Vec<SiblingIdentity>,
    parent_key: Option<NodeKey>,
    patch_index: usize,
    kind: TombstoneKind,
}

pub(super) struct ResolutionFailure {
    pub(super) source: DirectPatchPreflightCause,
    pub(super) parent: Option<NodeKey>,
}

impl ResolutionFailure {
    fn plain(source: DirectPatchPreflightCause) -> Self {
        Self {
            source,
            parent: None,
        }
    }
}

fn entry_matches(entry: &VirtualNodeEntry, key: NodeKey) -> bool {
    if ScopedNodeIdentity::is_scoped_patch_address(key) {
        entry
            .identity
            .scoped_patch_address(entry.legacy_key)
            .identity()
            == key.identity()
    } else {
        entry.legacy_key.identity() == key.identity() || entry.raw_key.identity() == key.identity()
    }
}

fn tombstone_matches(tombstone: &Tombstone, key: NodeKey) -> bool {
    if ScopedNodeIdentity::is_scoped_patch_address(key) {
        tombstone.scoped_key.identity() == key.identity()
            || tombstone.scoped_aliases.contains(&key.identity())
    } else {
        tombstone.legacy_key.identity() == key.identity()
            || tombstone.raw_key.identity() == key.identity()
            || tombstone.raw_aliases.contains(&key.identity())
    }
}

fn matching_tombstones<'a>(
    tombstones: &'a [Tombstone],
    key: NodeKey,
    parent: Option<&ScopedNodeIdentity>,
) -> Vec<&'a Tombstone> {
    let mut latest_by_identity = HashMap::new();
    for tombstone in tombstones {
        if parent.is_none_or(|parent| tombstone.identity.parent() == Some(parent))
            && tombstone_matches(tombstone, key)
        {
            latest_by_identity.insert(tombstone.identity.clone(), tombstone);
        }
    }
    latest_by_identity.into_values().collect()
}

fn dependency(tombstone: &Tombstone) -> DirectPatchPreflightCause {
    match tombstone.kind {
        TombstoneKind::Removed => DirectPatchPreflightCause::DependencyRemoved {
            prior_patch_index: tombstone.patch_index,
        },
        TombstoneKind::Replaced => DirectPatchPreflightCause::DependencyReplaced {
            prior_patch_index: tombstone.patch_index,
        },
    }
}

fn stale_alias_path() -> DirectPatchPreflightCause {
    DirectPatchPreflightCause::Identity(ReconcilePlanError::CommittedTreeMismatch {
        reason: "virtual alias path is absent from the current index",
    })
}

fn paths_have_same_parent(left: &[usize], right: &[usize]) -> bool {
    match (left.split_last(), right.split_last()) {
        (Some((_, left_parent)), Some((_, right_parent))) => left_parent == right_parent,
        _ => false,
    }
}

pub(super) fn resolve_virtual(
    index: &[VirtualNodeEntry],
    key: NodeKey,
    role: LookupRole,
    tombstones: &[Tombstone],
    aliases: &VirtualAliases,
) -> Result<VirtualNodeEntry, ResolutionFailure> {
    let matches = if ScopedNodeIdentity::is_scoped_patch_address(key) {
        match aliases.path_for(key.identity()) {
            Some(path) => vec![
                index
                    .iter()
                    .find(|entry| entry.path == path)
                    .cloned()
                    .ok_or_else(|| ResolutionFailure::plain(stale_alias_path()))?,
            ],
            None => Vec::new(),
        }
    } else {
        let batch_paths = aliases.batch_local_paths_anywhere(key);
        let mut paths = Vec::new();
        for entry in index {
            let batch_owns_positional_generation = key.user_key.is_none()
                && batch_paths
                    .iter()
                    .any(|batch_path| paths_have_same_parent(batch_path, &entry.path));
            if entry_matches(entry, key)
                && !aliases.batch_local_path_has_different_key(&entry.path, key)
                && !batch_owns_positional_generation
            {
                paths.push(entry.path.clone());
            }
        }
        for path in batch_paths {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        let mut resolved = Vec::with_capacity(paths.len());
        for path in paths {
            let entry = index
                .iter()
                .find(|entry| entry.path == path)
                .cloned()
                .ok_or_else(stale_alias_path)
                .map_err(ResolutionFailure::plain)?;
            resolved.push(entry);
        }
        resolved
    };
    let matches = if matches.is_empty() && ScopedNodeIdentity::is_scoped_patch_address(key) {
        index
            .iter()
            .filter(|entry| entry_matches(entry, key))
            .cloned()
            .collect()
    } else {
        matches
    };
    match matches.as_slice() {
        [entry] => Ok(entry.clone()),
        [] => {
            let tombstones = matching_tombstones(tombstones, key, None);
            if let [tombstone] = tombstones.as_slice() {
                return Err(ResolutionFailure {
                    source: dependency(tombstone),
                    parent: match role {
                        LookupRole::Target => tombstone.parent_key,
                        LookupRole::Parent => Some(tombstone.scoped_key),
                    },
                });
            }
            if tombstones.len() > 1 {
                return Err(ResolutionFailure::plain(match role {
                    LookupRole::Target => DirectPatchPreflightCause::AmbiguousTarget {
                        match_count: tombstones.len(),
                    },
                    LookupRole::Parent => DirectPatchPreflightCause::AmbiguousParent {
                        match_count: tombstones.len(),
                    },
                }));
            }
            Err(ResolutionFailure::plain(match role {
                LookupRole::Target => DirectPatchPreflightCause::MissingTarget,
                LookupRole::Parent => DirectPatchPreflightCause::MissingParent,
            }))
        }
        _ => Err(ResolutionFailure::plain(match role {
            LookupRole::Target => DirectPatchPreflightCause::AmbiguousTarget {
                match_count: matches.len(),
            },
            LookupRole::Parent => DirectPatchPreflightCause::AmbiguousParent {
                match_count: matches.len(),
            },
        })),
    }
}

pub(super) fn resolve_virtual_child(
    index: &[VirtualNodeEntry],
    parent: &VirtualNodeEntry,
    key: NodeKey,
    tombstones: &[Tombstone],
    aliases: &VirtualAliases,
) -> Result<VirtualNodeEntry, DirectPatchPreflightCause> {
    let matches: Vec<_> = if ScopedNodeIdentity::is_scoped_patch_address(key) {
        aliases
            .path_for(key.identity())
            .into_iter()
            .map(ToOwned::to_owned)
            .collect()
    } else {
        let batch_paths = aliases.batch_local_paths(key, &parent.path);
        if key.user_key.is_none() && !batch_paths.is_empty() {
            batch_paths
        } else {
            let mut paths = Vec::new();
            for entry in index {
                if entry.path.len() == parent.path.len() + 1
                    && entry.path.starts_with(&parent.path)
                    && entry_matches(entry, key)
                    && !aliases.batch_local_path_has_different_key(&entry.path, key)
                {
                    paths.push(entry.path.clone());
                }
            }
            if paths.is_empty() {
                for path in aliases.matching_paths(key, &parent.path) {
                    if !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
            for path in batch_paths {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            paths
        }
    };
    let mut resolved = Vec::with_capacity(matches.len());
    for path in matches {
        let entry = index
            .iter()
            .find(|entry| entry.path == path)
            .ok_or_else(stale_alias_path)?;
        if entry.path.len() == parent.path.len() + 1 && entry.path.starts_with(&parent.path) {
            resolved.push(entry.clone());
        } else if ScopedNodeIdentity::is_scoped_patch_address(key) {
            continue;
        }
    }
    let matches = resolved;
    let matches = if matches.is_empty() && ScopedNodeIdentity::is_scoped_patch_address(key) {
        index
            .iter()
            .filter(|entry| {
                entry.path.len() == parent.path.len() + 1
                    && entry.path.starts_with(&parent.path)
                    && entry_matches(entry, key)
            })
            .cloned()
            .collect()
    } else {
        matches
    };
    match matches.as_slice() {
        [entry] => Ok(entry.clone()),
        [] => {
            let tombstones = matching_tombstones(tombstones, key, Some(&parent.identity));
            if let [tombstone] = tombstones.as_slice() {
                return Err(dependency(tombstone));
            }
            if tombstones.len() > 1 {
                return Err(DirectPatchPreflightCause::AmbiguousTarget {
                    match_count: tombstones.len(),
                });
            }
            Err(DirectPatchPreflightCause::MissingTarget)
        }
        _ => Err(DirectPatchPreflightCause::AmbiguousTarget {
            match_count: matches.len(),
        }),
    }
}

pub(super) fn record_tombstones(
    index: &[VirtualNodeEntry],
    root_path: &[usize],
    patch_index: usize,
    kind: TombstoneKind,
    tombstones: &mut Vec<Tombstone>,
    aliases: &VirtualAliases,
) {
    tombstones.extend(
        index
            .iter()
            .filter(|entry| entry.path.starts_with(root_path))
            .map(|entry| Tombstone {
                identity: entry.identity.clone(),
                legacy_key: entry.legacy_key,
                raw_key: entry.raw_key,
                scoped_key: entry.identity.scoped_patch_address(entry.legacy_key),
                scoped_aliases: aliases.addresses_at(&entry.path),
                raw_aliases: aliases.batch_local_aliases_at(&entry.path),
                parent_key: entry.identity.parent().and_then(|parent| {
                    index
                        .iter()
                        .find(|candidate| &candidate.identity == parent)
                        .map(|parent| parent.identity.scoped_patch_address(parent.legacy_key))
                }),
                patch_index,
                kind,
            }),
    );
}

pub(super) fn record_error_origins(
    index: &[VirtualNodeEntry],
    root_path: &[usize],
    patch_index: usize,
    origins: &mut HashMap<SiblingIdentity, usize>,
) {
    for entry in index
        .iter()
        .filter(|entry| entry.path.starts_with(root_path))
    {
        origins.insert(
            entry
                .identity
                .scoped_patch_address(entry.legacy_key)
                .identity(),
            patch_index,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::core::VNode;
    use crate::layout::DirectPatchPreflightCause;

    use super::{LookupRole, VirtualAliases, resolve_virtual};

    #[test]
    fn stale_virtual_alias_path_is_an_identity_failure() {
        let tree = VNode::root().child(VNode::box_node().with_key("child"));
        let index = super::super::virtual_index(&tree).expect("valid fixture");
        let aliases = VirtualAliases::from_index(&index);
        let child = &index[1];
        let scoped_key = child.identity.scoped_patch_address(child.legacy_key);
        let Err(error) = resolve_virtual(&[], scoped_key, LookupRole::Target, &[], &aliases) else {
            panic!("an alias without an indexed node must be inconsistent");
        };

        assert!(matches!(
            error.source,
            DirectPatchPreflightCause::Identity(_)
        ));
    }
}
