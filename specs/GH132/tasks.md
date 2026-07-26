# Task Plan：保留有符号坐标与失败元素身份

## Linked Issue

GH-132: https://github.com/majiayu000/rnk/issues/132

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Route at spec creation:
  `ready_to_spec -> write_spec` (`gh132-route-gate.json`, decision `allowed`)

2026-07-27 round-5 fresh 查询得到 issue 唯一 readiness 为 `ready_to_spec`；该快照不能
作为未来 route 的硬编码输入。本 task plan是spec-only交付的一部分；所有 implementation
checkbox保持未完成，直到 human spec approval 和 `SP132-T1` 对届时 live readiness 的
fresh gate全部通过。

## 实现任务

- [ ] `SP132-T1` 执行 implementation authorization、dependency与owner preflight。 Owner: `gh132-implementation-coordinator` | Done when: human approval、fresh implement gate、PR #137/#142 merged changed-file refresh与clean exact base全部通过 | Verify: 运行本任务下列route/PR/git命令并人工核对approval和owner evidence。
  - Owner: `gh132-implementation-coordinator`
  - Dependencies: GH-132 spec PR 已 merged并有 human approval；不得把本 spec PR的创建、
    agent review或当前 premature label当作批准。
  - Done when:
    1. fresh GitHub query证明 issue #132 只有一个由 pinned SpecRail `labels.yaml` 声明的
       canonical readiness，并把该逐字值传给 implement route，不得硬编码；
    2. 上述 live readiness 必须是 `ready_to_implement`，且fresh SpecRail implement route
       对 merged product/tech返回 `allowed`；`ready_to_spec`或任何其他值都fail closed；
    3. PR #137 已最终 merged、非 draft，其 merge commit是fresh expected main祖先，fresh
       6路径file set、newline SHA-256和与GH132 manifest精确两路径受控交集均匹配；两条
       zero-width focused tests各以`matched=1`重跑；
    4. PR #142 已最终 merged、非 draft，其 merge commit是fresh expected main祖先，fresh
       6路径file set、newline SHA-256和与GH132 manifest精确三路径受控交集均匹配；在交集
       路径完成 source-drift refresh，确认GH132不重写 VirtualText/span-only contract；
    5. 在首次 implementation edit 前，worktree porcelain精确为空，`HEAD`精确等于fresh
       `origin/main` SHA，并记录该 SHA 为`GH132_IMPLEMENTATION_BASE_SHA`；implementation
       12路径manifest逐路径冻结。
  - Verify:
    ```sh
    set -euo pipefail
    run_exact() {
      GH132_LIST="$("$@" -- --exact --list --format terse 2>&1)"
      GH132_MATCHED="$(printf '%s\n' "$GH132_LIST" |
        awk -F ': ' '$2 == "test" { count++ } END { print count + 0 }')"
      test "$GH132_MATCHED" -eq 1
      GH132_RESULT="$("$@" -- --exact 2>&1)" || {
        printf '%s\n' "$GH132_RESULT" >&2
        return 1
      }
      printf '%s\n' "$GH132_RESULT"
      GH132_COUNTS="$(printf '%s\n' "$GH132_RESULT" | awk '
        /^test result:/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "passed;") passed += $(i - 1)
            if ($i == "failed;") failed += $(i - 1)
            if ($i == "ignored;") ignored += $(i - 1)
          }
        }
        END { printf "%d %d %d\n", passed, failed, ignored }')"
      test "$GH132_COUNTS" = "1 0 0"
    }
    : "${SPEC_RAIL_ROOT:?set SPEC_RAIL_ROOT to the checked-out SpecRail workflow-pack root}"
    SPEC_RAIL_REV=bfc60f26164af5df1ebd3b5cb79d07379fc416b7
    test "$(git -C "$SPEC_RAIL_ROOT" rev-parse 'HEAD^{commit}')" = "$SPEC_RAIL_REV"
    test "$(git -C "$SPEC_RAIL_ROOT" remote get-url origin)" = \
      https://github.com/majiayu000/specrail.git
    test "$(python3 -c \
      'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
      "$SPEC_RAIL_ROOT/checks/route_gate.py")" = \
      d77cad0763713ca589be1c4278edcec7c90c017bc383fd6a7976402be22a7433
    test "$(python3 -c \
      'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
      "$SPEC_RAIL_ROOT/checks/check_workflow.py")" = \
      c5bd73060037b0e8febace0e5ee8473e17973e1ca17257ea1517a94e05fa7549
    test "$(python3 -c \
      'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
      "$SPEC_RAIL_ROOT/checks/github_duplicate_evidence.py")" = \
      eab228a33d84a43cde1ba3587d5edde50993ae11c5c5a522ee8d01b64b284d55
    ISSUE_JSON="$(gh issue view 132 --repo majiayu000/rnk \
      --json number,state,labels)"
    test "$(printf '%s\n' "$ISSUE_JSON" | jq -r '.number')" = "132"
    test "$(printf '%s\n' "$ISSUE_JSON" | jq -r '.state')" = "OPEN"
    CANONICAL_READINESS_LABELS="$(awk '
      /^  readiness:/ { inside = 1; next }
      inside && /^    - / { sub(/^    - /, ""); print; next }
      inside { exit }' "$SPEC_RAIL_ROOT/labels.yaml" | LC_ALL=C sort)"
    ISSUE_LABELS="$(printf '%s\n' "$ISSUE_JSON" |
      jq -r '.labels[].name' | LC_ALL=C sort)"
    LIVE_READINESS_LABELS="$(comm -12 \
      <(printf '%s\n' "$CANONICAL_READINESS_LABELS") \
      <(printf '%s\n' "$ISSUE_LABELS"))"
    test "$(printf '%s\n' "$LIVE_READINESS_LABELS" |
      awk 'NF { count++ } END { print count + 0 }')" -eq 1
    LIVE_ROUTE_STATE="$LIVE_READINESS_LABELS"
    WORKTREE_STATUS="$(git status --porcelain=v1 --untracked-files=all)"
    test -z "$WORKTREE_STATUS"
    git fetch --no-tags origin main
    EXPECTED_MAIN_SHA="$(git rev-parse 'FETCH_HEAD^{commit}')"
    CURRENT_HEAD_SHA="$(git rev-parse 'HEAD^{commit}')"
    test -n "$EXPECTED_MAIN_SHA"
    test "$CURRENT_HEAD_SHA" = "$EXPECTED_MAIN_SHA"
    printf 'GH132_IMPLEMENTATION_BASE_SHA=%s\n' "$EXPECTED_MAIN_SHA"
    SPEC_RAIL_MIRROR="$(mktemp -d "${TMPDIR:-/tmp}/gh132-specrail.XXXXXX")"
    git -C "$SPEC_RAIL_ROOT" archive "$SPEC_RAIL_REV" | tar -x -C "$SPEC_RAIL_MIRROR"
    mkdir -p "$SPEC_RAIL_MIRROR/specs/GH132" "$SPEC_RAIL_MIRROR/evidence"
    cp specs/GH132/product.md specs/GH132/tech.md specs/GH132/tasks.md \
      "$SPEC_RAIL_MIRROR/specs/GH132/"
    python3 "$SPEC_RAIL_MIRROR/checks/check_workflow.py" \
      --repo "$SPEC_RAIL_MIRROR" --spec-dir specs/GH132
    python3 "$SPEC_RAIL_MIRROR/checks/github_duplicate_evidence.py" \
      --github-repo majiayu000/rnk --issue 132 --remote origin --json \
      > "$SPEC_RAIL_MIRROR/evidence/gh132-duplicate.json"
    test "$LIVE_ROUTE_STATE" = "ready_to_implement"
    python3 "$SPEC_RAIL_MIRROR/checks/route_gate.py" \
      --repo "$SPEC_RAIL_MIRROR" \
      --route implement --issue 132 --state "$LIVE_ROUTE_STATE" \
      --artifact product_spec=specs/GH132/product.md \
      --artifact tech_spec=specs/GH132/tech.md \
      --duplicate-evidence "$SPEC_RAIL_MIRROR/evidence/gh132-duplicate.json" \
      --mode required --json > "$SPEC_RAIL_MIRROR/evidence/gh132-route.json"
    test "$(jq -r '.decision' \
      "$SPEC_RAIL_MIRROR/evidence/gh132-route.json")" = "allowed"
    PR137_JSON="$(gh pr view 137 --repo majiayu000/rnk \
      --json state,isDraft,headRefOid,mergeCommit,files)"
    test "$(printf '%s\n' "$PR137_JSON" | jq -r '.state')" = "MERGED"
    test "$(printf '%s\n' "$PR137_JSON" | jq -r '.isDraft')" = "false"
    PR137_MERGE_SHA="$(printf '%s\n' "$PR137_JSON" | jq -r '.mergeCommit.oid')"
    test -n "$PR137_MERGE_SHA"
    git merge-base --is-ancestor "$PR137_MERGE_SHA" "$EXPECTED_MAIN_SHA"
    PR137_EXPECTED_FILES="$(printf '%s\n' \
      src/renderer/output.rs \
      src/renderer/output/tests.rs \
      src/renderer/output/zero_width.rs \
      src/renderer/tree_renderer/projection/staged.rs \
      src/renderer/tree_renderer/projection/tests.rs \
      src/renderer/tree_renderer/projection/tests/zero_width.rs)"
    PR137_ACTUAL_FILES="$(printf '%s\n' "$PR137_JSON" |
      jq -r '.files[].path' | LC_ALL=C sort)"
    test "$PR137_ACTUAL_FILES" = "$PR137_EXPECTED_FILES"
    PR137_FILES_SHA256="$(printf '%s\n' "$PR137_ACTUAL_FILES" | python3 -c \
      'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')"
    test "$PR137_FILES_SHA256" = \
      ee2af110e7751fc058e8b87dde9b15666e161808317cc8b4481cd93f0dcb06be
    IMPLEMENTATION_MANIFEST_JSON="$(sed -n \
      '/<!-- specrail-planned-changes/{n;p;}' specs/GH132/tech.md)"
    GH132_IMPLEMENTATION_FILES="$(printf '%s\n' "$IMPLEMENTATION_MANIFEST_JSON" |
      jq -r '.paths[]' | LC_ALL=C sort)"
    test "$(printf '%s\n' "$GH132_IMPLEMENTATION_FILES" | wc -l |
      tr -d ' ')" -eq 12
    PR137_EXPECTED_OVERLAP="$(printf '%s\n' \
      src/renderer/tree_renderer/projection/staged.rs \
      src/renderer/tree_renderer/projection/tests.rs)"
    PR137_ACTUAL_OVERLAP="$(comm -12 \
      <(printf '%s\n' "$PR137_ACTUAL_FILES") \
      <(printf '%s\n' "$GH132_IMPLEMENTATION_FILES"))"
    test "$PR137_ACTUAL_OVERLAP" = "$PR137_EXPECTED_OVERLAP"
    GH132_NON_DEPENDENCY_FILES="$(comm -23 \
      <(printf '%s\n' "$GH132_IMPLEMENTATION_FILES") \
      <(printf '%s\n' "$PR137_ACTUAL_FILES"))"
    test "$(printf '%s\n' "$GH132_NON_DEPENDENCY_FILES" | wc -l |
      tr -d ' ')" -eq 10
    test -z "$(comm -12 \
      <(printf '%s\n' "$GH132_NON_DEPENDENCY_FILES") \
      <(printf '%s\n' "$PR137_ACTUAL_FILES"))"
    printf 'PR137_FILES_SHA256=%s\nPR137_OVERLAP=%s\n' \
      "$PR137_FILES_SHA256" "$PR137_ACTUAL_OVERLAP"
    PR142_JSON="$(gh pr view 142 --repo majiayu000/rnk \
      --json state,isDraft,headRefOid,mergeCommit,files)"
    test "$(printf '%s\n' "$PR142_JSON" | jq -r '.state')" = "MERGED"
    test "$(printf '%s\n' "$PR142_JSON" | jq -r '.isDraft')" = "false"
    PR142_MERGE_SHA="$(printf '%s\n' "$PR142_JSON" | jq -r '.mergeCommit.oid')"
    test -n "$PR142_MERGE_SHA"
    git merge-base --is-ancestor "$PR142_MERGE_SHA" "$EXPECTED_MAIN_SHA"
    PR142_EXPECTED_FILES="$(printf '%s\n' \
      src/layout/engine/text_flow_bridge.rs \
      src/renderer/render_to_string.rs \
      src/renderer/tree_renderer.rs \
      src/renderer/tree_renderer/projection.rs \
      src/renderer/tree_renderer/tests.rs \
      tests/text_source_compat.rs)"
    PR142_ACTUAL_FILES="$(printf '%s\n' "$PR142_JSON" |
      jq -r '.files[].path' | LC_ALL=C sort)"
    test "$PR142_ACTUAL_FILES" = "$PR142_EXPECTED_FILES"
    PR142_FILES_SHA256="$(printf '%s\n' "$PR142_ACTUAL_FILES" | python3 -c \
      'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')"
    test "$PR142_FILES_SHA256" = \
      6db38f157f5fe455302e2c37d55f503b2a74f61795e522d8ab507132befdc3a9
    PR142_EXPECTED_OVERLAP="$(printf '%s\n' \
      src/renderer/tree_renderer.rs \
      src/renderer/tree_renderer/projection.rs \
      src/renderer/tree_renderer/tests.rs)"
    PR142_ACTUAL_OVERLAP="$(comm -12 \
      <(printf '%s\n' "$PR142_ACTUAL_FILES") \
      <(printf '%s\n' "$GH132_IMPLEMENTATION_FILES"))"
    test "$PR142_ACTUAL_OVERLAP" = "$PR142_EXPECTED_OVERLAP"
    GH132_NON_PR142_FILES="$(comm -23 \
      <(printf '%s\n' "$GH132_IMPLEMENTATION_FILES") \
      <(printf '%s\n' "$PR142_ACTUAL_FILES"))"
    test "$(printf '%s\n' "$GH132_NON_PR142_FILES" | wc -l |
      tr -d ' ')" -eq 9
    test -z "$(comm -12 \
      <(printf '%s\n' "$GH132_NON_PR142_FILES") \
      <(printf '%s\n' "$PR142_ACTUAL_FILES"))"
    printf 'PR142_FILES_SHA256=%s\nPR142_OVERLAP=%s\n' \
      "$PR142_FILES_SHA256" "$PR142_ACTUAL_OVERLAP"
    run_exact cargo test --workspace --lib --locked \
      renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence
    run_exact cargo test --workspace --lib --locked \
      renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection
    ```
    `SPEC_RAIL_ROOT` 必须由执行环境显式设置为 SpecRail workflow pack checkout 根目录；
    checkout必须来自`https://github.com/majiayu000/specrail.git`的上述exact detached
    revision，未设置、路径/revision/hash错误时命令立即失败，不得跳过或降级为 warning。
    `SPEC_RAIL_MIRROR`必须保留到交付证据归档完成；route JSON还必须证明artifact路径均为
    mirror内的`specs/GH132/{product,tech,tasks}.md`，duplicate evidence为fresh collector
    输出，且没有open PR或remote branch占用GH132实现token。PR #137 sorted file set、
    newline digest与两路径受控交集必须逐字匹配；PR #142 sorted file set、newline digest
    与三路径受控交集也必须逐字匹配。两个比较器都要用one-missing、one-unexpected和
    one-additional-overlap fixture证明均fail closed，任何变化都停止并重新冻结spec。两条
    filtered test由`run_exact`逐条机械记录`matched=1`、`passed=1`、`ignored=0`，不能只看
    exit 0。
    上述整段必须在任何 implementation source/test edit 前运行并保存输出。
    人工核对 GH132 spec approval、PR #137/#142 source-drift refresh 与无共享writer。
  - Covers: B-020, B-021

