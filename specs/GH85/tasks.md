# Task Plan：聊天布局 benchmark baseline 与回归门

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Dependency: [`../GH61/product.md`](../GH61/product.md)、
  [`../GH61/tech.md`](../GH61/tech.md)、[`../GH61/tasks.md`](../GH61/tasks.md)

## 实现前置门

maintainer 必须对当前 exact packet/head 明确确认可以实施；readiness label 仅描述队列状态。
#61 implementation 必须已合入并记录 exact merged SHA；在生产测量 seam 合入前不得启动以下
implementation tasks，也不得把 spec 中的拟议 API 当成已存在实现。

## 实现任务

- [ ] `SP85-T7A` 在GH-61合入后只读发现真实measurement seam与prerequisite候选。Owner:
      `gh61-dependency-discovery-lane` | Dependencies: maintainer对当前exact packet/head的明确
      implementation授权、#61 exact merged implementation SHA | Done when: read-only evidence记录
      real `SnapshotBuildReport`/`SnapshotWorkCounters`、full/incremental/recovered path/symbol、
      closed counter set，以及parity/work-counter exact argv；search merged GH61是否已有明确证明
      allocation operation归属、计数与reset语义的exact test，只输出`found(real argv)`或
      `missing(require GH85 fallback)`，不得创建或定稿dependency manifest，也不得引用尚不存在的
      fallback test | Verify:
      `git merge-base --is-ancestor "$GH61_MERGED_SHA" HEAD`；
      `test "$(git show -s --format=%H "$GH61_MERGED_SHA")" = "$GH61_MERGED_SHA"`；
      `rg -n 'SnapshotBuildReport|SnapshotWorkCounters|visited_nodes|mutated_nodes|text_flow_recomputes|snapshot_nodes|rebuild_count' src tests`。
  - File ownership: 全仓只读；只交付discovery evidence，不写manifest、T1 contract file、
    production、bench、checker、Cargo或workflow。
  - Covers: B-004。
  - Handoff: 向T1交付GH61 merged SHA、resolved anchors、counter/strategy closed sets、两条
    real prerequisite argv与allocation discovery result后停止；T1据此决定是否创建fallback。

- [ ] `SP85-T1` 建立 benchmark contract 的可重复 red fixtures。Owner:
      `benchmark-root-cause-lane` | Dependencies: SP85-T7A discovery handoff、fresh duplicate
      evidence与maintainer implementation授权 | Done when: fixtures 在实现前证明现有 generic layout benches
      缺少固定 chat matrix/versioned artifact/trusted-baseline lifecycle，并列出所有
      positive/negative case 名称；dependency wiring、closed/unknown-key schema、roles/hashes、
      closed build provenance、prerequisite category/spec_ref/command/result、allocation
      fallback-before-benchmark、pending-status/phase-zero/sandbox/validator/final-reporter handoff、
      same-repo/fork head+test-merge dedicated-App status source、container/permission/archive negatives、
      status result matrix、
      external human authorization、full-tree topology/canonical-only raw diff、five-route/status separation、
      event exact refs、exact-checkout ABBA/pair mismatch、zero denominators、reviewed full-SHA action
      allowlist与three-stage authority/promotion handoff
      都有命名 red fixture；不得修改 test assertion 基础设施 | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked -- --list`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_requires_complete_prerequisite_category_set -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_missing_duplicate_unknown_categories_and_spec_refs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_correctness_fallback_runs_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`。
  - File ownership: 独占 `tests/layout_snapshot_benchmark_contract.rs`；不写 production、
    bench、Cargo、workflow、checker 或 baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 提交red checkpoint与完整fixture清单后停止写该文件；若T7A报告allocation test
    missing，本checkpoint必须已创建命名fallback test；随后T7B才能定稿manifest。

