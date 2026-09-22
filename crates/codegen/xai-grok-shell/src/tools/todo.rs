//! Types are canonical in `xai-grok-tools`.
//! This module adds conversions between ACP plan entries and `TodoItem` since `xai-grok-tools` is protocol-agnostic.

pub use xai_grok_tools::implementations::grok_build::todo::TodoId;
pub use xai_grok_tools::implementations::grok_build::todo::TodoItem;
pub use xai_grok_tools::implementations::grok_build::todo::TodoPriority;
pub use xai_grok_tools::implementations::grok_build::todo::TodoState;
pub use xai_grok_tools::implementations::grok_build::todo::TodoStatus;

use agent_client_protocol as acp;

/// ACP has no `Cancelled` status, so cancelled items are stored as `Completed` with `{"cancelled": true}` in meta.
pub fn todo_item_from_plan_entry(entry: acp::PlanEntry) -> TodoItem {
    let status = match entry.status {
        acp::PlanEntryStatus::Pending => TodoStatus::Pending,
        acp::PlanEntryStatus::InProgress => TodoStatus::InProgress,
        acp::PlanEntryStatus::Completed => {
            if entry
                .meta
                .as_ref()
                .and_then(|m| m.get("cancelled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                TodoStatus::Cancelled
            } else {
                TodoStatus::Completed
            }
        }
        // TODO(acp-0.10): `PlanEntryStatus` is #[non_exhaustive].
        _ => TodoStatus::Pending,
    };
    TodoItem {
        content: entry.content,
        priority: match entry.priority {
            acp::PlanEntryPriority::High => TodoPriority::High,
            acp::PlanEntryPriority::Medium => TodoPriority::Medium,
            acp::PlanEntryPriority::Low => TodoPriority::Low,
            // TODO(acp-0.10): `PlanEntryPriority` is #[non_exhaustive].
            _ => TodoPriority::Medium,
        },
        status,
        meta: entry.meta.map(serde_json::Value::Object),
    }
}

/// Cancelled items become `Completed` with `{"cancelled": true}` in meta.
pub(crate) fn plan_entry_from_todo_item(item: TodoItem) -> acp::PlanEntry {
    let status = match item.status {
        TodoStatus::Pending => acp::PlanEntryStatus::Pending,
        TodoStatus::InProgress => acp::PlanEntryStatus::InProgress,
        TodoStatus::Completed => acp::PlanEntryStatus::Completed,
        TodoStatus::Cancelled => acp::PlanEntryStatus::Completed,
    };
    let mut meta = item.meta;
    if item.status == TodoStatus::Cancelled {
        let mut m = meta.unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = m.as_object_mut() {
            obj.insert("cancelled".into(), true.into());
        }
        meta = Some(m);
    }
    acp::PlanEntry::new(
        item.content,
        match item.priority {
            TodoPriority::High => acp::PlanEntryPriority::High,
            TodoPriority::Medium => acp::PlanEntryPriority::Medium,
            TodoPriority::Low => acp::PlanEntryPriority::Low,
        },
        status,
    )
    .meta(meta.and_then(|v| v.as_object().cloned()))
}

/// ACP `Plan` projection of the live todo resource.
///
/// Compaction republishes this after `AutoCompactCompleted`. The pane is fed
/// only by `Plan` events, and that completion event must not be the last word
/// or a client that clears on it ends the replay with an empty list.
pub(crate) fn plan_from_todo_state(state: &TodoState) -> acp::Plan {
    acp::Plan::new(
        state
            .todo_items()
            .cloned()
            .map(plan_entry_from_todo_item)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_from_todo_state_keeps_actionable_items() {
        let mut state = TodoState::default();
        state.push(
            "1".into(),
            TodoItem {
                content: "wire up auth".into(),
                priority: TodoPriority::High,
                status: TodoStatus::InProgress,
                meta: None,
            },
        );
        state.push(
            "2".into(),
            TodoItem {
                content: "add tests".into(),
                priority: TodoPriority::Medium,
                status: TodoStatus::Pending,
                meta: None,
            },
        );
        let plan = plan_from_todo_state(&state);
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.entries[0].content, "wire up auth");
        assert_eq!(plan.entries[0].status, acp::PlanEntryStatus::InProgress);
        assert_eq!(plan.entries[1].content, "add tests");
        assert_eq!(plan.entries[1].status, acp::PlanEntryStatus::Pending);
    }
}
