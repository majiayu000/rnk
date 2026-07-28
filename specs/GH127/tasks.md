# Task Plan：线性 styled boundary 归一化

## Linked Issue

GH-127: https://github.com/majiayu000/rnk/issues/127

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Behavior set: `B-001` 至 `B-022`
- Planned implementation manifest:
  `src/layout/text_flow.rs`、`src/layout/text_flow/style_normalization.rs`、
  `src/layout/text_flow/tests.rs`、`src/layout/text_flow/tests/style_normalization.rs`、
  `tests/text_flow_style_normalization.rs`
- Upstream behavior contract: [`../GH58/product.md`](../GH58/product.md)、
  [`../GH58/tech.md`](../GH58/tech.md)、[`../GH58/tasks.md`](../GH58/tasks.md)
- Merged ordering contract: #126 / PR #136 merge
  `50f6a203c1861814d288d4bdeae0e28d877af34c`

## 当前实现门

packet 文本不得充当 live readiness 证据；此次 spec PR 与已有 `write_spec` route artifact
都不授权 implementation。开始 `SP127-T1` 前，coordinator 必须 fresh 证明：

1. 本三文件 spec-only PR 已 merged，并有绑定 exact spec head/scope 的 human approval。
2. 仅在第 1 项满足后由 maintainer 将唯一 readiness 从 `ready_to_spec` 替换为
   `ready_to_implement`；agent 不改 label。
3. 紧邻 implementation route gate fresh 查询 live issue，按 pinned `labels.yaml` 要求
   恰好一个 canonical readiness，并把查询所得 state 传给 gate；非
   `ready_to_implement`、closed 或冲突状态均 fail closed。
4. #126 merge `50f6a203c1861814d288d4bdeae0e28d877af34c` 是 implementation
   head ancestor。
5. duplicate search 未发现 GH-127 implementation PR、remote/local branch、worktree owner；
   创建恰好一个 implementation branch/PR。
6. base 包含 #128 PR #134、#129 PR #135、#130 PR #138 的 merge commits。
7. manifest 五路径和 GH-58 spec refs 仍存在；current API/error/diagnostic/cache shape 与本
   packet 一致。任一失败保持 blocked，不改 label、不创建 implementation commit。

## 实现任务

- [ ] `SP127-T1` 执行 dependency、duplicate、route、current-API 与 root-cause preflight。 Owner: `gh127-preflight-owner` | Done when: fresh evidence bundle 和 red root-cause reproduction 完整 | Verify: T1 preflight、baseline check、exact style/property tests 全部通过。
  一份 fresh、只读 evidence bundle 绑定
  implementation base/head，证明 spec approval/readiness、#126 exact merge ancestry、
  #128/#129/#130 ancestry、零 duplicate owner、五路径 manifest 与 PR #109 unresolved
  thread；另在隔离
  scratch checkout 记录 current nested range scans 和 2k/4k/8k red operation-count
  reproduction，不向 implementation branch 提交红测。下列 preflight commands
  全部 nonzero-fail-closed，root-cause evidence 明确显示 4k/8k density 违反 B-002 而现有
  semantic regressions仍 green。
  - Dependencies: human implementation gate。
  - File ownership: 无 target writable path；只读 repo/GitHub evidence 与 scratch artifact。
  - Covers: B-001, B-002, B-003, B-018, B-019, B-020, B-022。
  - Verify:
    `git merge-base --is-ancestor 50f6a203c1861814d288d4bdeae0e28d877af34c HEAD`；
    `git merge-base --is-ancestor "$GH128_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH129_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH130_MERGE_SHA" HEAD`；
    `rg -n 'styled_ranges|StyleBoundaryNormalized|tokenize_source' src/layout/text_flow.rs`；
    `cargo check --workspace --all-targets --all-features --locked`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes -- --exact`；
    `PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact`。
  - Handoff: 记录 exact base、#126 merge SHA、duplicate evidence、root-cause
    counter/raw output、现有 semantic outputs；T2 接受 handoff 后 T1 不写任何 owned path。

- [ ] `SP127-T2` 实现 typed validation plan、monotonic style/boundary normalization 与 deterministic private counter。 Owner: `gh127-normalization-core-owner` | Done when: linear merge、compatibility、polling与 L1-L5 完整 | Verify: GH127-L1 至 GH127-L5 及 T2 regressions 各 1 passed/0 ignored。
  validation 保留 caller-first invalid 与 sorted
  overlap pair；private plan 保存 original range/endpoint ordinals；style/boundary cursor
  对 post-validation `G+R` 单调前进；adjacent/empty/unsorted diagnostics 顺序与重数完全
  保持；range preprocessing 有 bounded interruption poll；ASCII、high-density
  combining/ZWJ 与 one-EGC skew 三类 2k/4k/8k production counter 在 debug/release 满足
  absolute+slope，内部 fixtures 的 ordered projection 非零并匹配 exact event count，
  negative bound diagnostics 完整；所有 error/cancellation 不产生 partial result。
  private build count 的失败原子性由 L5 精确断言。T2 regression commands恰好各
  1 passed/0 ignored。
  - Dependencies: SP127-T1 完整 handoff；#126 exact merge ancestor 已证明。
  - File ownership: 独占 `src/layout/text_flow.rs`、
    `src/layout/text_flow/style_normalization.rs`、`src/layout/text_flow/tests.rs` 与
    `src/layout/text_flow/tests/style_normalization.rs`；自然移动现有 styled-normalization
    unit bodies 到新子模块，父文件只保留 module declaration/必要 stable selector wrapper，
    最终 `tests.rs <= 800` 行，禁止压缩/削弱测试。不得修改 `wrap.rs`、`truncate.rs`、
    engine、property/integration/CI 文件。T2 完成后冻结四个文件，只允许因 T3 暴露真实
    production defect 时显式 handback。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-015, B-016, B-017, B-018, B-021。
  - Verify:
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact`；
    `cargo test --release --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::style_boundary_event_order_and_multiplicity_are_stable -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_range_extremes_preserve_typed_errors -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_normalization_polling_and_cache_count_are_atomic -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_styled_runs -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_interruption -- --exact`。
  - Handoff: 交付 operation definition、2k/4k/8k raw counts/bounds、private plan invariants、
    exact test outputs 与 source freeze SHA；T3 只消费 public surface。