- [ ] `SP85-T7B` 在T1 checkpoint后定稿fail-closed dependency manifest。Owner:
      `gh61-dependency-finalization-lane` | Dependencies: SP85-T7A evidence、SP85-T1 stopped；若
      allocation test缺失，T1 fallback exact test已实际存在 | Done when:
      `tests/fixtures/gh85_gh61_dependency.json`记录real anchors/strategies/counters，三个
      prerequisite commands严格按parity/work-counter/allocation-correctness各一次；第三项只能取
      T7A发现的real test或T1已存在的fallback，所有path/argv在merged SHA与current HEAD唯一解析，
      禁止placeholder/未来test名 | Verify:
      `python3 -m json.tool tests/fixtures/gh85_gh61_dependency.json >/dev/null`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`。
  - File ownership: 独占`tests/fixtures/gh85_gh61_dependency.json`；T1 contract file与其他路径只读。
  - Covers: B-004。
  - Handoff: manifest commit后停止；T2A只读消费manifest，不得静默改写。

- [ ] `SP85-T2A` 建立可先合入的trust-root foundation。Owner: `benchmark-trust-root-foundation-lane` |
      Dependencies: SP85-T1与SP85-T7B handoff；两者writer均停止；#61 merged counter seam已按exact
      SHA重新定位 | Done when: checker/schema/config/corpus/toolchain/contract tests与base-owned
      `.github/workflows/layout-benchmark-authority.yml`形成闭合checkpoint；PR jobs只监听规定的
      `pull_request_target`事件，并按`status_pending_request -> phase_zero -> sandbox_collect ->
      trusted_validate -> status_reporter_request`建立fresh-VM依赖；per-PR concurrency取消旧run且所有job
      有timeout。全部PR jobs有explicit event guard，两个requester权限仅`id-token:write`，repo workflow无
      statuses/checks write且无reporter key/token secret；它们向T3 protected external service发送closed
      OIDC bundle。service才fresh解析head/test-merge pair并用dedicated App双写
      active `gh85/layout-benchmark/v{reporter_epoch}`；bundle/status绑定epoch与service config digest，
      classifier不读reviews。
      sandbox host权限为空，unauthenticated fetch exact public SHA；PR Cargo/build/binary只在tech固定
      digest的networkless/read-only/no-capability ephemeral containers执行，base/head/leg输出隔离，host在
      container结束后hash并由host `raw_upload`输出artifact name/id/digest/run/attempt/PR/head binding；
      container不能写Actions outputs。trusted validate以actions read消费controller outputs、通过REST下载raw ZIP bytes，archive
      preflight通过后才stream-extract/closed-parse且只执行base checker。四种trust-root
      action均使用tech列出的reviewed full SHA，升级只能走contract-update route；authority job实现
      subject -> pinned attest action -> finalize -> immutable upload顺序；authority仅workflow_dispatch，
      PR jobs不在dispatch运行，promotion要求entire authority workflow run success。Cargo注册的
      `chat_layout` entrypoint在本foundation仅显式返回implementation-unavailable blocked状态，不测量、
      不产生candidate且不能被解释为green；T2B将从trusted base独占替换为真实runner。matrix
      严格等于 tech 表；row 聚合与 counters 完整；closed schema 拒绝 unknown/duplicate
      keys；每个 role 都有 closed build provenance；role/source/config/corpus/content/binary
      hash、三个按 closed category/spec_ref 顺序记录的 prerequisite results 与 ABBA
      trace/pair identity 完整；allocation fallback 证明 counter 的 operation 归属、计数与
      reset 语义且先于任何 benchmark；
      `median_ns`由exact 10 observations的checked even median产生，deterministic counters逐sample
      相同；checker从exact base tree取canonical并实现topology-based five-route classifier，promotion
      raw diff只允许canonical；`git ls-files -z`每个current tracked path精确命中一个class；route、
      external human authorization、performance status独立；只有`comparison_passed`表示无回归；bootstrap
      显式绑定repo/base/head/run/target/artifact，
      promotion validation只读authority evidence与committed blob；zero denominators、containment、
      ancestry/ref/runner/trust/promotion负例均fail closed | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked closed_schema_rejects_unknown_duplicate_and_partial_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked artifact_hashes_cover_roles_sources_config_corpus_trace_and_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked all_roles_require_closed_build_provenance -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked candidate_canonical_and_current_run_roles_are_not_interchangeable -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ten_sample_even_median_and_deterministic_counters_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked per_operation_counters_sum_checked_and_abba_samples_keep_leg_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_invalid_prerequisite_command_arrays -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_requires_complete_prerequisite_category_set -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_missing_duplicate_unknown_categories_and_spec_refs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_correctness_fallback_runs_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_paths_and_argv_are_contained -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_compare_require_base_ancestor_and_exact_merge_base -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_regression_fails_on_any_paired_batch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked canonical_refs_are_historical_and_current_refs_are_invocation_scoped -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked abba_requires_identical_current_runner_observation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked pinned_toolchain_target_profile_and_runner_class_are_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_uses_base_owned_checker_and_rejects_untrusted_head_policy -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_rejects_mixed_spec_symlink_mode_and_ambiguous_diffs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_same_run_handoff_is_exact_and_replay_safe -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_jobs_have_exact_permissions_and_isolated_fresh_vms -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_collect_isolates_pr_code_in_pinned_networkless_containers -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_rejects_mutable_image_network_privilege_host_mount_and_output_reuse -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_validate_preflights_raw_zip_bytes_before_extraction -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked raw_zip_preflight_rejects_bombs_links_traversal_duplicates_crc_and_trailing_data -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dedicated_reporter_oidc_rejects_repo_token_pr_head_workflow_and_spoofed_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_service_verifies_oidc_workflow_repository_and_binding_claims -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_registration_check_binds_required_status_to_verified_app_integration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked registration_check_is_non_required_and_cannot_satisfy_benchmark_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_epoch_one_requires_audited_absence_without_revocation_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked lifecycle_manifests_enforce_genesis_absence_or_rotation_causal_chain -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_rejects_prior_rule_context_integration_and_rotation_rejects_epoch_one -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_intent_is_signed_non_reusable_and_precedes_registration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_registration_binds_intent_not_future_revocation_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked revocation_receipt_finalizes_after_registration_revoke_and_failed_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_priming_manifest_verifies_intent_registration_receipt_causal_chain -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_registration_binds_absence_receipt_and_rejects_rotation_intent -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_priming_pending_follows_absence_activation_and_registration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_priming_rejects_revocation_fields_and_non_strict_anchor_timestamps -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_priming_rejects_pending_not_strictly_after_revocation_and_failed_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_keeps_same_app_integration_and_versions_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_primes_all_open_pr_head_and_test_merge_pairs_before_switch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_and_config_digest_bind_oidc_status_and_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_revocation_requires_failed_old_credential_canary_and_audit_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_rotation_revokes_old_credential_before_final_epoch_priming -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked post_revocation_pending_is_latest_new_credential_status_and_overwrites_old_success -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_compromise_provisions_new_app_epoch_and_revokes_old_credentials -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked required_statuses_bind_current_head_and_test_merge_for_same_repo_and_fork -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked commit_status_response_and_combined_status_schema_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked raw_upload_controller_outputs_bind_artifact_to_head_and_run -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked newest_head_concurrency_replay_and_timeout_are_fail_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked route_selection_is_mutually_exclusive_and_only_comparison_passed_is_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked safe_docs_route_is_not_applicable_and_mixed_runtime_routes_are_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked repository_path_classes_cover_legitimate_topology_and_block_unknowns -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked all_current_tracked_paths_match_exactly_one_class -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked canonical_promotion_diff_is_exactly_one_path -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked restricted_routes_require_external_authorization_without_review_interpretation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked benchmark_status_never_grants_merge_authorization -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked review_rules_and_contributing_authorization_are_external_prerequisites -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked external_authorization_and_performance_status_are_independent -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authorized_contract_update_is_non_green_and_requires_rebaseline_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_requires_explicit_repo_refs_and_exact_merge_base -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rejects_committed_blob_not_matching_authority -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_workflow_permissions_and_attestation_identity_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_root_actions_are_pinned_to_reviewed_full_shas -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_pipeline_requires_action_bundle_outputs_and_finalizes_after_attest -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_artifact_handoff_rejects_missing_expired_wrong_run_id_digest_or_bundle -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked combined_workflow_event_guards_and_authority_whole_run_success_are_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`。
  - File ownership: 接管 `tests/layout_snapshot_benchmark_contract.rs`；独占
    `.github/scripts/check_gh61_benchmark.py`、`.github/workflows/layout-benchmark-authority.yml`、
    `benches/chat_layout.rs`的blocked foundation entrypoint、
    `benches/support/chat_layout.rs`、`tests/fixtures/gh61_benchmark_schema.json`、
    `Cargo.toml`、`Cargo.lock`。只读 T7 dependency manifest 与 GH-61 production seam；
    不写 canonical baseline或`.github/workflows/ci.yml`。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 提交并冻结exact checker/workflow/schema/config/corpus/toolchain/API checkpoint后停止；
    T3合入并配置exact required commit status前T2B不得开始。T2B不得修改任何T2A contract path。

