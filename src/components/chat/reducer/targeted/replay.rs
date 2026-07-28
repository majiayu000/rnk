use super::*;

pub(super) fn replay_matches(left: &ConversationEvent, right: &ConversationEvent) -> bool {
    left.event_id == right.event_id
        && left.sequence == right.sequence
        && updates_equal(&left.update, &right.update)
}

fn updates_equal(left: &ConversationUpdate, right: &ConversationUpdate) -> bool {
    match left {
        ConversationUpdate::Push(left) => match right {
            ConversationUpdate::Push(right) => {
                left.guard == right.guard && messages_equal(&left.message, &right.message)
            }
            _ => false,
        },
        ConversationUpdate::AppendText(left) => match right {
            ConversationUpdate::AppendText(right) => left == right,
            _ => false,
        },
        ConversationUpdate::AppendMessageBlock(left) => match right {
            ConversationUpdate::AppendMessageBlock(right) => {
                left.guard == right.guard && entries_equal_one(&left.entry, &right.entry)
            }
            _ => false,
        },
        ConversationUpdate::InsertMessageBlock(left) => match right {
            ConversationUpdate::InsertMessageBlock(right) => {
                left.guard == right.guard
                    && left.position == right.position
                    && entries_equal_one(&left.entry, &right.entry)
            }
            _ => false,
        },
        ConversationUpdate::ReplaceBlock(left) => match right {
            ConversationUpdate::ReplaceBlock(right) => {
                left.guard == right.guard
                    && left.block_id == right.block_id
                    && blocks_equal(&left.replacement, &right.replacement)
            }
            _ => false,
        },
        ConversationUpdate::Complete(left) => match right {
            ConversationUpdate::Complete(right) => left == right,
            _ => false,
        },
        ConversationUpdate::Cancel(left) => match right {
            ConversationUpdate::Cancel(right) => left == right,
            _ => false,
        },
        ConversationUpdate::Fail(left) => match right {
            ConversationUpdate::Fail(right) => left == right,
            _ => false,
        },
        ConversationUpdate::EditMessage(left) => match right {
            ConversationUpdate::EditMessage(right) => {
                left.guard == right.guard && entries_equal(&left.entries, &right.entries)
            }
            _ => false,
        },
        ConversationUpdate::DeleteMessage(left) => match right {
            ConversationUpdate::DeleteMessage(right) => left == right,
            _ => false,
        },
        ConversationUpdate::Resend(left) => match right {
            ConversationUpdate::Resend(right) => {
                left.source_guard == right.source_guard
                    && messages_equal(&left.message, &right.message)
            }
            _ => false,
        },
    }
}

fn messages_equal(left: &ChatMessage, right: &ChatMessage) -> bool {
    left.id == right.id
        && left.role == right.role
        && left.status == right.status
        && left.revision == right.revision
        && entries_equal(&left.blocks, &right.blocks)
        && left.metadata() == right.metadata()
}

fn entries_equal(left: &[MessageBlockEntry], right: &[MessageBlockEntry]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| entries_equal_one(left, right))
}

fn entries_equal_one(left: &MessageBlockEntry, right: &MessageBlockEntry) -> bool {
    record_block_visits(2);
    left == right
}

fn blocks_equal(left: &MessageBlock, right: &MessageBlock) -> bool {
    record_block_visits(2);
    left == right
}