- [ ] `SP132-T2` 实现element-scoped floor conversion、signed clip和staged projection原子性。 Owner: `gh132-coordinate-core` | Done when: scoped floor/clip/owner/single-commit合同及拆分测试全部完成 | Verify: 运行本任务下列12个exact Rust tests。
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
    1. 单一scoped boundary checker先逐个验证`f32` operand有限；每个现有f32语义边界用
       wider shadow按原顺序检测范围，但下一边界继续使用该边界原f32舍入结果；shadow超出
       f32范围或最终floor超出`[-2^63,2^63)`均为Overflow，`-0.0`与正向行为兼容；
       exact fixture至少覆盖`f32::MAX + f32::MAX`、负向MAX组合、
       `-33_554_432 + 1 + 33_554_432 == 0`、nested ancestor累积、scroll subtraction与
       own/ancestor clip edge组合，逐轴区分Overflow/NonFinite；
    2. x/y/root offset/layout/ancestor/scroll/text/background/border/clip都使用同一scoped
       boundary checker或checked constructor；padding保持独立signed conversion/floor，
       再与screen origin checked integer add，exact fixture锁定`0.5 + 0.5`仍为列/行0；
    3. clip edges在viewport/active-clip交集前保持signed half-open；
    4. coordinate variants在产生点携带exact current element ID；
       `validate_tree_flows -> validate_flow(element.id, ...) ->
       validate_row_footprints(element.id, ...)`逐层保留child owner；coordinate overflow、
       malformed flow和带`ProjectionId`的forward/reverse duplicate均消费已有owner；
       reverse duplicate在publish使用当前`ProjectionId`、round-trip validation使用
       `record.id`；exact fixture分别注入两处duplicate并让root/current ID不同，断言public
       error精确归属current owner。root fallback只用于错误数据中真正没有ID的finish failure；
    5. caller Output与projection只在全部校验成功后single commit；
    6. 两个既有tests文件只增加子模块声明，新增fixture放到各自`coordinates.rs`，所有
       production/test文件低于800行，不压缩旧测试、不用`#[rustfmt::skip]`。
  - Verify:
    ```sh
    set -euo pipefail
    run_exact() {
      GH132_LIST="$("$@" -- --exact --list --format terse 2>&1)"
      GH132_MATCHED="$(printf '%s\n' "$GH132_LIST" |
        awk -F ': ' '$2 == "test" { count++ } END { print count + 0 }')"
      test "$GH132_MATCHED" -eq 1
      GH132_RESULT="$("$@" -- --exact 2>&1)" || {
        printf '%s\n' "$GH132_RESULT" >&2
        return 1
      }
      printf '%s\n' "$GH132_RESULT"
      GH132_COUNTS="$(printf '%s\n' "$GH132_RESULT" | awk '
        /^test result:/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "passed;") passed += $(i - 1)
            if ($i == "failed;") failed += $(i - 1)
            if ($i == "ignored;") ignored += $(i - 1)
          }
        }
        END { printf "%d %d %d\n", passed, failed, ignored }')"
      test "$GH132_COUNTS" = "1 0 0"
    }
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinates_use_one_floor_conversion
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::negative_zero_positive_fractional_and_integral_coordinates_are_compatible
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinate_composition_is_checked_and_axis_independent
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::finite_operands_that_overflow_f32_composition_and_i64_bounds_classify_overflow
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nan_and_infinities_classify_as_non_finite_for_each_coordinate_source
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nested_coordinate_failures_report_exact_current_child
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::coordinate_owner_survives_conversion_and_only_unscoped_failures_use_root_fallback
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::nested_flow_validation_overflow_reaches_public_error_with_exact_child
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::late_nested_coordinate_failure_discards_earlier_staged_paint
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_x_and_y_clip_instead_of_painting_at_zero
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_scroll_ancestor_and_clip_preserve_signed_disposition
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::coordinate_failure_commits_neither_output_nor_projection
    ```
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-014, B-016, B-020