- [ ] `SP85-T3` provision专用reporter service、合入trust root并配置external gates。Owner:
      `benchmark-trust-root-integration-lane`；External owner: `gh85-status-reporter-service-owner` |
      Dependencies: SP85-T2A signed checkpoint；T2A writer已停止；maintainer已明确授权foundation PR、
      dedicated App install、external service与ruleset/review rule变更 | Done when: external owner按下述
      genesis/rotation lifecycle provision
      `gh85-benchmark-status-reporter`，只安装base repo且permissions精确metadata/PR read、statuses write、
      checks write；Checks API与App credential只由external service持有，任何Actions job均不可取得；
      key/token只在server-side secret manager/HSM，repo/org/environment Actions secrets均无副本。记录并
      保护exact endpoint、固定OIDC audience `gh85-benchmark-status-reporter`、App/installation id、
      allowed workflow path/ref/SHA、positive monotonic reporter epoch、active status/registration contexts、
      service config
      digest、key version、rotation/revocation procedure、audit retention/alerting；不新增或假设service repo
      source path。external owner进入merge-locked maintenance后先闭合二选一lifecycle。genesis只允许epoch
      1，并通过GitHub App/installation、server key/config、ruleset与known context audit证明无prior reporter
      App/installation/key/context/rule/integration，记录closed `genesis_absence_receipt`；receipt验证后才
      provision/验证initial App/installation/key/service config，且不运行revoke/canary。
      后续key/install/config/trust rotation要求prior active evidence及next epoch >1，先provision/验证new
      credential/service config，old credential此时可暂存；registration前生成signed、closed、non-reusable
      `rotation_intent`，含唯一nonce、old/new App/installation/key refs、new epoch/context/config digest、timestamp
      与signature，且禁止任何未来registration/revocation/canary/status字段。service用new credential调用
      `POST /repos/{base_owner}/{base_repo}/check-runs`，于current protected
      default-branch SHA创建唯一completed/success、non-required
      `gh85/reporter-registration/v{reporter_epoch}` check run；genesis closed external_id绑定absence receipt digest与
      new App identity，rotation closed external_id绑定`rotation_intent` digest与new App identity，绝不绑定未来
      `revocation_receipt`；
      `GET /repos/{base_owner}/{base_repo}/check-runs/{check_run_id}` fresh验证check id/name/head/status/
      conclusion/external_id，并证明returned `app.id`/`app.slug`与service-held installation对应App一致后记录
      immutable response digest。registration check不得required，也不得满足benchmark。rotation registration通过后、
      final vN priming前先撤销old credential/installation并取得provider evidence，再运行authenticated canary；只有
      明确authentication failure后才finalize signed、closed `revocation_receipt`，其中含intent digest、registration
      check id/response digest、revoke timestamp/provider digest、failed-canary result/digest/timestamp以及new
      credential/config/epoch。必须验证
      `intent.created_at < registration.created_at < registration.verified_at < revoked_at < failed_canary.observed_at < receipt.finalized_at`；
      timeout、仍成功、ambiguous failure、提前receipt或link/digest/timestamp mismatch均blocked；genesis明确跳过
      intent/revocation/canary且不得伪造。registration manifest是closed union：genesis只含absence receipt，rotation
      只含intent与registration evidence且不得含未来receipt。priming lifecycle manifest是另一closed union：genesis
      链接absence receipt+registration，rotation链接intent+registration+final receipt并验证nonce唯一与因果顺序；
      wrong variant、neither/both或forbidden fields均blocked。

      shared priming是closed tagged union。genesis variant绑定`genesis_absence_receipt` digest/verified_at、
      initial credential activated_at与registration created_at/verified_at，pending必须有new status id且created_at严格晚于
      全部anchors且禁止revocation fields/proof。rotation variant绑定`revocation_receipt` digest、revoked_at与
      failed_canary_at以及intent/registration/receipt chain digest，pending必须有new status id且created_at严格晚于两者，并禁止genesis fields。两者精确XOR并共同绑定
      new credential/config/epoch；wrong variant、forbidden field、neither/both、equal/earlier timestamp均blocked。
      selected route anchor验证后service才fresh完整分页枚举每个open PR，验证current head/test-merge ordered pair，
      只用new credential向每个pair两端写或覆盖
      `gh85/layout-benchmark/v{reporter_epoch}=pending`并记录closed sorted priming manifest/page/pair digest、
      selected lifecycle anchor/chain、new credential/config identity及status ids/timestamps；逐SHA fresh验证
      latest same-context status是new-credential pending、created_at严格晚于route anchor且target binding含new
      config/epoch/lifecycle chain digest。rotation中old credential预写的任何vN success必须被覆盖，failed canary证明
      撤销后不能竞态；genesis不得携带这些revocation fields/proof；
      rule switch前再次分页与re-query，集合或pair变化就丢弃并重做。全部current pair已pending后，才原子
      切换service active epoch/config与ruleset required tuple到
      `(gh85/layout-benchmark/v{reporter_epoch},current App integration_id)`，smoke通过后解除merge lock。
      routine rotation保持同一App integration id；old context仅因不再required/context mismatch不能满足，
      old status保持同一App source且不得改写；revoke-before-final-priming不改变App integration id。只有
      confirmed App credential compromise才额外provision新App ID/integration与new epoch，并同样在final
      priming前撤销old App credential、验证failed canary+receipt，再完成new-App priming/switch。service
      unavailable/revoked/audit gap或任一步失败即blocked，无repo token、unversioned
      context、workflow-check或双required-context fallback。

      reviewed T2A checkpoint合入
      protected default ref；GitHub branch protection/ruleset将exact commit status context
      `gh85/layout-benchmark/v{reporter_epoch}`设为required，并绑定dedicated App exact integration_id；workflow
      job check明确不required。fresh API/read-only UI evidence证明context/source、default-ref workflow SHA
      与ruleset准确。T3还在maintainer confirmation下实际配置required approving review、new commit时
      dismiss stale approvals，并验证CONTRIBUTING final merge authorization绑定exact head；这些是planned
      prerequisite，不假设当前已存在。`.github/workflows/ci.yml`及八job
      `ci-gate`保持byte-for-byte不变且独立required；不消费任何跨workflow benchmark artifact。
      foundation workflow以same-repo与fork真实PR smoke run证明pending/final status都附着到current exact
      head与verified current test-merge，combined status schema/context/run/binding与dedicated integration正确；
      若任一SHA status被GitHub拒绝或无valid test merge则block implementation。
      smoke还证明sandbox container无network/token/host/Docker socket访问、base/head输出隔离、raw ZIP在
      extraction前fail-closed preflight，stale head/wrong source/concurrent supersession/cancel/replay/
      mismatch/timeout均不得写success；
      foundation blocked runner不能输出candidate或performance pass | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_same_run_handoff_is_exact_and_replay_safe -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_uses_base_owned_checker_and_rejects_untrusted_head_policy -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dedicated_reporter_oidc_rejects_repo_token_pr_head_workflow_and_spoofed_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_service_verifies_oidc_workflow_repository_and_binding_claims -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_registration_check_binds_required_status_to_verified_app_integration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked registration_check_is_non_required_and_cannot_satisfy_benchmark_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_epoch_one_requires_audited_absence_without_revocation_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked lifecycle_manifests_enforce_genesis_absence_or_rotation_causal_chain -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_rejects_prior_rule_context_integration_and_rotation_rejects_epoch_one -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_intent_is_signed_non_reusable_and_precedes_registration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_registration_binds_intent_not_future_revocation_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked revocation_receipt_finalizes_after_registration_revoke_and_failed_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_priming_manifest_verifies_intent_registration_receipt_causal_chain -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_registration_binds_absence_receipt_and_rejects_rotation_intent -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_priming_pending_follows_absence_activation_and_registration -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked genesis_priming_rejects_revocation_fields_and_non_strict_anchor_timestamps -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_priming_rejects_pending_not_strictly_after_revocation_and_failed_canary -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_keeps_same_app_integration_and_versions_context -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_primes_all_open_pr_head_and_test_merge_pairs_before_switch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_and_config_digest_bind_oidc_status_and_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked rotation_revocation_requires_failed_old_credential_canary_and_audit_receipt -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_rotation_revokes_old_credential_before_final_epoch_priming -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked post_revocation_pending_is_latest_new_credential_status_and_overwrites_old_success -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked reporter_compromise_provisions_new_app_epoch_and_revokes_old_credentials -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked required_statuses_bind_current_head_and_test_merge_for_same_repo_and_fork -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked commit_status_response_and_combined_status_schema_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked raw_upload_controller_outputs_bind_artifact_to_head_and_run -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_jobs_have_exact_permissions_and_isolated_fresh_vms -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_collect_isolates_pr_code_in_pinned_networkless_containers -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_rejects_mutable_image_network_privilege_host_mount_and_output_reuse -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_validate_preflights_raw_zip_bytes_before_extraction -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked raw_zip_preflight_rejects_bombs_links_traversal_duplicates_crc_and_trailing_data -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked review_rules_and_contributing_authorization_are_external_prerequisites -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked combined_workflow_event_guards_and_authority_whole_run_success_are_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked newest_head_concurrency_replay_and_timeout_are_fail_closed -- --exact`；
      `git diff --exit-code "$FOUNDATION_BASE" "$FOUNDATION_HEAD" -- .github/workflows/ci.yml`；
      maintainer以`GET /repos/{base_owner}/{base_repo}/rulesets/{ruleset_id}`验证
      `required_status_checks`精确包含
      `(context=gh85/layout-benchmark/v{reporter_epoch},integration_id=verified App)`且不含
      `gh85/reporter-registration/v{reporter_epoch}`，并核对registration response、完整open-PR priming manifest
      与non-required性质；initial provisioning记录live `genesis_absence_receipt`及absence query digests；
      external owner分别执行same-App routine rotation与compromise new-App dry run，证明
      genesis pending晚于absence/activation/registration anchors且无revocation proof；same-App integration id
      稳定、rotation old credential在final priming前已由failed canary+receipt证明撤销、latest pending覆盖
      任何old vN success且status/config属于new credential、old context不再
      required，并运行same-repo/fork audit query。
  - File ownership: repo全只读；仅maintainer执行foundation merge/ruleset/review-rule配置，external
    service owner独占repo外App/service/keys/audit configuration；不新增service source path，不修改baseline。
  - Covers: B-004, B-008, B-009。
  - Handoff: 记录trusted default-ref SHA、workflow/checker digest、service config/key version、OIDC/App/
    installation id、registration check id/response digest/reporter epoch、selected
    `genesis_absence_receipt`或`rotation_intent`/registration/`revocation_receipt`/canary chain digest、
    open-PR route-specific priming manifest/status ids/timestamps/anchor digest、
    active versioned contexts、head+test-merge smoke及
    status/review/ruleset evidence后
    停止；T2B必须从该exact SHA新开implementation head，不能复用foundation worktree。

- [ ] `SP85-T2B` 从trusted base实现真实benchmark runner tranche。Owner:
      `benchmark-runtime-runner-lane` | Dependencies: SP85-T3 handoff；exact trusted base含required
      workflow/checker/schema/config/corpus/toolchain checkpoint | Done when: 从fresh exact trusted base只在
      approved implementation path `benches/chat_layout.rs`实现真实measurement runner，调用T2A冻结support API，
      输出六scenario/strategy、per-operation counters、10-sample/ABBA identity与allocation/timing rows；
      foundation implementation-unavailable状态被真实runner替换。不得修改checker、workflow、schema、
      config/corpus owner、tests、Cargo或dependency manifest；任何contract缺陷退回T2A新授权contract
      route，不在implementation PR内顺手修改 | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ten_sample_even_median_and_deterministic_counters_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked per_operation_counters_sum_checked_and_abba_samples_keep_leg_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_regression_fails_on_any_paired_batch -- --exact`；
      diff guard证明runtime diff精确只改`benches/chat_layout.rs`。
  - File ownership: 从T2A stopped checkpoint串行接管`benches/chat_layout.rs`，并独占spec已先声明的
    该单一implementation path；所有contract paths只读。
  - Covers: B-001, B-002, B-003, B-005, B-006, B-008。
  - Handoff: 交付exact implementation head、workflow run与required commit-status evidence；停止全部写入后T4开始。

