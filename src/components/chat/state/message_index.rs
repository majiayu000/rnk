//! Private message-position index derived from stable transcript order.

use super::super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::components::chat) struct MessageIndex {
    positions: BTreeMap<MessageId, usize>,
}

impl MessageIndex {
    pub(super) const fn empty() -> Self {
        Self {
            positions: BTreeMap::new(),
        }
    }

    pub(super) fn rebuild(messages: &[ChatMessage]) -> Result<Self, ()> {
        let mut positions = BTreeMap::new();
        for (position, message) in messages.iter().enumerate() {
            if positions.insert(message.id, position).is_some() {
                return Err(());
            }
        }
        Ok(Self { positions })
    }

    pub(super) fn position(&self, messages: &[ChatMessage], id: MessageId) -> Option<usize> {
        let position = *self.positions.get(&id)?;
        let message = match messages.get(position) {
            Some(message) => message,
            None => panic!("internal message index points outside the transcript"),
        };
        assert_eq!(
            message.id, id,
            "internal message index points at the wrong message"
        );
        Some(position)
    }

    pub(in crate::components::chat) fn inconsistent_id(
        &self,
        messages: &[ChatMessage],
    ) -> Option<MessageId> {
        for (position, message) in messages.iter().enumerate() {
            if self.positions.get(&message.id) != Some(&position) {
                return Some(message.id);
            }
        }
        self.positions.iter().find_map(|(id, position)| {
            messages
                .get(*position)
                .is_none_or(|message| message.id != *id)
                .then_some(*id)
        })
    }
}
