use super::super::{BlockId, ChatMessage, ChatRole, MessageBlock, MessageBlockEntry, MessageId};
use super::message_index::MessageIndex;

macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::compact::test_cases::$name();
        }
    };
}

case!(thinking_replacement_requires_same_identity);
case!(thinking_id_message_lifetime_rules_are_exhaustive);
case!(message_transition_matrix_is_exhaustive);
case!(nested_status_transition_matrices_are_exhaustive);
case!(terminal_updates_are_single_effect_and_race_safe);
case!(cross_level_terminality_never_freezes_active_nested_blocks);
case!(identity_and_correlation_helpers_cover_all_namespaces);
case!(append_block_cross_level_rules_are_exhaustive);
case!(replace_block_kind_rules_are_exhaustive);
case!(static_completion_readiness_matrix_is_exhaustive);
case!(tool_call_result_correlation_matrix_is_exhaustive);
case!(message_revision_checked_increment_is_exhaustive);
case!(block_id_state_lifetime_rules_are_exhaustive);
case!(restore_history_validation_is_exhaustive);
case!(tool_result_slot_history_rules_are_exhaustive);
case!(revision_exhaustion_is_checked_and_atomic_at_u64_max);

fn indexed_message(id: u64) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        ChatRole::User,
        vec![MessageBlockEntry::new(
            BlockId::new(id),
            MessageBlock::Text("message".into()),
        )],
    )
    .unwrap()
}

#[test]
fn message_index_rejects_duplicate_ids() {
    let message = indexed_message(1);
    assert_eq!(MessageIndex::rebuild(&[message.clone(), message]), Err(()),);
}

#[test]
#[should_panic(expected = "internal message index points outside the transcript")]
fn message_index_fails_loudly_for_out_of_bounds_position() {
    let index = MessageIndex::rebuild(&[indexed_message(1)]).unwrap();
    index.position(&[], MessageId::new(1));
}

#[test]
fn message_index_reports_wrong_message_at_recorded_position() {
    let index = MessageIndex::rebuild(&[indexed_message(1)]).unwrap();
    assert_eq!(
        index.inconsistent_id(&[indexed_message(2)], &mut || {}),
        Some(MessageId::new(2)),
    );
}

#[test]
fn message_index_validation_reports_out_of_bounds_position() {
    let index = MessageIndex::rebuild(&[indexed_message(1)]).unwrap();
    assert_eq!(
        index.inconsistent_id(&[], &mut || {}),
        Some(MessageId::new(1)),
    );
}

#[test]
fn message_index_validation_reports_extra_wrong_identity() {
    let messages = [indexed_message(1)];
    let mut index = MessageIndex::rebuild(&messages).unwrap();
    index.insert_position_for_test(MessageId::new(2), 0);
    assert_eq!(
        index.inconsistent_id(&messages, &mut || {}),
        Some(MessageId::new(2)),
    );
}

#[test]
fn message_index_reports_every_rebuild_and_validation_visit() {
    let messages = [indexed_message(1), indexed_message(2)];
    let mut rebuild_visits = 0;
    let index = MessageIndex::rebuild_with(&messages, &mut || rebuild_visits += 1).unwrap();
    assert_eq!(rebuild_visits, 2);
    let mut validation_visits = 0;
    assert_eq!(
        index.inconsistent_id(&messages, &mut || validation_visits += 1),
        None,
    );
    assert_eq!(validation_visits, 4);
}
