use super::*;

fn keyed(value: &str) -> ScopedIdentitySegment {
    ScopedIdentitySegment::Keyed {
        key: CanonicalKey::Exact(Arc::from(value)),
        type_id: TypeId::of::<u8>(),
    }
}

fn positional(index: usize) -> ScopedIdentitySegment {
    ScopedIdentitySegment::Positional {
        type_id: TypeId::of::<u8>(),
        index,
    }
}

fn forced(
    parent: ScopedNodeIdentity,
    segment: ScopedIdentitySegment,
    cached_hash: u64,
    depth: usize,
) -> ScopedNodeIdentity {
    ScopedNodeIdentity::Child(Arc::new(ScopedIdentityNode {
        parent,
        segment,
        cached_hash,
        depth,
    }))
}

fn same_handle(left: &ScopedNodeIdentity, right: &ScopedNodeIdentity) -> bool {
    match (left, right) {
        (ScopedNodeIdentity::Root, ScopedNodeIdentity::Root) => true,
        (ScopedNodeIdentity::Child(left), ScopedNodeIdentity::Child(right)) => {
            Arc::ptr_eq(left, right)
        }
        _ => false,
    }
}

fn identity_hash(identity: &ScopedNodeIdentity) -> u64 {
    let mut hasher = DefaultHasher::new();
    identity.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn legacy_sibling_identity_contract_covers_all_domains() {
    let type_id = TypeId::of::<u8>();
    let at_front = NodeKey::with_key("a", type_id, 0);
    let at_back = NodeKey::with_key("a", type_id, 7);
    let other_key = NodeKey::with_key("b", type_id, 0);
    let other_type = NodeKey::with_key("a", TypeId::of::<u16>(), 0);
    let first = NodeKey::new(type_id, 0);
    let second = NodeKey::new(type_id, 1);

    assert_eq!(at_front.identity(), at_back.identity());
    assert!(at_front.matches(&at_back));
    assert_ne!(at_front.identity(), other_key.identity());
    assert!(!at_front.matches(&other_type));
    assert_ne!(first.identity(), second.identity());
    assert!(!first.matches(&second));
    assert!(!at_front.matches(&first));

    let keys = [at_front, at_back, other_key, first, second, NodeKey::root()];
    for left in &keys {
        for right in &keys {
            assert_eq!(
                left.matches(right),
                left.identity() == right.identity(),
                "{left:?} vs {right:?}"
            );
        }
    }
}

#[test]
fn canonical_diagnostics_and_resolution_matrix_are_exact() {
    assert_eq!(
        CanonicalKey::Exact(Arc::from("exact")).diagnostic_kind(),
        IdentityKeyKind::Exact
    );
    assert_eq!(
        CanonicalKey::Opaque(9).diagnostic_kind(),
        IdentityKeyKind::Opaque
    );

    let parent = ScopedNodeIdentity::Root;
    let mut arena = ScopedIdentityArena::default();
    let exact = VNode::text("x").with_props(Props::new().key("exact"));
    let exact_resolved = resolve_child_identity(
        &exact,
        3,
        &parent,
        &compatibility_token_for_exact,
        &mut arena,
    )
    .expect("props-only exact metadata is valid");
    assert!(matches!(
        exact_resolved.match_key(),
        SiblingMatchKey::Keyed(CanonicalKey::Exact(_))
    ));
    assert_eq!(exact_resolved.legacy_key.index, 3);
    assert_eq!(
        exact_resolved.compatibility_token(),
        Some(compatibility_token_for_exact("exact"))
    );

    let opaque = VNode::text("x").with_key("opaque");
    let opaque_resolved = resolve_child_identity(
        &opaque,
        4,
        &parent,
        &compatibility_token_for_exact,
        &mut arena,
    )
    .expect("opaque metadata is valid");
    assert!(matches!(
        opaque_resolved.canonical_key(),
        Some(CanonicalKey::Opaque(_))
    ));

    let positional = resolve_child_identity(
        &VNode::text("x"),
        5,
        &parent,
        &compatibility_token_for_exact,
        &mut arena,
    )
    .expect("missing metadata is positional");
    assert_eq!(positional.match_key(), SiblingMatchKey::Positional(5));
    assert_eq!(positional.compatibility_token(), None);

    let mismatched_token = VNode::text("x")
        .with_key("opaque")
        .with_props(Props::new().key("exact"));
    assert!(matches!(
        resolve_child_identity(
            &mismatched_token,
            0,
            &parent,
            &compatibility_token_for_exact,
            &mut arena,
        ),
        Err(ReconcilePlanError::KeyMetadataMismatch { index: 0, .. })
    ));

    let mut mismatched_type = VNode::text("x");
    mismatched_type.key.type_id = VNode::box_node().node_type.type_id();
    assert!(matches!(
        resolve_child_identity(
            &mismatched_type,
            1,
            &parent,
            &compatibility_token_for_exact,
            &mut arena,
        ),
        Err(ReconcilePlanError::KeyTypeMismatch { index: 1, .. })
    ));
}

#[test]
fn scoped_projection_is_keyed_idempotent_and_disjoint_from_raw_identity() {
    let raw = NodeKey::with_key("child", TypeId::of::<u8>(), 2);
    let mut arena = ScopedIdentityArena::default();
    let scoped = arena.child(&ScopedNodeIdentity::Root, keyed("child"));
    let projected = scoped.composite_identity(raw);
    let address = scoped.scoped_patch_address(raw);

    assert_ne!(projected, raw.identity());
    assert!(matches!(projected, SiblingIdentity::Keyed { .. }));
    assert_eq!(address.index, raw.index);
    assert!(ScopedNodeIdentity::is_scoped_patch_address(address));
    assert!(!ScopedNodeIdentity::is_scoped_patch_address(raw));
    assert!(!ScopedNodeIdentity::is_scoped_patch_address(NodeKey::new(
        TypeId::of::<u8>(),
        0,
    )));
    assert_eq!(scoped.parent(), Some(&ScopedNodeIdentity::Root));
    assert_eq!(ScopedNodeIdentity::Root.parent(), None);
    assert!(scoped.diagnostic().starts_with("scope:"));

    let mut projections = HashMap::new();
    insert_composite_projection(&mut projections, &scoped, raw)
        .expect("first insertion claims the projection");
    insert_composite_projection(&mut projections, &scoped, raw)
        .expect("reinserting the same exact scope is idempotent");
    assert_eq!(projections.len(), 1);
    assert_eq!(projections.get(&projected), Some(&scoped));
}

#[test]
fn composite_projection_collision_preserves_first_exact_scope() {
    let legacy = NodeKey::with_key("child", TypeId::of::<u8>(), 0);
    let left = forced(ScopedNodeIdentity::Root, positional(0), 7, 1);
    let right = forced(ScopedNodeIdentity::Root, positional(1), 7, 1);
    let mut projections = HashMap::new();

    insert_composite_projection(&mut projections, &left, legacy)
        .expect("first exact scope claims the projection");
    let (collision, first_scope) = insert_composite_projection(&mut projections, &right, legacy)
        .expect_err("a colliding hash must not alias another exact scope");

    assert_eq!(collision, left.composite_identity(legacy));
    assert_eq!(first_scope, left);
    assert_eq!(projections.len(), 1);
}

#[test]
fn exact_scopes_do_not_serialize_delimiters() {
    let root = ScopedNodeIdentity::Root;
    let mut arena = ScopedIdentityArena::default();
    let first = arena.child(&root, keyed("a/key:b"));
    let prefix = arena.child(&root, keyed("a"));
    let nested = arena.child(&prefix, keyed("b"));

    assert_ne!(first, nested);
}

#[test]
fn cached_hash_collision_and_depth_mismatch_use_iterative_exact_equality() {
    let left = forced(ScopedNodeIdentity::Root, positional(1), 7, 1);
    let other_segment = forced(ScopedNodeIdentity::Root, positional(2), 7, 1);
    let other_hash = forced(ScopedNodeIdentity::Root, positional(1), 8, 1);
    let other_depth = forced(ScopedNodeIdentity::Root, positional(1), 7, 2);
    let same = forced(ScopedNodeIdentity::Root, positional(1), 7, 1);

    assert_eq!(left, left.clone(), "Arc pointer equality is exact");
    assert_eq!(left, same, "independent equal paths remain equal");
    assert_ne!(left, other_segment);
    assert_ne!(left, other_hash);
    assert_ne!(left, other_depth);
    assert_ne!(left, ScopedNodeIdentity::Root);
    assert_ne!(ScopedNodeIdentity::Root, left);

    let mut identities = HashMap::new();
    identities.insert(left.clone(), "left");
    identities.insert(other_segment.clone(), "other");
    assert_eq!(identities.len(), 2);
    assert_eq!(identities[&left], "left");
    assert_eq!(identities[&other_segment], "other");
}

#[derive(Default)]
struct WriteCountingHasher {
    writes: usize,
}

impl Hasher for WriteCountingHasher {
    fn finish(&self) -> u64 {
        self.writes as u64
    }

    fn write(&mut self, _bytes: &[u8]) {
        self.writes += 1;
    }

    fn write_u64(&mut self, _value: u64) {
        self.writes += 1;
    }
}

#[test]
fn cached_scope_hash_has_depth_independent_hasher_writes() {
    let mut arena = ScopedIdentityArena::default();
    let mut scope = ScopedNodeIdentity::Root;
    for index in 0..512 {
        scope = arena.child(&scope, positional(index));
    }

    let mut root_hasher = WriteCountingHasher::default();
    ScopedNodeIdentity::Root.hash(&mut root_hasher);
    let mut deep_hasher = WriteCountingHasher::default();
    scope.hash(&mut deep_hasher);

    assert_eq!(root_hasher.finish(), 1);
    assert_eq!(deep_hasher.finish(), 1);
}

#[test]
fn independently_built_deep_scopes_keep_exact_equality_and_hash() {
    let mut left_arena = ScopedIdentityArena::default();
    let mut right_arena = ScopedIdentityArena::default();
    let (mut left, mut right) = (ScopedNodeIdentity::Root, ScopedNodeIdentity::Root);
    for index in 0..512 {
        left = left_arena.child(&left, positional(index));
        right = right_arena.child(&right, positional(index));
    }

    assert_eq!(left, right);
    assert_eq!(identity_hash(&left), identity_hash(&right));
    assert!(!same_handle(&left, &right));
    assert_ne!(left, right_arena.child(&right, positional(512)));
}

#[test]
fn seeded_deep_frame_reuses_handles_with_linear_intern_calls() {
    let mut initial = ScopedIdentityArena::default();
    let mut path = vec![ScopedNodeIdentity::Root];
    for index in 0..512 {
        let next = initial.child(&path[index], positional(index));
        path.push(next);
    }
    let mut frame = ScopedIdentityArena::seeded(path.iter());
    let mut rebuilt = ScopedNodeIdentity::Root;
    for index in 0..512 {
        rebuilt = frame.child(&rebuilt, positional(index));
    }

    assert!(same_handle(&path[512], &rebuilt));
    assert_eq!(frame.intern_calls, 512);
}

#[test]
fn dropping_a_partial_arena_leaves_no_shared_mutable_state() {
    let cancelled = ScopedIdentityArena::default().child(&ScopedNodeIdentity::Root, keyed("x"));
    let fresh = ScopedIdentityArena::default().child(&ScopedNodeIdentity::Root, keyed("x"));

    assert_eq!(cancelled, fresh);
    assert!(!same_handle(&cancelled, &fresh));
}

#[test]
fn semantic_equality_covers_false_and_typed_error_matrix() {
    let equal_left = VNode::box_node().child(VNode::text("x").with_key("stable"));
    let mut equal_right = equal_left.clone();
    equal_right.props.key = Some("root metadata is ignored".to_owned());
    assert!(
        semantically_equal_vnode_in(
            &equal_left,
            &equal_right,
            &mut ScopedIdentityArena::default()
        )
        .expect("valid equal trees compare")
    );

    let different_type = VNode::text("x");
    assert!(
        !semantically_equal_vnode_in(
            &VNode::box_node(),
            &different_type,
            &mut ScopedIdentityArena::default()
        )
        .expect("valid unequal root types compare")
    );

    let mut different_props = VNode::box_node();
    different_props.props.scroll_offset_x = Some(1);
    assert!(
        !semantically_equal_vnode_in(
            &VNode::box_node(),
            &different_props,
            &mut ScopedIdentityArena::default()
        )
        .expect("valid unequal props compare")
    );

    assert!(
        !semantically_equal_vnode_in(
            &VNode::box_node(),
            &VNode::box_node().child(VNode::text("extra")),
            &mut ScopedIdentityArena::default()
        )
        .expect("valid unequal child counts compare")
    );

    assert!(
        !semantically_equal_vnode_in(
            &VNode::box_node().child(VNode::text("x").with_key("left")),
            &VNode::box_node().child(VNode::text("x").with_key("right")),
            &mut ScopedIdentityArena::default()
        )
        .expect("different child identities compare false")
    );

    let nested_left = VNode::box_node().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("old")),
    );
    let nested_right = VNode::box_node().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("new")),
    );
    assert!(
        !semantically_equal_vnode_in(
            &nested_left,
            &nested_right,
            &mut ScopedIdentityArena::default()
        )
        .expect("matching branch with unequal descendant compares false")
    );

    let mut invalid_left = VNode::text("x");
    invalid_left.key.type_id = VNode::box_node().node_type.type_id();
    assert!(matches!(
        semantically_equal_vnode_in(
            &VNode::box_node().child(invalid_left),
            &VNode::box_node().child(VNode::text("x")),
            &mut ScopedIdentityArena::default(),
        ),
        Err(ReconcilePlanError::KeyTypeMismatch { .. })
    ));

    let mut invalid_right = VNode::text("x");
    invalid_right.key.type_id = VNode::box_node().node_type.type_id();
    assert!(matches!(
        semantically_equal_vnode_in(
            &VNode::box_node().child(VNode::text("x")),
            &VNode::box_node().child(invalid_right),
            &mut ScopedIdentityArena::default(),
        ),
        Err(ReconcilePlanError::KeyTypeMismatch { .. })
    ));

    let mut invalid_nested = VNode::text("leaf");
    invalid_nested.key.type_id = VNode::box_node().node_type.type_id();
    assert!(matches!(
        semantically_equal_vnode_in(
            &VNode::box_node().child(
                VNode::box_node()
                    .with_key("branch")
                    .child(VNode::text("leaf")),
            ),
            &VNode::box_node().child(VNode::box_node().with_key("branch").child(invalid_nested),),
            &mut ScopedIdentityArena::default(),
        ),
        Err(ReconcilePlanError::KeyTypeMismatch { .. })
    ));
}