- [ ] `SP127-T3` 建立 public behavior integration oracle 并运行 dependency regressions。 Owner: `gh127-public-contract-owner` | Done when: GH127-L6 至 L13 与 GH-58/#126/#128/#129/#130 contracts 全 green | Verify: T3 public exact/regression commands 各 1 passed/0 ignored或完整 target green。
  `tests/text_flow_style_normalization.rs` 只用 public API 覆盖 combining、ZWJ、adjacent、
  internal empty、合法未排序、default style、reverse/non-char/out-of-bounds/
  `usize::MAX`、overlap、cache vector order/style/endpoint changes、immediate/during-build
  cancellation、previous Arc/cache identity/完整 flow 与 retry-cold parity；integration
  不读取 private `build_count`、不复制 merge 算法/计数器、不访问 clock，也不得推动 public
  accessor；critical ledger GH127-L6 至 GH127-L13、现有 property/engine/truncation及 #126
  tests 全 green。下列 public exact/regression commands全部满足首行 Verify。
  - Dependencies: SP127-T2 source freeze/handoff。
  - File ownership: 独占 `tests/text_flow_style_normalization.rs`；默认不得写 T2 两文件。
    若发现 production defect，停止 T3，显式把 ownership handback 给 T2 修正并重跑
    T2 全部 gates；禁止在 integration test 内绕过或弱化。
  - Covers: B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012,
    B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021。
  - Verify:
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_first_source_style_and_diagnostics -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_typed_failures -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_complete_flow_identity -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_exact_cache_identity -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_failure_precedence_is_stable -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_failures_and_interruption_are_atomic -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_retry_matches_cold_build -- --exact`；
    `PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact`；
    `cargo test --test text_flow_truncate_regressions --locked`；
    `cargo test --workspace --lib --locked layout::engine::text_flow_bridge::tests::replace_and_reorder_preserve_only_live_flows -- --exact`；
    `cargo test --workspace --lib --locked layout::engine::context_sync::tests::identical_context_sync_keeps_text_leaf_and_root_clean_and_reuses_flow -- --exact`；
    `cargo test --workspace --lib --locked layout::engine::context_sync::tests::source_style_wrap_and_overflow_changes_dirty_only_the_affected_text_path -- --exact`；
    `cargo test --test text_flow_wrap_interruption --locked`。
  - Handoff: 交付 GH127-L6..L13 raw outputs、complete-flow equality/cache evidence、
    dependency regression outputs与 exact implementation head；冻结唯一 integration file。

- [ ] `SP127-T4` 完成 immutable exact-head closure audit。 Owner: `gh127-verification-review-owner` | Done when: B/ledger/manifest/coverage/full CI/SpecRail/review closure 全部 fresh | Verify: tech Verification Plan 与 fresh GitHub evidence 全部通过。
  product/tech/tasks B-set 均 exact `B-001..B-022`，task Covers union 无遗漏；
  manifest 只从 fresh remote-main/base 的 exact regular `tech.md` blob读取，五路径是唯一完整
  allowed set；NUL raw diff只接受其非空 `A/M` regular-mode子集；父 unit file <=800 行；
  GH127-L1..L13 各先经 harness inventory 证明 selector 恰好一个，再实际执行且
  `matched=1/passed=1/ignored=0`；debug/release counts、property、dependency regressions、full Rust、
  >=80% changed production line coverage、critical normalization line/branch 100%、
  closure 前后 fresh remote main、GitHub PR base/head、caller base/local head与merge-base
  全等，raw diff/tree/LCOV SHA-256 provenance、exact Git blob/OID只读source materialization、
  pinned revision/checker SHA-256 + byte-identical blob-bound SpecRail input、exact-head
  hosted CI、独立 review 与零 unresolved non-outdated current threads 全部 fresh；PR #109
  thread只由human在证据满足后处理。
  - Dependencies: SP127-T1、SP127-T2、SP127-T3 完成并停止写所有 owned paths。
  - File ownership: 无 writable path；纯只读 verification/review，不 resolve thread、
    approve、merge、改 label 或修测试。任一失败 handback 给对应 owner并使全部旧
    head-bound evidence失效。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019,
    B-020, B-021, B-022。
  - Verify:
    `cargo fmt --all -- --check`；
    `cargo check --workspace --all-targets --all-features --locked`；
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
    `cargo test --workspace --all-targets --all-features --locked`；
    tech Verification Plan 与下方reference的fresh exact-head `cargo llvm-cov` + raw
    diff/blob/LCOV fail-closed verifier，并保留raw LCOV、exact tree manifest、checksum/
    provenance JSON；所有build/evidence output位于只读source tree外；LCOV `SF:`仅精确匹配
    blob manifest中的tracked Rust path，不从mutable checkout读取EOF，`LF/LH`/`DA`与
    `BRF/BRH`/`BRDA` summary一致，两个planned production record的changed-executable交集
    均非空；每条`DA/BRDA` line在对应exact blob的`1..=EOF`；
    NUL raw-diff Cargo.toml、source/destination symlink、existing target、selector ignored、
    suffix/outside/duplicate `SF:`、line 0/超 EOF `DA/BRDA`、negative `DA` hits、
    negative/invalid `BRDA` taken、empty/deleted DA、summary mismatch、bad hash与shell
    early-failure negative fixtures；
    所有 shell block 第一行 `set -euo pipefail`，临时路径统一
    `${TMPDIR:-/tmp}` + `mktemp`，可由 Unix Bash 或 Windows Git Bash/MSYS2 执行；
    absolute interpreter、清空Python startup/path注入、每次`-I -S`；fixed revision
    `23caa70e76904eaa82323208d645d5781a365649` external descriptor-materialized mirror
    中的checker SHA-256、base/head byte-identical GH127/GH58 exact blobs、
    `check_workflow.py`与`route_gate.py`显式trusted import path（同时记录target route
    gate不存在）；
    fresh GraphQL reviewThreads 与 exact `headRefOid` check rollup。
  - Handoff: 向 human maintainer 提交 exact head、dependency SHAs、2k/4k/8k counts、
    ledger 13/13、B coverage 22/22、manifest/diff、raw LCOV + checksum/provenance、
    SpecRail checker/input hashes、CI/review JSON；不宣称 final approval/merge。

### SP127-T4 exact-head closure reference

下列 block 固定 closure 的信任边界与顺序；ledger/focused commands仍按 tech ledger和上述
T4 Verify逐项执行。`GH127_EVIDENCE_DIR`必须是调用者预先创建的空、mode `0700`、
worktree外绝对目录，成功证据不在退出时删除。

```sh
set -euo pipefail
: "${IMPLEMENTATION_PR:?set implementation PR}" \
  "${BASE_SHA:?set caller base SHA}" \
  "${GH127_EVIDENCE_DIR:?set external empty evidence directory}" \
  "${SPEC_RAIL_ROOT:?set external SpecRail object repository}"