- [ ] `SP132-T3` 锁定public typed error、safe context以及string/TestRenderer caller。 Owner: `gh132-public-error-callers` | Done when: public error shape兼容且nested child ID/source/safe-context/independent caller fixtures完成 | Verify: 运行本任务下列8个exact Rust tests。
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
    6. `gh132_current_head_coverage_contract` 实现fixture/collect(no-op)/produce/validate四模式，
       只从exact diff、raw llvm-cov JSON和固定critical production区域生成/复核provenance。
  - Verify:
    ```sh
    set -euo pipefail
    run_exact() {
      GH132_LIST="$("$@" -- --exact --list --format terse 2>&1)"
      GH132_MATCHED="$(printf '%s\n' "$GH132_LIST" |
        awk -F ': ' '$2 == "test" { count++ } END { print count + 0 }')"
      test "$GH132_MATCHED" -eq 1
      GH132_RESULT="$("$@" -- --exact 2>&1)" || {
        printf '%s\n' "$GH132_RESULT" >&2
        return 1
      }
      printf '%s\n' "$GH132_RESULT"
      GH132_COUNTS="$(printf '%s\n' "$GH132_RESULT" | awk '
        /^test result:/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "passed;") passed += $(i - 1)
            if ($i == "failed;") failed += $(i - 1)
            if ($i == "ignored;") ignored += $(i - 1)
          }
        }
        END { printf "%d %d %d\n", passed, failed, ignored }')"
      test "$GH132_COUNTS" = "1 0 0"
    }
    run_exact cargo test --workspace --lib --locked renderer::error::tests::coordinate_error_context_is_typed_and_does_not_leak_content
    run_exact cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_string_api_with_exact_id
    run_exact cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_test_renderer_with_exact_id
    run_exact cargo test --test text_flow_renderer_error_paths --locked independent_coordinate_failures_do_not_share_owner_or_frame_state
    run_exact cargo test --test text_flow_renderer_error_paths --locked try_render_to_string_preserves_source_and_returns_no_partial_string
    run_exact cargo test --test text_flow_error_paths --locked typed_error_reaches_remaining_callers
    run_exact cargo test --test text_flow_error_paths --locked caller_failure_commits_no_partial_output
    export GH132_COVERAGE_MODE=fixture
    run_exact cargo test --test text_flow_renderer_error_paths --locked gh132_current_head_coverage_contract
    unset GH132_COVERAGE_MODE
    ```
  - Covers: B-008, B-009, B-010, B-011, B-013, B-014, B-016, B-019, B-020, B-021

