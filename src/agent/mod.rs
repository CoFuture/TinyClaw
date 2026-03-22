//! Agent module - AI model client and tool execution

pub mod client;
pub mod context;
pub mod context_manager;
pub mod error_recovery;
pub mod retry;
pub mod runtime;
pub mod scheduled_task;
pub mod scheduler;
pub mod session_notes;
pub mod skill;
pub mod skill_manager;
pub mod skill_registry;
pub mod suggestion;
pub mod suggestion_manager;
pub mod task;
pub mod task_manager;
pub mod tools;
pub mod turn_log;

pub use client::Agent;
pub use scheduled_task::ScheduledTaskSummary;
#[allow(unused_imports)]
pub use scheduled_task::ScheduleType;
pub use scheduler::Scheduler;
pub use session_notes::{SessionNoteUpdate, SessionNotesManager};
#[allow(unused_imports)]
pub use session_notes::{SessionNote, SessionNoteSummary};
pub use skill::Skill;
pub use skill_registry::SkillRegistry;
pub use skill_manager::SessionSkillManager;
#[allow(unused_imports)]
pub use suggestion::{Suggestion, SuggestionEngine, SuggestionSummary, SuggestionType};
#[allow(unused_imports)]
pub use suggestion_manager::{SuggestionManager, SuggestionFeedback, TrackedSuggestion, TrackedSuggestionSummary};
pub use task::{TaskState, TaskSummary};
pub use task_manager::TaskManager;
#[allow(unused_imports)]
pub use turn_log::{TurnLog, TurnLogEntry, TurnLogSummary};