ABS_PYTHON="$(type -P python3)"
GIT_BIN="$(type -P git)"
case "$ABS_PYTHON:$GIT_BIN:$GH127_EVIDENCE_DIR" in /*:/*:/*) ;; *) exit 1 ;; esac
while IFS= read -r PYTHON_ENV_NAME
do
  unset "$PYTHON_ENV_NAME"
done < <(compgen -A variable PYTHON)
unset PYTHON_ENV_NAME
export PYTHONNOUSERSITE=1 GIT_NO_REPLACE_OBJECTS=1
umask 077
WORKTREE_ROOT="$("$GIT_BIN" rev-parse --show-toplevel)"
WORKTREE_REAL="$("$ABS_PYTHON" -I -S -c \
  'import os,sys; print(os.path.realpath(sys.argv[1]))' "$WORKTREE_ROOT")"
EVIDENCE_REAL="$("$ABS_PYTHON" -I -S -c \
  'import os,sys; print(os.path.realpath(sys.argv[1]))' "$GH127_EVIDENCE_DIR")"
test -d "$EVIDENCE_REAL"
case "$EVIDENCE_REAL/" in "$WORKTREE_REAL/"*) exit 1 ;; esac
test -z "$(find "$EVIDENCE_REAL" -mindepth 1 -print -quit)"
test -z "$("$GIT_BIN" status --porcelain=v1 --untracked-files=all)"
BASE_SHA="$("$GIT_BIN" rev-parse "$BASE_SHA^{commit}")"
HEAD_SHA="$("$GIT_BIN" rev-parse 'HEAD^{commit}')"
"$GIT_BIN" fetch --no-tags origin main
EXPECTED_CURRENT_MAIN_SHA="$("$GIT_BIN" rev-parse 'FETCH_HEAD^{commit}')"
PR_BEFORE="$EVIDENCE_REAL/pr-before.json"
gh pr view "$IMPLEMENTATION_PR" --repo majiayu000/rnk \
  --json baseRefOid,headRefOid > "$PR_BEFORE"
PR_BASE_SHA="$(jq -r .baseRefOid "$PR_BEFORE")"
PR_HEAD_SHA="$(jq -r .headRefOid "$PR_BEFORE")"
MERGE_BASE_SHA="$("$GIT_BIN" merge-base "$PR_BASE_SHA" "$PR_HEAD_SHA")"
test "$BASE_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
test "$PR_BASE_SHA" = "$BASE_SHA"
test "$MERGE_BASE_SHA" = "$BASE_SHA"
test "$PR_HEAD_SHA" = "$HEAD_SHA"
"$GIT_BIN" merge-base --is-ancestor \
  50f6a203c1861814d288d4bdeae0e28d877af34c "$HEAD_SHA"
APPROVED_TECH_ENTRY="$("$GIT_BIN" ls-tree "$BASE_SHA" -- specs/GH127/tech.md)"
test "$(printf '%s\n' "$APPROVED_TECH_ENTRY" | awk '{print $1" "$2}')" = \
  "100644 blob"
APPROVED_TECH_OID="$(printf '%s\n' "$APPROVED_TECH_ENTRY" | awk '{print $3}')"
IMPLEMENTATION_MANIFEST_JSON="$("$GIT_BIN" cat-file blob "$APPROVED_TECH_OID" |
  "$ABS_PYTHON" -I -S -c 'import sys; marker="<!-- specrail-planned-changes"; rows=sys.stdin.read().splitlines(); hits=[i for i,row in enumerate(rows) if row.startswith(marker)]; len(hits)==1 and hits[0]+1<len(rows) or sys.exit("trusted manifest missing/duplicated"); print(rows[hits[0]+1])')"

verify_raw_diff() {
  "$GIT_BIN" diff --raw -z --no-renames "$BASE_SHA...$HEAD_SHA" -- |
    "$ABS_PYTHON" -I -S -c '
import json,sys
from pathlib import PurePosixPath as P
manifest=json.loads(sys.argv[1]); expected=manifest.get("paths")
if manifest.get("issue") != 127 or manifest.get("complete") is not True:
    raise SystemExit("wrong trusted manifest identity")
if not isinstance(expected,list) or len(expected)!=len(set(expected)) or len(expected)!=5:
    raise SystemExit("trusted manifest must contain five unique paths")
raw=[item for item in sys.stdin.buffer.read().split(b"\0") if item]
if not raw or len(raw)%2: raise SystemExit("empty/malformed raw diff")
rows=[(meta.split(),path.decode("utf-8")) for meta,path in zip(raw[0::2],raw[1::2])]
actual=[path for _,path in rows]
required={"src/layout/text_flow/style_normalization.rs",
          "src/layout/text_flow/tests/style_normalization.rs",
          "tests/text_flow_style_normalization.rs"}
def valid(meta,path):
    canonical=P(path)
    if len(meta)!=5 or meta[4] not in {b"A",b"M"}: return False
    if meta[1] not in {b"100644",b"100755"}: return False
    if meta[4]==b"A" and meta[0]!=b":000000": return False
    if meta[4]==b"M" and meta[0] not in {b":100644",b":100755"}: return False
    return path and not canonical.is_absolute() and ".." not in canonical.parts and canonical.as_posix()==path
if len(actual)!=len(set(actual)) or not set(actual)<=set(expected) or not required<=set(actual):
    raise SystemExit("implementation diff does not match trusted manifest")
if not all(valid(meta,path) for meta,path in rows):
    raise SystemExit("unsafe status/mode/path in implementation raw diff")
' "$IMPLEMENTATION_MANIFEST_JSON"
}
verify_raw_diff
test "$("$GIT_BIN" show "$HEAD_SHA:src/layout/text_flow/tests.rs" | wc -l |
  tr -d ' ')" -le 800
DIFF_SHA256="$("$GIT_BIN" diff --no-ext-diff --binary \
  "$BASE_SHA...$HEAD_SHA" -- |
  "$ABS_PYTHON" -I -S -c \
  'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')"

materialize_tree() {
  local object_repo="$1" revision="$2" destination="$3" manifest_name="$4"
  "$ABS_PYTHON" -I -S - "$GIT_BIN" "$object_repo" "$revision" \
    "$destination" "$EVIDENCE_REAL" "$manifest_name" <<'PY'
import hashlib,json,os,stat,subprocess,sys
from pathlib import PurePosixPath as P
git,repo,rev,destination,evidence,manifest_name=sys.argv[1:]
nofollow=getattr(os,"O_NOFOLLOW",None); directory=getattr(os,"O_DIRECTORY",None)
if nofollow is None or directory is None or os.open not in os.supports_dir_fd or os.mkdir not in os.supports_dir_fd:
    raise SystemExit("descriptor-relative nofollow materialization unavailable")
if not os.path.isabs(destination) or not os.path.isabs(evidence):
    raise SystemExit("materialization roots must be absolute")
if P(manifest_name).name != manifest_name: raise SystemExit("unsafe manifest name")
env={**os.environ,"GIT_NO_REPLACE_OBJECTS":"1"}
def run(*args):
    return subprocess.run([git,"-C",repo,*args],check=True,stdout=subprocess.PIPE,env=env).stdout
if run("rev-parse","--show-object-format").strip()!=b"sha1":
    raise SystemExit("unexpected Git object format")
tree=run("rev-parse",f"{rev}^{{tree}}").decode().strip()
raw=[row for row in run("ls-tree","-rz","--full-tree","-r",rev).split(b"\0") if row]
entries=[]; seen=set()
for row in raw:
    meta,path_bytes=row.split(b"\t",1); mode,kind,oid=meta.split()
    path=path_bytes.decode("utf-8"); rel=P(path)
    if mode not in {b"100644",b"100755"} or kind!=b"blob":
        raise SystemExit(f"non-regular tree entry: {path}")
    if not path or rel.is_absolute() or ".." in rel.parts or rel.as_posix()!=path or path in seen:
        raise SystemExit(f"unsafe/duplicate tree path: {path}")
    seen.add(path); entries.append((rel,mode.decode(),oid.decode()))
flags=os.O_RDONLY|nofollow; dir_flags=flags|directory
root=os.open(destination,dir_flags); evidence_fd=os.open(evidence,dir_flags)
created=set()
def parent_fd(rel):
    current=os.dup(root)
    for depth,part in enumerate(rel.parts[:-1],1):
        try: os.mkdir(part,0o700,dir_fd=current)
        except FileExistsError: pass
        child=os.open(part,dir_flags,dir_fd=current); os.close(current); current=child
        created.add(rel.parts[:depth])
    return current
manifest={}
for rel,mode,oid in entries:
    data=run("cat-file","blob",oid)
    actual=hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest()
    if actual!=oid: raise SystemExit(f"blob OID mismatch: {rel}")
    parent=parent_fd(rel)
    leaf=os.open(rel.name,os.O_WRONLY|os.O_CREAT|os.O_EXCL|nofollow,0o600,dir_fd=parent)
    if not stat.S_ISREG(os.fstat(leaf).st_mode): raise SystemExit(f"non-regular target: {rel}")
    with os.fdopen(leaf,"wb") as handle:
        handle.write(data); handle.flush()
        os.fchmod(handle.fileno(),0o555 if mode=="100755" else 0o444)
    os.close(parent)
    manifest[rel.as_posix()]={"mode":mode,"oid":oid,"lines":len(data.splitlines())}
def open_dir(parts):
    current=os.dup(root)
    for part in parts:
        child=os.open(part,dir_flags,dir_fd=current); os.close(current); current=child
    return current
for parts in sorted(created,key=len,reverse=True):
    descriptor=open_dir(parts); os.fchmod(descriptor,0o555); os.close(descriptor)
os.fchmod(root,0o555)
payload=(json.dumps({"revision":rev,"tree":tree,"entries":manifest},
                    sort_keys=True,separators=(",",":"))+"\n").encode()
out=os.open(manifest_name,os.O_WRONLY|os.O_CREAT|os.O_EXCL|nofollow,0o400,dir_fd=evidence_fd)
with os.fdopen(out,"wb") as handle: handle.write(payload)
os.close(root); os.close(evidence_fd)
PY
}

SOURCE_ROOT="$(mktemp -d "$EVIDENCE_REAL/source.XXXXXX")"
SOURCE_MANIFEST_NAME=source-blobs.json
materialize_tree "$WORKTREE_ROOT" "$HEAD_SHA" "$SOURCE_ROOT" "$SOURCE_MANIFEST_NAME"
SOURCE_MANIFEST="$EVIDENCE_REAL/$SOURCE_MANIFEST_NAME"
SOURCE_MANIFEST_SHA256="$(shasum -a 256 "$SOURCE_MANIFEST" | awk '{print $1}')"
test "$("$GIT_BIN" ls-tree -r "$BASE_SHA" -- specs/GH127 specs/GH58)" = \
  "$("$GIT_BIN" ls-tree -r "$HEAD_SHA" -- specs/GH127 specs/GH58)"
export CARGO_TARGET_DIR="$EVIDENCE_REAL/cargo-target"
mkdir "$CARGO_TARGET_DIR"
(
  cd "$SOURCE_ROOT"
  cargo fmt --all -- --check
  cargo check --workspace --all-targets --all-features --locked
  cargo clippy --workspace --all-targets --all-features --locked -- \
    -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
  cargo test --workspace --all-targets --all-features --locked
  cargo llvm-cov clean --workspace
  cargo llvm-cov --branch --workspace --lib --all-features --lcov \
    --output-path "$EVIDENCE_REAL/coverage.lcov" --locked
)
LCOV_PATH="$EVIDENCE_REAL/coverage.lcov"
LCOV_SHA256="$(shasum -a 256 "$LCOV_PATH" | awk '{print $1}')"
PROVENANCE_PATH="$EVIDENCE_REAL/coverage-provenance.json"
"$ABS_PYTHON" -I -S - "$GIT_BIN" "$WORKTREE_ROOT" "$BASE_SHA" "$HEAD_SHA" \
  "$SOURCE_ROOT" "$SOURCE_MANIFEST" "$SOURCE_MANIFEST_SHA256" \
  "$LCOV_PATH" "$LCOV_SHA256" "$DIFF_SHA256" "$PROVENANCE_PATH" <<'PY'
import hashlib,json,os,re,subprocess,sys
git,repo,base,head,root,manifest_path,manifest_sha,lcov_path,lcov_sha,diff_sha,out=sys.argv[1:]
def read_regular(path):
    fd=os.open(path,os.O_RDONLY|getattr(os,"O_NOFOLLOW",0))
    import stat
    if not stat.S_ISREG(os.fstat(fd).st_mode): raise ValueError("non-regular evidence")
    with os.fdopen(fd,"rb") as handle: data=handle.read(100_000_001)
    if len(data)>100_000_000: raise ValueError("oversized evidence")
    return data
manifest_bytes=read_regular(manifest_path)
lcov_bytes=read_regular(lcov_path)
if hashlib.sha256(manifest_bytes).hexdigest()!=manifest_sha: raise SystemExit("manifest hash mismatch")
if not lcov_bytes or hashlib.sha256(lcov_bytes).hexdigest()!=lcov_sha: raise SystemExit("LCOV hash mismatch")
manifest=json.loads(manifest_bytes); entries=manifest["entries"]
production=("src/layout/text_flow.rs","src/layout/text_flow/style_normalization.rs")
def git_run(*args,text=False):
    return subprocess.run([git,"-C",repo,*args],check=True,stdout=subprocess.PIPE,
                          text=text,env={**os.environ,"GIT_NO_REPLACE_OBJECTS":"1"}).stdout
for path in production:
    meta=git_run("ls-tree",head,"--",path,text=True).strip().split()
    if len(meta)<3 or meta[0] not in {"100644","100755"} or meta[1]!="blob": raise SystemExit("bad production blob")
    data=git_run("cat-file","blob",meta[2])
    oid=hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest()
    if entries.get(path)!={"mode":meta[0],"oid":oid,"lines":len(data.splitlines())}:
        raise SystemExit("production blob manifest mismatch")
nofollow=getattr(os,"O_NOFOLLOW",None); directory=getattr(os,"O_DIRECTORY",None)
if nofollow is None or directory is None or os.open not in os.supports_dir_fd:
    raise SystemExit("descriptor-relative source revalidation unavailable")
root_fd=os.open(root,os.O_RDONLY|nofollow|directory); actual=set()
def walk(parent,prefix=()):
    for name in os.listdir(parent):
        child=os.open(name,os.O_RDONLY|nofollow,dir_fd=parent); info=os.fstat(child)
        path="/".join((*prefix,name))
        if __import__("stat").S_ISDIR(info.st_mode):
            walk(child,(*prefix,name)); os.close(child); continue
        if not __import__("stat").S_ISREG(info.st_mode) or path not in entries:
            raise SystemExit(f"unexpected materialized entry: {path}")
        with os.fdopen(child,"rb") as handle: data=handle.read()
        oid=hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest()
        executable="100755" if info.st_mode&0o111 else "100644"
        if oid!=entries[path]["oid"] or executable!=entries[path]["mode"]:
            raise SystemExit(f"materialized source drift: {path}")
        actual.add(path)
walk(root_fd); os.close(root_fd)
if actual!=set(entries): raise SystemExit("materialized source set mismatch")
expected={os.path.join(root,path):path for path in entries if path.endswith(".rs")}
records={}; record=None
for line in lcov_bytes.decode().splitlines():
    if line.startswith("SF:"):
        if record is not None or line[3:] not in expected: raise ValueError("unexpected/nested SF")
        path=expected[line[3:]]
        if path in records: raise ValueError("duplicate SF")
        record={"path":path,"lines":{},"branches":{},"lf":None,"lh":None,"brf":None,"brh":None}
    elif line=="end_of_record":
        if record is None or not record["lines"]: raise ValueError("empty/orphan LCOV record")
        if record["lf"]!=len(record["lines"]) or record["lh"]!=sum(v>0 for v in record["lines"].values()):
            raise ValueError("LF/LH mismatch")
        hits=sum(v is not None and v>0 for v in record["branches"].values())
        if record["brf"]!=len(record["branches"]) or record["brh"]!=hits: raise ValueError("BRF/BRH mismatch")
        records[record["path"]]=record; record=None
    elif record is not None and line.startswith("DA:"):
        number,hits,*_=line[3:].split(","); number=int(number); hits=int(hits)
        if not 1<=number<=entries[record["path"]]["lines"] or hits<0 or number in record["lines"]:
            raise ValueError("invalid DA")
        record["lines"][number]=hits
    elif record is not None and line.startswith("BRDA:"):
        number,block,branch,taken=line[5:].split(","); number=int(number)
        value=None if taken=="-" else int(taken); key=(number,block,branch)
        if not 1<=number<=entries[record["path"]]["lines"] or (value is not None and value<0) or key in record["branches"]:
            raise ValueError("invalid BRDA")
        record["branches"][key]=value
    elif record is not None and line.startswith(("LF:","LH:","BRF:","BRH:")):
        key,value=line.split(":",1); field=key.lower()
        if record[field] is not None: raise ValueError("duplicate summary")
        record[field]=int(value)
if record is not None or not set(production)<=records.keys(): raise SystemExit("missing/unterminated production record")
diff=git_run("diff","--unified=0",f"{base}...{head}","--",*production,text=True)
changed={path:set() for path in production}; current=None
for line in diff.splitlines():
    if line.startswith("+++ b/"): current=line[6:]
    elif current in changed and line.startswith("@@"):
        match=re.search(r"\+(\d+)(?:,(\d+))?",line)
        if not match: raise SystemExit("bad diff hunk")
        start,count=int(match.group(1)),int(match.group(2) or 1)
        changed[current].update(range(start,start+count))
covered=[]
for path in production:
    executable=changed[path]&records[path]["lines"].keys()
    if not executable: raise SystemExit("empty changed executable intersection")
    covered.extend(records[path]["lines"][line] for line in executable)
critical=records[production[1]]
if any(v<=0 for v in critical["lines"].values()) or not critical["branches"] or any(v is None or v<=0 for v in critical["branches"].values()):
    raise SystemExit("critical line/branch coverage below 100%")
if sum(v>0 for v in covered)*100<len(covered)*80: raise SystemExit("changed line coverage below 80%")
payload={"schema_version":2,"base_sha":base,"head_sha":head,"merge_base_sha":base,
         "diff_sha256":diff_sha,"tree_sha":manifest["tree"],"source_manifest_sha256":manifest_sha,
         "lcov_sha256":lcov_sha,"changed_executable_lines":len(covered),
         "covered_changed_executable_lines":sum(v>0 for v in covered)}
fd=os.open(out,os.O_WRONLY|os.O_CREAT|os.O_EXCL|getattr(os,"O_NOFOLLOW",0),0o400)
with os.fdopen(fd,"w",encoding="utf-8") as handle: json.dump(payload,handle,sort_keys=True); handle.write("\n")
PY

SPEC_RAIL_REV=23caa70e76904eaa82323208d645d5781a365649
test "$("$GIT_BIN" -C "$SPEC_RAIL_ROOT" rev-parse "$SPEC_RAIL_REV^{commit}")" = \
  "$SPEC_RAIL_REV"
SPEC_RAIL_MIRROR="$(mktemp -d "$EVIDENCE_REAL/specrail.XXXXXX")"
materialize_tree "$SPEC_RAIL_ROOT" "$SPEC_RAIL_REV" "$SPEC_RAIL_MIRROR" \
  specrail-blobs.json
test "$(shasum -a 256 "$SPEC_RAIL_MIRROR/checks/check_workflow.py" | awk '{print $1}')" = \
  8c791545f78d93649385ef0f9780454a7d4552f8da06da1fdee0de9cb8030a7e
test "$(shasum -a 256 "$SPEC_RAIL_MIRROR/checks/route_gate.py" | awk '{print $1}')" = \
  56954390bc5f9733601d94b5d18f78a7d5179c07fc47cd6dd8e8135685c8ac4a
run_specrail() {
  "$ABS_PYTHON" -I -S -c 'import runpy,sys; from pathlib import Path; checks=Path(sys.argv[1]); entry=(checks/sys.argv[2]); entry.is_file() and entry.parent==checks or sys.exit("unsafe SpecRail entry"); sys.path.insert(0,str(checks)); sys.argv=[str(entry),*sys.argv[3:]]; runpy.run_path(str(entry),run_name="__main__")' \
    "$SPEC_RAIL_MIRROR/checks" "$@"
}
run_specrail check_workflow.py --repo "$SPEC_RAIL_MIRROR" \
  --spec-dir "$SOURCE_ROOT/specs/GH127"
ISSUE_JSON="$EVIDENCE_REAL/gh127-live-issue.json"
gh issue view 127 --repo majiayu000/rnk --json state,labels > "$ISSUE_JSON"
READINESS="$("$ABS_PYTHON" -I -S - "$SPEC_RAIL_MIRROR/checks" \
  "$SPEC_RAIL_MIRROR/labels.yaml" "$ISSUE_JSON" <<'PY'
import json,sys
from pathlib import Path
checks=Path(sys.argv[1]); sys.path.insert(0,str(checks))
from specrail_lib import load_yaml_file
issue=json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
canonical=set(load_yaml_file(Path(sys.argv[2]))["labels"]["readiness"])
matches=sorted({item["name"] for item in issue["labels"]}&canonical)
if issue["state"]!="OPEN" or len(matches)!=1:
    raise SystemExit(f"expected OPEN issue with one readiness; got {issue['state']} {matches}")
print(matches[0])
PY
)"
test "$READINESS" = ready_to_implement
run_specrail route_gate.py --repo "$SPEC_RAIL_MIRROR" --route implement \
  --issue 127 --state "$READINESS" --mode required --json

"$GIT_BIN" -C "$WORKTREE_ROOT" fetch --no-tags origin main
FINAL_CURRENT_MAIN_SHA="$("$GIT_BIN" -C "$WORKTREE_ROOT" rev-parse 'FETCH_HEAD^{commit}')"
PR_AFTER="$EVIDENCE_REAL/pr-after.json"
gh pr view "$IMPLEMENTATION_PR" --repo majiayu000/rnk \
  --json baseRefOid,headRefOid > "$PR_AFTER"
FINAL_PR_BASE_SHA="$(jq -r .baseRefOid "$PR_AFTER")"
FINAL_PR_HEAD_SHA="$(jq -r .headRefOid "$PR_AFTER")"
FINAL_MERGE_BASE_SHA="$("$GIT_BIN" -C "$WORKTREE_ROOT" merge-base \
  "$FINAL_PR_BASE_SHA" "$FINAL_PR_HEAD_SHA")"
test "$FINAL_CURRENT_MAIN_SHA" = "$EXPECTED_CURRENT_MAIN_SHA"
test "$FINAL_PR_BASE_SHA" = "$BASE_SHA"
test "$FINAL_PR_HEAD_SHA" = "$HEAD_SHA"
test "$FINAL_MERGE_BASE_SHA" = "$BASE_SHA"
test "$("$GIT_BIN" -C "$WORKTREE_ROOT" rev-parse 'HEAD^{commit}')" = "$HEAD_SHA"
test "$("$GIT_BIN" -C "$WORKTREE_ROOT" diff --no-ext-diff --binary \
  "$BASE_SHA...$HEAD_SHA" -- |
  "$ABS_PYTHON" -I -S -c \
  'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())')" = \
  "$DIFF_SHA256"
verify_raw_diff
test "$(shasum -a 256 "$SOURCE_MANIFEST" | awk '{print $1}')" = \
  "$SOURCE_MANIFEST_SHA256"
test -z "$("$GIT_BIN" -C "$WORKTREE_ROOT" status \
  --porcelain=v1 --untracked-files=all)"
```

coverage parser的negative matrix必须对同一 parser入口逐项注入并断言nonzero；descriptor
materializer也必须以source ancestor symlink、destination ancestor symlink、existing target、
tampered blob bytes和non-regular tree-entry fixtures证明在首个target写入前失败。SpecRail
route gate紧邻执行前fresh读取issue state/labels，从verified mirror的`labels.yaml`求唯一
canonical readiness；packet不把live maintainer授权值当成静态证据。

## Execution Graph and Ownership

```text
Human implementation gate
  -> SP127-T1 (read-only/scratch evidence)
  -> SP127-T2 (private normalization module + split unit tests)
  -> SP127-T3 (public integration test)
  -> SP127-T4 (read-only verification/review)
```

- writer tasks 不并行；每个时刻每个 target path 只有一个 owner。
- #126 merge `50f6a203c1861814d288d4bdeae0e28d877af34c` 固定 `wrap.rs` 与其
  integration test 行为；GH-127 不接管。
- T2→T3 如需 production handback，必须先停止 T3 writer、废弃当前 head evidence，再由
  T2 单独修改；修正后重新执行 T2、T3 全部 gates。
- 不预提交红测，不创建 future-owner test 依赖，不用脚本批量改写 semantic fixtures。

## Invariant Coverage Audit

| Task | Covers |
| --- | --- |
| SP127-T1 | B-001, B-002, B-003, B-018, B-019, B-020, B-022 |
| SP127-T2 | B-001..B-012, B-015, B-016, B-017, B-018, B-021 |
| SP127-T3 | B-004..B-021 |
| SP127-T4 | B-001..B-022 |

- Product invariant set：`B-001` 至 `B-022`，共 22。
- Tech Product-to-Test Mapping set：`B-001` 至 `B-022`，共 22。
- Tasks `Covers:` union：`B-001` 至 `B-022`，共 22。
- Critical ledger：`GH127-L1` 至 `GH127-L13`，共 13；T2 owns L1-L5，T3 owns
  L6-L13，T4只审计。

## 验证

- exact base、human spec approval、readiness 与 #126/#128/#129/#130 ancestry fresh。
- actual implementation diff 是 manifest 五路径非空子集；no-write paths diff为空；
  三个 required new file 存在，`src/layout/text_flow/tests.rs <= 800` 行；额外
  `Cargo.toml` fixture 必须被 allowlist gate 拒绝。
- GH127-L1..L13 每项 test selector 均 `matched=1/passed=1/ignored=0`，ignored fixture
  必须被拒绝。
- 2k/4k/8k debug/release counts 同时满足 absolute bound、doubling slope 和完整 failure
      diagnostics；无 wall-clock gate。
- public complete-flow/cache/error/cancellation/retry fixtures与4096 property green。
- #126/#128/#129/#130 regressions green且断言未修改。
- fmt/check/clippy/all-target/all-feature tests、branch-aware coverage、fixed-revision external
      SpecRail mirror、CI、independent review、reviewThreads 全绑定同一 exact head；coverage
  前后 fresh PR base/head、local equality、merge-base、LCOV 1..EOF line bounds、
  record/summary/worktree/hash 与 shell early-failure negatives 全部 fail closed。

## Handoff Notes

- 当前 PR 只交付 specs；不得实现、设置/修改 readiness label、resolve PR #109 thread、
  approve、merge 或关闭 issue。
- spec PR body 使用 `Refs #127`，不得用 `Fixes #127`；implementation完成前 issue保持 open。
- `StyleBoundaryNormalized` 的 ordered duplicates、caller 原始 range vector cache identity 和
  validation-vs-interruption precedence 都是 compatibility contract，不是可自由清理的
  implementation detail。
- operation counter 只测 post-validation normalization；review 还必须 source-scan，确认
  `G×R` 没有移到未计数 helper。
- #126 merge 已是 hard ancestor；任何后续 current API/path/ledger 变化都使旧
  implementation evidence失效，需要 retarget、重跑并必要时更新 specs。