- [ ] `SP132-T4` 验证dynamic App的canonical ID传播、candidate atomicity与重试边界。 Owner: `gh132-dynamic-transaction` | Done when: static filter/dynamic/App exact original ID、candidate atomicity和clean retry合同全部完成 | Verify: 运行本任务下列7个exact Rust tests。
  - Owner: `gh132-dynamic-transaction`
  - Dependencies: `SP132-T2`、`SP132-T3`。
  - File ownership:
    `src/renderer/pipeline.rs`、
    `src/renderer/app.rs`、
    `src/renderer/static_content.rs`。
  - Done when:
    1. static filter对每个保留dynamic node保持caller original ID，nested child
       NaN/overflow经dynamic pipeline/App保持该exact original child ID；
    2. failure不提交frame string、previous VNode、runtime layout/key aliases、static lines或
       candidate flow/layout cache；
    3. LayoutEngine失败后处于明确clean retry状态，重复failure稳定，corrected retry无重复
       node/cell/alias；
    4. typed renderer failure的candidate从不发布，later successful frame不观察失败
       candidate；同步pipeline没有pre-commit cancellation checkpoint，测试不得把failure
       或drop标成cancellation；
    5. `App::run`既有I/O mapping保留同一`TextRenderError`和nested typed source chain。
  - Verify:
    ```sh
    set -euo pipefail
    run_exact() {
      GH132_LIST="$("$@" -- --exact --list --format terse 2>&1)"
      GH132_MATCHED="$(printf '%s\n' "$GH132_LIST" |
        awk -F ': ' '$2 == "test" { count++ } END { print count + 0 }')"
      test "$GH132_MATCHED" -eq 1
      GH132_RESULT="$("$@" -- --exact 2>&1)" || {
        printf '%s\n' "$GH132_RESULT" >&2
        return 1
      }
      printf '%s\n' "$GH132_RESULT"
      GH132_COUNTS="$(printf '%s\n' "$GH132_RESULT" | awk '
        /^test result:/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "passed;") passed += $(i - 1)
            if ($i == "failed;") failed += $(i - 1)
            if ($i == "ignored;") ignored += $(i - 1)
          }
        }
        END { printf "%d %d %d\n", passed, failed, ignored }')"
      test "$GH132_COUNTS" = "1 0 0"
    }
    run_exact cargo test --workspace --lib --locked renderer::static_content::tests::filter_static_elements_preserves_original_ids_for_retained_dynamic_nodes
    run_exact cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::nested_child_coordinate_errors_keep_id_and_candidate_state
    run_exact cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::repeated_coordinate_failure_then_correction_retries_cleanly
    run_exact cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::failed_coordinate_candidate_is_never_published
    run_exact cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::incremental_failure_retries_from_clean_layout_tree
    run_exact cargo test --workspace --lib --locked renderer::app::tests::nested_child_coordinate_error_reaches_app_io_source_chain
    run_exact cargo test --workspace --lib --locked renderer::app::tests::app_render_candidate_preserves_typed_error_source
    ```
  - Covers: B-008, B-012, B-013, B-015, B-016, B-017, B-019, B-020

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
    4. implementation PR changed files精确等于独立的12路径
       `specrail-planned-changes` manifest；三份已合入packet不在该diff内，也不得被重改；
    5. exact implementation head的新代码line coverage>=80%，signed conversion/owner/
       transaction/static-identity critical line/branches=100%，raw与summary provenance可重算；
    6. fresh CI、independent review、zero unresolved current actionable review threads和
       SpecRail PR gate都绑定同一head SHA。
  - Verify:
    ```sh
    set -euo pipefail
    run_exact() {
      GH132_LIST="$("$@" -- --exact --list --format terse 2>&1)"
      GH132_MATCHED="$(printf '%s\n' "$GH132_LIST" |
        awk -F ': ' '$2 == "test" { count++ } END { print count + 0 }')"
      test "$GH132_MATCHED" -eq 1
      GH132_RESULT="$("$@" -- --exact 2>&1)" || {
        printf '%s\n' "$GH132_RESULT" >&2
        return 1
      }
      printf '%s\n' "$GH132_RESULT"
      GH132_COUNTS="$(printf '%s\n' "$GH132_RESULT" | awk '
        /^test result:/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "passed;") passed += $(i - 1)
            if ($i == "failed;") failed += $(i - 1)
            if ($i == "ignored;") ignored += $(i - 1)
          }
        }
        END { printf "%d %d %d\n", passed, failed, ignored }')"
      test "$GH132_COUNTS" = "1 0 0"
    }
    : "${GH132_IMPLEMENTATION_PR:?set the GH132 implementation PR number}"
    : "${GH132_IMPLEMENTATION_BASE_SHA:?set the exact starting main SHA recorded by SP132-T1}"
    : "${GH132_EVIDENCE_DIR:?set an absolute directory outside the repository}"
    : "${SPEC_RAIL_ROOT:?set the pinned SpecRail checkout root}"
    : "${GH132_PR_EVIDENCE:?set the exact-head PR evidence JSON path}"
    case "$GH132_EVIDENCE_DIR" in /*) ;; *) exit 1 ;; esac
    WORKTREE_ROOT="$(git rev-parse --show-toplevel)"
    WORKTREE_REAL="$(python3 -c \
      'import os,sys; print(os.path.realpath(sys.argv[1]))' "$WORKTREE_ROOT")"
    GH132_EVIDENCE_REAL="$(python3 -c \
      'import os,sys; print(os.path.realpath(sys.argv[1]))' "$GH132_EVIDENCE_DIR")"
    case "$GH132_EVIDENCE_REAL/" in
      "$WORKTREE_REAL/"*) exit 1 ;;
    esac
    GH132_EVIDENCE_DIR="$GH132_EVIDENCE_REAL"
    SPEC_RAIL_REV=bfc60f26164af5df1ebd3b5cb79d07379fc416b7
    test "$(git -C "$SPEC_RAIL_ROOT" rev-parse 'HEAD^{commit}')" = "$SPEC_RAIL_REV"
    test "$(git -C "$SPEC_RAIL_ROOT" remote get-url origin)" = \
      https://github.com/majiayu000/specrail.git
    test "$(python3 -c \
      'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
      "$SPEC_RAIL_ROOT/checks/pr_gate.py")" = \
      10cb7412ff504291d136a2c1486bc96e6b5e811c8040d1f61a8d222994e87873
    test -f "$GH132_PR_EVIDENCE"
    WORKTREE_STATUS="$(git status --porcelain=v1 --untracked-files=all)"
    test -z "$WORKTREE_STATUS"
    git fetch --no-tags origin main
    EXPECTED_CURRENT_MAIN_SHA="$(git rev-parse 'FETCH_HEAD^{commit}')"
    PR_JSON="$(gh pr view "$GH132_IMPLEMENTATION_PR" --repo majiayu000/rnk \
      --json baseRefOid,headRefOid,body,statusCheckRollup)"
    PR_BASE_SHA="$(printf '%s\n' "$PR_JSON" | jq -r '.baseRefOid')"
    PR_HEAD_SHA="$(printf '%s\n' "$PR_JSON" | jq -r '.headRefOid')"
    test "$PR_BASE_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
    test "$(git rev-parse 'HEAD^{commit}')" = "$PR_HEAD_SHA"
    SPEC_RAIL_GATE_REPO="$(mktemp -d \
      "${TMPDIR:-/tmp}/gh132-specrail-pr-gate.XXXXXX")"
    git clone --quiet --no-local --no-checkout "$PWD" "$SPEC_RAIL_GATE_REPO"
    git -C "$SPEC_RAIL_GATE_REPO" checkout --quiet --detach "$PR_HEAD_SHA"
    cp "$SPEC_RAIL_ROOT/workflow.yaml" "$SPEC_RAIL_ROOT/states.yaml" \
      "$SPEC_RAIL_ROOT/labels.yaml" "$SPEC_RAIL_GATE_REPO/"
    cp -R "$SPEC_RAIL_ROOT/schemas" "$SPEC_RAIL_GATE_REPO/"
    test "$(git merge-base "$PR_HEAD_SHA" "$EXPECTED_CURRENT_MAIN_SHA")" = \
      "$EXPECTED_CURRENT_MAIN_SHA"
    git merge-base --is-ancestor "$GH132_IMPLEMENTATION_BASE_SHA" \
      "$EXPECTED_CURRENT_MAIN_SHA"
    IMPLEMENTATION_MANIFEST_JSON="$(sed -n \
      '/<!-- specrail-planned-changes/{n;p;}' specs/GH132/tech.md)"
    test "$(printf '%s\n' "$IMPLEMENTATION_MANIFEST_JSON" | jq -r '.paths | length')" -eq 12
    EXPECTED_CHANGED_PATHS="$(printf '%s\n' "$IMPLEMENTATION_MANIFEST_JSON" |
      jq -r '.paths[]' | LC_ALL=C sort)"
    ACTUAL_CHANGED_PATHS="$(git diff --name-only \
      "$EXPECTED_CURRENT_MAIN_SHA...$PR_HEAD_SHA" | LC_ALL=C sort)"
    test "$ACTUAL_CHANGED_PATHS" = "$EXPECTED_CHANGED_PATHS"
    GH132_MERGE_BASE_SHA="$(git merge-base "$EXPECTED_CURRENT_MAIN_SHA" "$PR_HEAD_SHA")"
    test "$GH132_MERGE_BASE_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
    GH132_DIFF_SHA256="$(git diff --no-ext-diff --binary \
      "$EXPECTED_CURRENT_MAIN_SHA...$PR_HEAD_SHA" -- | python3 -c \
      'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')"
    printf '%s\n' "$PR_JSON" | jq -e --arg head "$PR_HEAD_SHA" \
      --arg base "$PR_BASE_SHA" --arg diff "$GH132_DIFF_SHA256" \
      '.body | contains($head) and contains($base) and contains($diff)' >/dev/null
    test "$(cargo llvm-cov --version)" = "cargo-llvm-cov 0.8.7"
    mkdir -p "$GH132_EVIDENCE_DIR"
    GH132_COVERAGE_RAW="$GH132_EVIDENCE_DIR/llvm-cov.json"
    GH132_COVERAGE_ARTIFACT="$GH132_EVIDENCE_DIR/gh132-coverage.json"
    export GH132_COVERAGE_MODE=fixture
    run_exact cargo test --test text_flow_renderer_error_paths --locked \
      gh132_current_head_coverage_contract
    GH132_COVERAGE_MODE=collect \
      cargo llvm-cov --workspace --all-targets --all-features --locked --branch --json \
        --output-path "$GH132_COVERAGE_RAW"
    export GH132_COVERAGE_MODE=produce
    export GH132_COVERAGE_BASE_SHA="$EXPECTED_CURRENT_MAIN_SHA"
    export GH132_COVERAGE_HEAD_SHA="$PR_HEAD_SHA"
    export GH132_COVERAGE_MERGE_BASE_SHA="$GH132_MERGE_BASE_SHA"
    export GH132_COVERAGE_DIFF_SHA256="$GH132_DIFF_SHA256"
    export GH132_COVERAGE_RAW GH132_COVERAGE_ARTIFACT
    run_exact cargo test --test text_flow_renderer_error_paths --locked \
      gh132_current_head_coverage_contract
    export GH132_COVERAGE_MODE=validate
    run_exact cargo test --test text_flow_renderer_error_paths --locked \
      gh132_current_head_coverage_contract
    python3 "$SPEC_RAIL_ROOT/checks/pr_gate.py" --repo "$SPEC_RAIL_GATE_REPO" \
      --evidence "$GH132_PR_EVIDENCE" --mode required --json
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_signed_coordinates_axis_clips_and_nested_active_clips_are_exact
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_failure_commits_neither_cells_nor_projection
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection
    run_exact cargo test --workspace --lib --locked renderer::tree_renderer::tests::scrolled_out_negative_rows_do_not_paint_at_top
    run_exact cargo test --test prelude_surfaces --locked try_render_to_string_surface
    cargo fmt --all -- --check
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
    cargo test --workspace --all-targets --all-features --locked
    git fetch --no-tags origin main
    FINAL_CURRENT_MAIN_SHA="$(git rev-parse 'FETCH_HEAD^{commit}')"
    FINAL_PR_JSON="$(gh pr view "$GH132_IMPLEMENTATION_PR" \
      --repo majiayu000/rnk \
      --json baseRefOid,headRefOid,body,statusCheckRollup)"
    FINAL_PR_BASE_SHA="$(printf '%s\n' "$FINAL_PR_JSON" |
      jq -r '.baseRefOid')"
    FINAL_PR_HEAD_SHA="$(printf '%s\n' "$FINAL_PR_JSON" |
      jq -r '.headRefOid')"
    FINAL_MERGE_BASE_SHA="$(git merge-base \
      "$FINAL_CURRENT_MAIN_SHA" "$FINAL_PR_HEAD_SHA")"
    test "$FINAL_CURRENT_MAIN_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
    test "$FINAL_PR_BASE_SHA" = "$PR_BASE_SHA"
    test "$FINAL_PR_HEAD_SHA" = "$PR_HEAD_SHA"
    test "$FINAL_MERGE_BASE_SHA" = "$GH132_MERGE_BASE_SHA"
    test "$(git rev-parse 'HEAD^{commit}')" = "$FINAL_PR_HEAD_SHA"
    FINAL_CI_TOTAL="$(printf '%s\n' "$FINAL_PR_JSON" |
      jq '[.statusCheckRollup[] | select(.__typename == "CheckRun")] | length')"
    FINAL_CI_SUCCESS="$(printf '%s\n' "$FINAL_PR_JSON" |
      jq '[.statusCheckRollup[] |
           select(.__typename == "CheckRun" and
                  .status == "COMPLETED" and .conclusion == "SUCCESS")] |
          length')"
    test "$FINAL_CI_TOTAL" -gt 0
    test "$FINAL_CI_SUCCESS" -eq "$FINAL_CI_TOTAL"
    FINAL_CI_SHA256="$(printf '%s\n' "$FINAL_PR_JSON" |
      jq -S '.statusCheckRollup' | python3 -c \
      'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')"
    FINAL_CI_RUN_IDS="$(printf '%s\n' "$FINAL_PR_JSON" | jq -r '
      [.statusCheckRollup[] |
       select(.__typename == "CheckRun") |
       .detailsUrl |
       capture("/actions/runs/(?<run>[0-9]+)(/|$)").run] |
      unique | .[]')"
    test -n "$FINAL_CI_RUN_IDS"
    printf '%s\n' "$FINAL_PR_JSON" |
      jq -e --arg head "$FINAL_PR_HEAD_SHA" \
        --arg base "$FINAL_PR_BASE_SHA" --arg main "$FINAL_CURRENT_MAIN_SHA" \
        --arg merge_base "$FINAL_MERGE_BASE_SHA" \
        --arg diff "$GH132_DIFF_SHA256" \
        '.body | contains($head) and contains($base) and contains($main) and
                 contains($merge_base) and contains($diff)' >/dev/null
    while IFS= read -r run_id; do
      test -n "$run_id"
      printf '%s\n' "$FINAL_PR_JSON" |
        jq -e --arg run_id "$run_id" '.body | contains($run_id)' >/dev/null
    done <<<"$FINAL_CI_RUN_IDS"
    FINAL_WORKTREE_STATUS="$(git status --porcelain=v1 --untracked-files=all)"
    test -z "$FINAL_WORKTREE_STATUS"
    printf 'FINAL_HEAD=%s\nFINAL_BASE=%s\nFINAL_MAIN=%s\nFINAL_MERGE_BASE=%s\nFINAL_CI_SHA256=%s\n' \
      "$FINAL_PR_HEAD_SHA" "$FINAL_PR_BASE_SHA" "$FINAL_CURRENT_MAIN_SHA" \
      "$FINAL_MERGE_BASE_SHA" "$FINAL_CI_SHA256"
    ```
    `PR_BASE_SHA` 必须与fresh current main完全相等，且该 SHA 必须是PR head的exact
    merge-base；仅证明PR137或旧main是ancestor不够。另逐条运行Tech mapping和ledger全部
    命令并证明`matched=1`。coverage contract必须从raw llvm-cov JSON和exact diff重算：
    changed production executable lines>=80%，`CheckedCoordinate`/scoped validation/
    static identity filter/pipeline publish boundary的changed line与branch各100%；artifact
    schema固定为`gh132-current-head-coverage-v1`并含repo、issue、base/head/merge-base、
    diff SHA-256、raw SHA-256、`cargo-llvm-cov 0.8.7`与exact command。missing/zero
    executable、stale SHA、非absolute artifact path、threshold或provenance不符全部失败。
    `collect`模式在llvm-cov递归测试中必须无副作用通过；未设置mode的普通full-suite同样通过，
    但不能produce/validate artifact。produce/validate成功后上述immutable coverage环境必须
    保留到全部mapped/full tests结束。`SPEC_RAIL_GATE_REPO`必须保留到PR gate JSON归档；
    它的`HEAD`必须等于`PR_HEAD_SHA`，overlay只允许来自已校验的固定SpecRail revision，
    不得拿SpecRail自身git history代替implementation history。PR body必须包含同一exact
    head、base、current main、merge-base、diff SHA-256与最终CI run ID。evidence目录
    realpath必须在worktree之外；coverage创建前和
    所有长门禁后worktree都必须clean。最后一次fresh fetch/query必须逐字证明head、base、
    current main和merge-base与长门禁前记录值不变，并从该最终PR JSON验证全部CheckRun
    success、计算CI evidence digest；之后才查询reviewThreads、coverage与SpecRail required
    PR gate。evidence path gate还必须以relative、worktree自身、worktree child和经symlink
    指回worktree的fixture证明均fail closed，并证明一个真实外部目录通过。
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
- T4独占static filter/dynamic pipeline/App source-module tests。
- T5只读。即使T3/T4文件集合不相交，T4也必须等待T3 typed contract稳定，避免对同一错误
  flow并发做不兼容假设。
- PR #137/#142 merge commits均必须属于fresh implementation base；T2开始前对两者各自
  changed-file set、digest、manifest overlap与交集路径完成source-drift refresh，不做共享
  writer或用旧PR元数据替代。

## 验证

- Product invariant set:
  `B-001..B-021`
- Tech mapping set:
  `B-001..B-021`
- Task Covers union:
  `B-001..B-021`
- `specrail-spec-packet-changes`精确包含三份packet；本 spec PR只按该三路径验证。
- `specrail-planned-changes`精确包含十个既有Rust/test文件和两个因800行上限新增的
  `coordinates.rs` test子模块，共12路径；future implementation PR只按该manifest与fresh
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
- #131已由PR #142关闭；开始实现前必须fresh验证其6路径set、digest和与GH132的三路径交集，
  并在交集路径重新定位VirtualText/span-only contract，不能用旧的“无分支”状态跳过。
- 若实现需要manifest外路径、public API变化、第二个coordinate helper、全局/thread-local
  current element或放宽atomic tests，停止并重新走spec review。
