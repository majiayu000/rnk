# Task Plan：保留有符号坐标与失败元素身份

## Linked Issue

GH-132: https://github.com/majiayu000/rnk/issues/132

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Route at spec creation:
  `ready_to_spec -> write_spec` (`gh132-route-gate.json`, decision `allowed`)

当前 issue 的 `ready_to_implement` label 早于本 packet，不能作为实现授权。本 task plan是
spec-only交付的一部分；所有 implementation checkbox保持未完成，直到 human spec approval
和 `SP132-T1` fresh gate全部通过。

## 实现任务

- [ ] `SP132-T1` 执行 implementation authorization、dependency与owner preflight。 Owner: `gh132-implementation-coordinator` | Done when: human approval、fresh implement gate、PR #137 merged refresh、#131 frozen manifest与clean exact base全部通过 | Verify: 运行本任务下列route/PR/git命令并人工核对approval和owner evidence。
  - Owner: `gh132-implementation-coordinator`
  - Dependencies: GH-132 spec PR 已 merged并有 human approval；不得把本 spec PR的创建、
    agent review或当前 premature label当作批准。
  - Done when:
    1. fresh SpecRail implement route对 issue #132、merged product/tech返回 `allowed`；
    2. issue只有唯一 canonical `ready_to_implement`且无冲突 readiness label；
    3. PR #137 已最终 merged、非 draft，其 merge commit是fresh expected main祖先，fresh
       file list与两条zero-width focused tests均以`matched=1`重跑；
    4. #131已有 frozen planned-change manifest；若与GH132相交，coordinator记录serial
       ownership，确认没有并发writer；
    5. 在首次 implementation edit 前，worktree porcelain精确为空，`HEAD`精确等于fresh
       `origin/main` SHA，并记录该 SHA 为`GH132_IMPLEMENTATION_BASE_SHA`；implementation
       11路径manifest逐路径冻结。
  - Verify:
    ```sh
    set -euo pipefail
    : "${SPEC_RAIL_ROOT:?set SPEC_RAIL_ROOT to the checked-out SpecRail workflow-pack root}"
    test -f "$SPEC_RAIL_ROOT/checks/route_gate.py"
    test -z "$(git status --porcelain=v1 --untracked-files=all)"
    git fetch --no-tags origin main
    EXPECTED_MAIN_SHA="$(git rev-parse 'FETCH_HEAD^{commit}')"
    CURRENT_HEAD_SHA="$(git rev-parse 'HEAD^{commit}')"
    test -n "$EXPECTED_MAIN_SHA"
    test "$CURRENT_HEAD_SHA" = "$EXPECTED_MAIN_SHA"
    printf 'GH132_IMPLEMENTATION_BASE_SHA=%s\n' "$EXPECTED_MAIN_SHA"
    python3 "$SPEC_RAIL_ROOT/checks/route_gate.py" \
      --repo "$SPEC_RAIL_ROOT" \
      --route implement --issue 132 --state ready_to_implement \
      --artifact product_spec="$PWD/specs/GH132/product.md" \
      --artifact tech_spec="$PWD/specs/GH132/tech.md" --json
    PR137_JSON="$(gh pr view 137 --repo majiayu000/rnk \
      --json state,isDraft,headRefOid,mergeCommit,files)"
    test "$(printf '%s\n' "$PR137_JSON" | jq -r '.state')" = "MERGED"
    test "$(printf '%s\n' "$PR137_JSON" | jq -r '.isDraft')" = "false"
    PR137_MERGE_SHA="$(printf '%s\n' "$PR137_JSON" | jq -r '.mergeCommit.oid')"
    test -n "$PR137_MERGE_SHA"
    git merge-base --is-ancestor "$PR137_MERGE_SHA" "$EXPECTED_MAIN_SHA"
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection -- --exact
    ```
    `SPEC_RAIL_ROOT` 必须由执行环境显式设置为 SpecRail workflow pack checkout 根目录；
    未设置、路径错误或 gate 文件缺失时上述命令立即失败，不得跳过或降级为 warning。两条
    filtered test还必须逐条记录`matched=1`、`passed=1`、`ignored=0`，不能只看exit 0。
    上述整段必须在任何 implementation source/test edit 前运行并保存输出。
    人工核对 GH132 spec approval、#131 frozen manifest 与无共享writer。
  - Covers: B-020, B-021

