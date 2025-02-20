use super::{EventLog, EventStats};
use uuid::Uuid;

#[derive(Debug)]
pub struct EventSummary {
    stats: EventStats,
    total_events: usize,
}

impl EventSummary {
    pub fn new(log: &EventLog) -> Self {
        let records = log.all();
        let mut stats = EventStats::new();

        for record in &records {
            stats.process_event(&record.event);
        }

        Self {
            stats,
            total_events: records.len(),
        }
    }

    pub fn to_string(&self) -> String {
        let mut summary = String::new();

        // Overall statistics
        summary.push_str(&format!("Total Events: {}\n", self.total_events));

        // Task statistics
        summary.push_str("\nTask Statistics:\n");
        summary.push_str("---------------\n");
        summary.push_str(&format!(
            "Tasks Submitted: {}\n",
            self.stats.tasks_submitted
        ));
        summary.push_str(&format!(
            "Tasks Completed: {}\n",
            self.stats.tasks_completed
        ));
        summary.push_str(&format!("Tasks Failed: {}\n", self.stats.tasks_failed));

        // Plugin statistics
        summary.push_str("\nPlugin Statistics:\n");
        summary.push_str("-----------------\n");
        summary.push_str(&format!(
            "Plugins Invoked: {}\n",
            self.stats.plugins_invoked
        ));
        summary.push_str(&format!(
            "Plugins Completed: {}\n",
            self.stats.plugins_completed
        ));
        summary.push_str(&format!("Plugins Failed: {}\n", self.stats.plugins_failed));

        // Agent statistics
        summary.push_str("\nAgent Statistics:\n");
        summary.push_str("----------------\n");
        summary.push_str(&format!("Agents Spawned: {}\n", self.stats.agents_spawned));
        summary.push_str(&format!(
            "Agents Completed: {}\n",
            self.stats.agents_completed
        ));
        summary.push_str(&format!("Agents Failed: {}\n", self.stats.agents_failed));

        // Detailed status summaries
        self.append_status_summary(&mut summary, "Task", &self.stats.task_statuses);
        self.append_status_summary(&mut summary, "Plugin", &self.stats.plugin_statuses);
        self.append_status_summary(&mut summary, "Agent", &self.stats.agent_statuses);

        summary
    }

    fn append_status_summary(
        &self,
        summary: &mut String,
        entity_type: &str,
        statuses: &std::collections::HashMap<Uuid, String>,
    ) {
        if !statuses.is_empty() {
            summary.push_str(&format!("\n{} Status Summary:\n", entity_type));
            summary.push_str(&"-".repeat(entity_type.len() + 17));
            summary.push('\n');
            for (id, status) in statuses {
                summary.push_str(&format!("{} {}: {}\n", entity_type, id, status));
            }
        }
    }
}

impl EventLog {
    pub fn replay_summary(&self) -> String {
        if self.all().is_empty() {
            return "No events to replay.".to_string();
        }
        EventSummary::new(self).to_string()
    }
}