- [ ] `SP85-T4` 完成 implementation exact-head verification 与人工 handoff。Owner:
      `benchmark-verification-lane` | Dependencies: SP85-T2B、SP85-T3完成且所有writers停止 |
      Done when: 每个 exact contract test 证明 matched=1/passed=1/ignored=0；full Rust gates、
      candidate schema、implementation diff guard、current CI、independent review、
      resolved review threads 与 maintainer 对当前 exact head 的 merge authorization 绑定同一
      head；只申请 implementation merge authorization，不申请 baseline promotion authorization |
      Verify: 重跑SP85-T2A/T2B/T3适用于implementation head的
      全部命令及本文件“验证”章节的 fresh full commands。
  - File ownership: 全仓只读；发现 production/test/workflow 缺陷时退回对应 owner 新 checkpoint，
    不跨 ownership 偷改。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: implementation合入后记录exact merged SHA；未合入不得启动T5A。implementation
    authorization只覆盖该PR，不覆盖authority run或promotion PR。

- [ ] `SP85-T5A` 在trusted default ref生成并attest authority。Owner:
      `benchmark-authority-run-lane` | Dependencies: SP85-T4 implementation或authorized
      contract-update PR已由maintainer明确授权并合入；exact merged source SHA、route与authorization
      evidence已记录；default-ref上的
      `.github/workflows/layout-benchmark-authority.yml`是reviewed trusted version | Done when:
      maintainer从protected default ref触发`workflow_dispatch`；workflow在exact merged source的
      detached checkout以同一fixed-digest sandbox image/policy fresh测量，source-controlled Cargo/
      binary不在token-bearing host执行；`generate-authority-subject`只产生subject+unsigned metadata，
      step id `attest`使用`actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4`，
      `finalize-authority`随后消费exact action
      `bundle-path`/`attestation-id`并输出final envelope；authority job permissions精确为contents
      read/id-token write/attestations write/artifact-metadata write，其他none。
      `actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4`
      以run/attempt/source唯一name、`overwrite:false`上传subject/metadata/bundle/envelope，并记录
      artifact id/digest/run；所有PR jobs因event guard不在dispatch运行，entire workflow run必须
      `event=workflow_dispatch`且`conclusion=success`；不读取PR candidate/checker、不写repo | Verify:
      `python3 .github/scripts/check_gh61_benchmark.py --mode generate-authority-subject --repo "$AUTHORITY_REPO" --repository-id "$GITHUB_REPOSITORY_ID" --workflow-ref "$GITHUB_WORKFLOW_REF" --default-ref-sha "$DEFAULT_REF_SHA" --source-sha "$AUTHORITY_SOURCE_SHA" --run-id "$AUTHORITY_RUN_ID" --run-attempt "$AUTHORITY_RUN_ATTEMPT" --target-root "$RUNNER_TEMP/gh85-authority-target" --artifact-dir "$RUNNER_TEMP/gh85-authority" --subject-out "$RUNNER_TEMP/gh85-authority/canonical.json" --unsigned-metadata-out "$RUNNER_TEMP/gh85-authority/unsigned-metadata.json"`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode finalize-authority --subject "$RUNNER_TEMP/gh85-authority/canonical.json" --unsigned-metadata "$RUNNER_TEMP/gh85-authority/unsigned-metadata.json" --attestation-bundle "$ATTEST_BUNDLE_PATH" --attestation-id "$ATTESTATION_ID" --repository-id "$GITHUB_REPOSITORY_ID" --workflow-ref "$GITHUB_WORKFLOW_REF" --default-ref-sha "$DEFAULT_REF_SHA" --source-sha "$AUTHORITY_SOURCE_SHA" --run-id "$AUTHORITY_RUN_ID" --run-attempt "$AUTHORITY_RUN_ATTEMPT" --authority-out "$RUNNER_TEMP/gh85-authority/authority.json"`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_pipeline_requires_action_bundle_outputs_and_finalizes_after_attest -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_artifact_handoff_rejects_missing_expired_wrong_run_id_digest_or_bundle -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_root_actions_are_pinned_to_reviewed_full_shas -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked combined_workflow_event_guards_and_authority_whole_run_success_are_closed -- --exact`；
      inspect exact permissions、step order/ids、`${{ steps.attest.outputs.bundle-path }}`、
      `${{ steps.attest.outputs.attestation-id }}`与`authority_upload` artifact id/digest outputs。
  - File ownership: repo全只读；只允许写repo外authority artifact。不得push、开PR或写canonical。
  - Covers: B-003, B-007, B-008, B-009。
  - Handoff: 向T5B交付repository、artifact id/digest/name、run id/attempt、source/default-ref SHA、
    attestation id与subject digest；T5B必须从GitHub API重新取得，不得重新生成或转换。