- [ ] `SP132-T2` 实现element-scoped floor conversion、signed clip和staged projection原子性。 Owner: `gh132-coordinate-core` | Done when: scoped floor/clip/owner/single-commit合同及拆分测试全部完成 | Verify: 运行本任务下列11个exact Rust tests。
  - Owner: `gh132-coordinate-core`
  - Dependencies: `SP132-T1`全部通过；PR #137 merge后的
    `projection/staged.rs`/tests合同已重新定位。
  - File ownership:
    `src/renderer/tree_renderer.rs`、
    `src/renderer/tree_renderer/projection.rs`、
    `src/renderer/tree_renderer/projection/staged.rs`、
    `src/renderer/tree_renderer/tests.rs`、
    `src/renderer/tree_renderer/tests/coordinates.rs`、
    `src/renderer/tree_renderer/projection/tests.rs`、
    `src/renderer/tree_renderer/projection/tests/coordinates.rs`。
  - Done when:
    1. 单一 scoped wider accumulator先逐个验证`f32` operand有限，再按顺序组合；每步超出
       `f32`范围或最终floor超出`[-2^63,2^63)`均为Overflow，`-0.0`与正向行为兼容；
    2. x/y/root offset/layout/ancestor/padding/scroll/text/background/border/clip都使用同一
       scoped转换或checked constructor；
    3. clip edges在viewport/active-clip交集前保持signed half-open；
    4. coordinate variants在产生点携带exact current element ID，root fallback不覆盖；
    5. caller Output与projection只在全部校验成功后single commit；
    6. 两个既有tests文件只增加子模块声明，新增fixture放到各自`coordinates.rs`，所有
       production/test文件低于800行，不压缩旧测试、不用`#[rustfmt::skip]`。
  - Verify:
    ```sh
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinates_use_one_floor_conversion -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::negative_zero_positive_fractional_and_integral_coordinates_are_compatible -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinate_composition_is_checked_and_axis_independent -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::finite_operands_that_overflow_f32_composition_and_i64_bounds_classify_overflow -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nan_and_infinities_classify_as_non_finite_for_each_coordinate_source -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nested_coordinate_failures_report_exact_current_child -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::coordinate_owner_survives_conversion_and_only_unscoped_failures_use_root_fallback -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::late_nested_coordinate_failure_discards_earlier_staged_paint -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_x_and_y_clip_instead_of_painting_at_zero -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_scroll_ancestor_and_clip_preserve_signed_disposition -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::coordinate_failure_commits_neither_output_nor_projection -- --exact
    ```
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-014, B-016, B-020

- [ ] `SP132-T3` 锁定public typed error、safe context以及string/TestRenderer caller。 Owner: `gh132-public-error-callers` | Done when: public error shape兼容且nested child ID/source/safe-context/independent caller fixtures完成 | Verify: 运行本任务下列7个exact Rust tests。
  - Owner: `gh132-public-error-callers`
  - Dependencies: `SP132-T2` core error owner与transaction boundary稳定。
  - File ownership:
    `src/renderer/error.rs`、
    `tests/text_flow_renderer_error_paths.rs`。
  - Done when:
    1. 不新增或改名public error variant/字段，`TextRenderError::Coordinate`保留exact child
       ID与`TextCoordinateError` source；
    2. Display/source chain只含ID和fixed classification，不含source text/frame/cache；
    3. nested child NaN和finite overflow都分别通过`try_render_to_string*`、
       `TestRenderer::try_render_to_ansi/plain` exact fixture；
    4. string失败没有partial返回，compat wrappers只在零外部提交后fail loudly；
    5. 独立/交错caller不共享owner或staged状态。
  - Verify:
    ```sh
    cargo test --workspace --lib --locked renderer::error::tests::coordinate_error_context_is_typed_and_does_not_leak_content -- --exact
    cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_string_api_with_exact_id -- --exact
    cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_test_renderer_with_exact_id -- --exact
    cargo test --test text_flow_renderer_error_paths --locked independent_coordinate_failures_do_not_share_owner_or_frame_state -- --exact
    cargo test --test text_flow_renderer_error_paths --locked try_render_to_string_preserves_source_and_returns_no_partial_string -- --exact
    cargo test --test text_flow_error_paths --locked typed_error_reaches_remaining_callers -- --exact
    cargo test --test text_flow_error_paths --locked caller_failure_commits_no_partial_output -- --exact
    ```
  - Covers: B-008, B-009, B-010, B-011, B-013, B-014, B-016, B-019, B-020