- [ ] `SP85-T5B` 在独立PR promotion authority-owned canonical bytes。Owner:
      `baseline-promotion-lane` | Dependencies: SP85-T5A authority完成；promotion PR exact head已知，
      branch protection已单独要求approving review/dismiss stale approvals，且maintainer final merge
      authorization将按CONTRIBUTING绑定exact head；automated classifier不读取review | Done when: promotion PR提交authority
      canonical bytes；base-owned phase zero判定`canonical_only_promotion`；required CI使用
      artifact id查询/download、`gh attestation verify`与trusted checker只读验证artifact
      name/id/digest/run/expiry、subject/repo/workflow/ref/source/event，未调用
      generator、未创建/覆盖canonical；成功tuple精确为
      `promotion_valid/external_required/not_available`，commit status可success但不授权merge或声称无回归 |
      Verify:
      `git cat-file -e "${PROMOTION_BASE}^{commit}"`；
      `git cat-file -e "${PROMOTION_HEAD}^{commit}"`；
      `git merge-base --is-ancestor "$PROMOTION_BASE" "$PROMOTION_HEAD"`；
      `test "$(git merge-base "$PROMOTION_BASE" "$PROMOTION_HEAD")" = "$PROMOTION_BASE"`；
      `gh api -H "Accept: application/vnd.github+json" "/repos/$EXPECTED_REPOSITORY/actions/artifacts/$AUTHORITY_ARTIFACT_ID" >"$RUNNER_TEMP/authority-artifact.json"`并以`jq`验证name/id/digest/expired=false/workflow_run.id；
      `gh api -H "Accept: application/vnd.github+json" "/repos/$EXPECTED_REPOSITORY/actions/runs/$AUTHORITY_RUN_ID" >"$RUNNER_TEMP/authority-run.json"`并以`jq`验证event=`workflow_dispatch`、conclusion=`success`、head SHA/ref/path/attempt；
      `gh api -H "Accept: application/vnd.github+json" "/repos/$EXPECTED_REPOSITORY/actions/artifacts/$AUTHORITY_ARTIFACT_ID/zip" >"$RUNNER_TEMP/authority.zip"`；
      `gh attestation verify "$CANONICAL_FILE" -R "$EXPECTED_REPOSITORY" --signer-workflow "$EXPECTED_REPOSITORY/.github/workflows/layout-benchmark-authority.yml" --signer-digest "$AUTHORITY_DEFAULT_REF_SHA" --source-ref "refs/heads/$DEFAULT_BRANCH" --source-digest "$AUTHORITY_DEFAULT_REF_SHA" --deny-self-hosted-runners --format json`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode validate-promotion --repo "$GITHUB_WORKSPACE" --pr-base-oid "$PROMOTION_BASE" --head-sha "$PROMOTION_HEAD" --run-id "$GITHUB_RUN_ID" --run-attempt "$GITHUB_RUN_ATTEMPT" --authority-envelope "$RUNNER_TEMP/authority/authority.json" --attestation-bundle "$RUNNER_TEMP/authority/attestation.json" --authority-artifact-id "$AUTHORITY_ARTIFACT_ID" --authority-artifact-digest "$AUTHORITY_ARTIFACT_DIGEST" --authority-run-id "$AUTHORITY_RUN_ID" --committed-canonical .github/benchmarks/gh61-baseline.json`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rejects_committed_blob_not_matching_authority -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked combined_workflow_event_guards_and_authority_whole_run_success_are_closed -- --exact`；
      `git diff --name-only "$PROMOTION_BASE"..."$PROMOTION_HEAD"`精确等于canonical path；validation
      前后`git status --porcelain`为空且blob digest不变；记录current exact-head CI、independent
      review、resolved threads与maintainer对同一promotion head的separate merge authorization。
  - File ownership: 独占 `.github/benchmarks/gh61-baseline.json`；其余全仓只读。
  - Covers: B-003, B-007, B-008, B-009。
  - Handoff: 只有promotion合入default branch后，未来PR才可把baseline当trusted base-tree
    evidence；本task不自动merge，且promotion authorization不得复用于未来contract update。

- [ ] `SP85-T6` 在未来 PR 验证首次 trusted compare。Owner:
      `benchmark-post-promotion-verification-lane` | Dependencies: SP85-T5B promotion 已人工授权
      并合入，且该 commit 位于当前 PR base tree | Done when: fresh fixture 验证 base-tree
      ancestry/content hash、stable compatibility/volatile observation、same-runner ABBA、
      timing 2-of-3、allocation any-batch双阈值和zero denominators；证据绑定当前exact
      base/head/run | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked abba_requires_identical_current_runner_observation -- --exact`。
  - File ownership: 全仓只读；只产出 current-run evidence。
  - Covers: B-005, B-006, B-007, B-009。
  - Handoff: compare 仍需普通 PR 的 current CI/review/merge authorization；本验证不授予 merge。

## 并行拆分

- Writable dependency graph：`T7A(read-only) -> T1 -> T7B -> T2A foundation -> T3 trust-root merge
  + required-status configuration/same-repo+fork smoke -> fresh trusted base -> T2B runner -> T4 verification
  -> implementation merge -> T5A(authority)
  -> T5B(promotion PR) -> promotion merge -> T6`；T4必须等T3 trusted workflow/checker已在base。
- 后续authorized contract-update PR走`contract_update_bootstrap -> contract_update_valid`，取得绑定
  exact head的独立maintainer authorization并合入后，同样进入`T5A -> T5B -> promotion merge`；
  `route_status=contract_update_valid`、`authorization_status=external_required`与
  `performance_status=not_available`，required commit status可success但
  不能声称performance passed。
- T7A全只读；T1独占contract test；T7B独占dependency manifest；T2A串行接管contract test并
  独占checker/workflow/Cargo/schema/support contract及blocked bench entrypoint。T3全仓只读完成
  trust-root merge/ruleset后，T2B才从fresh base串行接管bench entrypoint实现real runner；T2B不得
  写任何T2A contract path。两者无并发ownership或共享worktree。
- T4/T5A/T6全仓只读；缺陷必须退回唯一owner。T5B是后续独立PR，唯一可写canonical baseline。
- implementation PR、promotion PR 的 merge 都是分离的人工 gate；任何 lane 不得自行 push、
  approve、resolve review threads 或 merge。

## 验证

- Product invariant 集合与 task `Covers:` union 均精确为 B-001 至 B-009。
- planned JSON中每个path必须有nonempty owner；`.github/workflows/ci.yml`不在planned paths且保持
  unchanged。首次implementation diff排除canonical与contract paths；promotion raw diff未经剥离
  必须精确只有canonical，包含docs/specs也blocked。
- 所有 filtered Rust tests 先 `--list --exact`，再执行并证明 matched=1、passed=1、ignored=0。
- dependency manifest 必须绑定 GH61 merged ancestry、真实 unique anchors、三种 strategy 与
  五个 counter fields；base-owned inline collector不得接收trusted checker artifact，prerequisite
  commands 必须按闭合 category/spec_ref 精确覆盖
  parity、work-counter、allocation-correctness；allocation 必须先 search merged GH61 的
  correctness test，缺失时使用已规划 GH85 contract fallback。sandbox container未按序执行/记录任一
  command、cwd不是exact checkout root、path absolute/traversal/symlink escape、argv不在closed
  Cargo exact-test allowlist、所选allocation correctness command未先于benchmark或wiring test
  未实际消费任一entry时blocked。
- combined workflow同时声明PR-target与workflow_dispatch；每个PR job显式event guard并按
  `status_pending_request -> phase_zero -> sandbox_collect -> trusted_validate -> status_reporter_request`
  隔离执行，authority只允许dispatch，always jobs也必须同时满足event guard，不能mixed execution；
  classifier不读reviews，workflow job check不required。sandbox host权限为空，PR代码只在fixed-digest、
  networkless/read-only/no-capability containers运行且无host/Actions/Docker socket mounts；base/head/leg
  output隔离，host-side raw_upload controller outputs绑定name/id/digest/run/attempt/PR/head。trusted validator
  以actions read取得匹配raw ZIP bytes并在extract前验证archive结构/CRC/quotas/fixed files。两个requester
  只有id-token write；external dedicated App service验证OIDC workflow/repo/run/binding、fresh head/test-merge
  object parents与pair currentness后才双写active `gh85/layout-benchmark/v{reporter_epoch}`；OIDC bundle、
  status与receipt均绑定epoch/config digest。repo Actions无status/check write或App
  secret；same-repo/fork都须smoke通过，missing/duplicate/wrong-source/stale-pair/cancel/replay/timeout均failure。
  既有`ci.yml`/八job`ci-gate`独立不变。
- artifact 的 scenario/strategy/minimum operation matrix、closed/unknown-key schema、
  role/source/closed build/config/corpus/content/binary hashes、prerequisite results、
  historical/current refs separation、stable compatibility/volatile observation separation、
  ABBA trace/pair identity、event exact head/base ancestry与merge-base、10-sample checked even
  median、sample reset、deterministic counter equality、zero-denominator与recovered aggregation
  必须完整；timing按2-of-3，allocation任一paired batch双阈值即失败；
  partial/invalid evidence non-green。
- route在initial、contract-update、canonical-only promotion、normal compare、non-benchmark中
  精确五选一。route/external authorization/performance statuses分离；classifier不读取review，三种
  受限route只输出`external_required`，required approving review/stale dismissal与CONTRIBUTING final
  merge authorization必须由T3实际配置/验证并由外部GitHub gate绑定exact head，benchmark status不能授权merge。只有
  `comparison_passed -> performance_status=passed`表示无回归；四种valid control route的
  performance为not_available，regression/needs_rebaseline/blocked均check failure。
- authority严格按generate-subject -> attest action -> finalize -> upload顺序运行；promotion按exact
  artifact id/digest/name/run/expiry下载，且whole workflow run必须dispatch+success，再只读验证committed
  canonical，前后blob/status不变。
- trust-root action必须精确使用tech allowlist中的四个reviewed full SHA并保留`# v4`注释；mutable
  tag、short SHA或其他digest均失败，升级必须走authorized contract-update route。
- fresh full commands：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
```

- exact-head repository CI、independent review、resolved review threads 与 maintainer explicit
  merge authorization 必须绑定同一implementation、contract-update或promotion head，不能跨PR复用。

## Handoff Notes

- 本packet只描述未来implementation/authority/promotion lanes；其中“不自行push/开PR/merge/
  resolve threads”约束仅适用于这些未来lanes，不限制当前spec PR按maintainer授权更新原分支。
  实时label事实必须从GitHub重新读取，且不构成实施或merge授权。
- checker 名称与 canonical path 沿用拆分来源：
  `.github/scripts/check_gh61_benchmark.py`、
  `.github/benchmarks/gh61-baseline.json`；如要改为 GH85 命名，必须先更新 issue/spec。
- timing使用same-runner ABBA、3 batches、20%+50µs、two-of-three；allocation使用
  10%+8 allocations / 4096 bytes且任一paired batch双阈值即失败。stable compatibility class
  不兼容是`needs_rebaseline`；volatile CPU identity只诊断，但同次ABBA observation必须相同。
- implementation bootstrap 与 canonical promotion 必须是两个 PR 生命周期；promotion 仍需
  independent review、current exact-head repository CI、resolved threads与maintainer merge authorization。