- [ ] `SP132-T4` 验证dynamic App的ID传播、candidate atomicity、重试与中断边界。 Owner: `gh132-dynamic-transaction` | Done when: dynamic/App exact ID、candidate atomicity和clean retry合同全部完成 | Verify: 运行本任务下列6个exact Rust tests。
  - Owner: `gh132-dynamic-transaction`
  - Dependencies: `SP132-T2`、`SP132-T3`。
  - File ownership:
    `src/renderer/pipeline.rs`、
    `src/renderer/app.rs`。
  - Done when:
    1. nested child NaN/overflow经dynamic pipeline保持exact child ID；
    2. failure不提交frame string、previous VNode、runtime layout/key aliases、static lines或
       candidate flow/layout cache；
    3. LayoutEngine失败后处于明确clean retry状态，重复failure稳定，corrected retry无重复
       node/cell/alias；
    4. dropped/injected candidate等价于cancellation-before-commit，later successful frame
       不观察失败candidate；
    5. `App::run`既有I/O mapping保留同一`TextRenderError`和nested typed source chain。
  - Verify:
    ```sh
    cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::nested_child_coordinate_errors_keep_id_and_candidate_state -- --exact
    cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::repeated_coordinate_failure_then_correction_retries_cleanly -- --exact
    cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::dropped_or_failed_coordinate_candidate_is_never_published -- --exact
    cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::incremental_failure_retries_from_clean_layout_tree -- --exact
    cargo test --workspace --lib --locked renderer::app::tests::nested_child_coordinate_error_reaches_app_io_source_chain -- --exact
    cargo test --workspace --lib --locked renderer::app::tests::app_render_candidate_preserves_typed_error_source -- --exact
    ```
  - Covers: B-008, B-012, B-013, B-015, B-016, B-017, B-018, B-019, B-020

- [ ] `SP132-T5` 完成compatibility、critical ledger、coverage与exact-head closure evidence。 Owner: `gh132-verification-review` | Done when: B-set、manifest、tests、coverage、CI、reviewThreads与PR gate绑定同一exact head | Verify: 运行Tech全部mapping/ledger和本任务full Rust/closure命令。
  - Owner: `gh132-verification-review`
  - Dependencies: `SP132-T1`至`SP132-T4`全部绿色；实现者与independent reviewer分离。
  - File ownership: none；只读verification lane，不修改实现、测试、label、thread或PR状态。
  - Done when:
    1. Product B-ID集合、Tech mapping集合与Tasks Covers union均精确为B-001至B-021；
    2. Tech Spec Product-to-Test Mapping和Critical Test Ledger每个filtered test均证明
       `matched=1`、`passed=1`、`ignored=0`；
    3. PR #137 merged-head zero-width regressions、全部GH132 focused tests和full Rust gates
       通过；
    4. implementation PR changed files精确等于独立的11路径
       `specrail-planned-changes` manifest；三份已合入packet不在该diff内，也不得被重改；
    5. exact implementation head的新代码line coverage>=80%，signed conversion/owner/
       transaction critical branches=100%；
    6. fresh CI、independent review、zero unresolved current actionable review threads和
       SpecRail PR gate都绑定同一head SHA。
  - Verify:
    ```sh
    set -euo pipefail
    : "${GH132_IMPLEMENTATION_PR:?set the GH132 implementation PR number}"
    : "${GH132_IMPLEMENTATION_BASE_SHA:?set the exact starting main SHA recorded by SP132-T1}"
    test -z "$(git status --porcelain=v1 --untracked-files=all)"
    git fetch --no-tags origin main
    EXPECTED_CURRENT_MAIN_SHA="$(git rev-parse 'FETCH_HEAD^{commit}')"
    PR_JSON="$(gh pr view "$GH132_IMPLEMENTATION_PR" --repo majiayu000/rnk \
      --json baseRefOid,headRefOid)"
    PR_BASE_SHA="$(printf '%s\n' "$PR_JSON" | jq -r '.baseRefOid')"
    PR_HEAD_SHA="$(printf '%s\n' "$PR_JSON" | jq -r '.headRefOid')"
    test "$PR_BASE_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
    test "$(git rev-parse 'HEAD^{commit}')" = "$PR_HEAD_SHA"
    test "$(git merge-base "$PR_HEAD_SHA" "$EXPECTED_CURRENT_MAIN_SHA")" = \
      "$EXPECTED_CURRENT_MAIN_SHA"
    git merge-base --is-ancestor "$GH132_IMPLEMENTATION_BASE_SHA" \
      "$EXPECTED_CURRENT_MAIN_SHA"
    IMPLEMENTATION_MANIFEST_JSON="$(sed -n \
      '/<!-- specrail-planned-changes/{n;p;}' specs/GH132/tech.md)"
    test "$(printf '%s\n' "$IMPLEMENTATION_MANIFEST_JSON" | jq -r '.paths | length')" -eq 11
    EXPECTED_CHANGED_PATHS="$(printf '%s\n' "$IMPLEMENTATION_MANIFEST_JSON" |
      jq -r '.paths[]' | LC_ALL=C sort)"
    ACTUAL_CHANGED_PATHS="$(git diff --name-only \
      "$EXPECTED_CURRENT_MAIN_SHA...$PR_HEAD_SHA" | LC_ALL=C sort)"
    test "$ACTUAL_CHANGED_PATHS" = "$EXPECTED_CHANGED_PATHS"
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_signed_coordinates_axis_clips_and_nested_active_clips_are_exact -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_failure_commits_neither_cells_nor_projection -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection -- --exact
    cargo test --workspace --lib --locked renderer::tree_renderer::tests::scrolled_out_negative_rows_do_not_paint_at_top -- --exact
    cargo test --test prelude_surfaces --locked try_render_to_string_surface -- --exact
    cargo fmt --all -- --check
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
    cargo test --workspace --all-targets --all-features --locked
    ```
    `PR_BASE_SHA` 必须与fresh current main完全相等，且该 SHA 必须是PR head的exact
    merge-base；仅证明PR137或旧main是ancestor不够。另逐条运行Tech mapping和ledger全部
    命令并证明`matched=1`，再fresh查询PR exact head、CI、reviewThreads、coverage与
    SpecRail PR gate。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019,
    B-020, B-021

## 并行拆分

生产任务按以下顺序执行，禁止并发写共享调用链：

```text
SP132-T1 -> SP132-T2 -> SP132-T3 -> SP132-T4 -> SP132-T5
```

- T2独占tree/projection/staged和两个coordinate test子模块。
- T3独占public error与public-facing integration fixture。
- T4独占dynamic pipeline/App source-module tests。
- T5只读。即使T3/T4文件集合不相交，T4也必须等待T3 typed contract稳定，避免对同一错误
  flow并发做不兼容假设。
- PR #137合入前不得开始T2；#131 frozen manifest与任一T2-T4路径相交时，整条GH132
  implementation lane与#131串行，不做共享writer。

## 验证

- Product invariant set:
  `B-001..B-021`
- Tech mapping set:
  `B-001..B-021`
- Task Covers union:
  `B-001..B-021`
- `specrail-spec-packet-changes`精确包含三份packet；本 spec PR只按该三路径验证。
- `specrail-planned-changes`精确包含九个既有Rust/test文件和两个因800行上限新增的
  `coordinates.rs` test子模块，共11路径；future implementation PR只按该manifest与fresh
  current main做changed-path diff，三份已合入spec不得出现。
- implementation命令是后续human-gated handoff，不在本 spec PR中实现或伪造通过。

## Handoff Notes

- 本spec PR title固定为
  `spec: define signed coordinate and element error preservation`，body使用`Refs #132`，
  绝不使用Fixes/Closes。
- 当前route artifact只授权`write_spec`。不得修改issue label来掩盖premature状态；human
  spec approval后再fresh运行implement gate。
- PR #137的head SHA只记录起草时证据，不是implementation pin；真正依赖是其最终merge
  commit和refreshed owner contract。
- #131当前没有权威manifest，因此不能宣称“无冲突”。开始实现前必须冻结并比较manifest，
  traversal/projection/caller tests有交集即串行。
- 若实现需要manifest外路径、public API变化、第二个coordinate helper、全局/thread-local
  current element或放宽atomic tests，停止并重新走spec review。
