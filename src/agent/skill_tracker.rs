//! Skill Tracker Module
//!
//! Tracks skill activations and effectiveness over time.
//! Records which skills are active during each turn, correlates with outcomes,
//! and provides insights for improving skill recommendations.
//!
//! The tracker:
//! - Records skill activations per turn/session
//! - Correlates skill usage with turn success and session quality
//! - Tracks skill effectiveness metrics
//! - Provides skill performance insights for recommendations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

/// Statistics for a single skill's effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    /// Skill name
    pub skill_name: String,
    /// How many times this skill was active
    pub activation_count: usize,
    /// How many turns with this skill succeeded
    pub success_count: usize,
    /// How many turns with this skill failed
    pub failure_count: usize,
    /// Success rate when skill is active (0.0 - 1.0)
    pub success_rate: f32,
    /// How many turns were accomplished when this skill was active
    pub accomplishment_count: usize,
    /// Average session quality score when skill is active
    pub avg_quality_score: f32,
    /// Skills that often co-occur with this skill
    pub often_co_occurs_with: Vec<(String, usize)>,
    /// Last activation timestamp
    pub last_seen: Option<DateTime<Utc>>,
    /// First activation timestamp
    pub first_seen: Option<DateTime<Utc>>,
}

impl SkillStats {
    /// Create new skill stats
    pub fn new(skill_name: String) -> Self {
        Self {
            skill_name,
            activation_count: 0,
            success_count: 0,
            failure_count: 0,
            success_rate: 0.0,
            accomplishment_count: 0,
            avg_quality_score: 0.0,
            often_co_occurs_with: Vec::new(),
            last_seen: None,
            first_seen: None,
        }
    }

    /// Record a skill activation with outcome
    pub fn record_activation(&mut self, success: bool, _quality_score: Option<f32>, co_occurrences: &[String]) {
        self.activation_count += 1;
        let now = Utc::now();
        
        if self.first_seen.is_none() {
            self.first_seen = Some(now);
        }
        self.last_seen = Some(now);

        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        
        self.success_rate = if self.activation_count > 0 {
            self.success_count as f32 / self.activation_count as f32
        } else {
            0.0
        };

        // Update co-occurrence counts
        for co_skill in co_occurrences {
            if *co_skill != self.skill_name {
                let existing = self.often_co_occurs_with.iter_mut()
                    .find(|(name, _)| *name == *co_skill);
                if let Some((_, count)) = existing {
                    *count += 1;
                } else {
                    self.often_co_occurs_with.push((co_skill.clone(), 1));
                }
            }
        }
        
        // Sort by count descending and keep top 10
        self.often_co_occurs_with.sort_by(|a, b| b.1.cmp(&a.1));
        self.often_co_occurs_with.truncate(10);
    }

    /// Record an accomplishment with this skill active
    pub fn record_accomplishment(&mut self) {
        self.accomplishment_count += 1;
    }

    /// Update average quality score
    pub fn update_quality_score(&mut self, score: f32) {
        if self.activation_count <= 1 {
            self.avg_quality_score = score;
        } else {
            // Running average
            let total = self.avg_quality_score * (self.activation_count as f32 - 1.0) + score;
            self.avg_quality_score = total / self.activation_count as f32;
        }
    }
}

/// A record of skill activations for a single turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSkillActivation {
    /// Turn ID
    pub turn_id: String,
    /// Session ID
    pub session_id: String,
    /// Skills that were active during this turn
    pub active_skills: Vec<String>,
    /// Whether the turn succeeded
    pub success: bool,
    /// Whether any accomplishment was recorded
    pub had_accomplishment: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Skill performance insight for recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInsight {
    /// Skill name
    pub skill_name: String,
    /// Insight type
    pub insight_type: SkillInsightType,
    /// Description of the insight
    pub description: String,
    /// Effectiveness score (0.0 - 1.0)
    pub effectiveness: f32,
    /// Recommendation action
    pub recommendation: String,
}

/// Types of skill insights
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillInsightType {
    /// This skill is highly effective
    HighlyEffective,
    /// This skill has low effectiveness
    LowEffectiveness,
    /// This skill often succeeds with other skills
    GoodSynergy,
    /// This skill has no data yet
    InsufficientData,
    /// This skill's effectiveness is declining
    DecliningEffectiveness,
}

/// Skill effectiveness report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffectivenessReport {
    /// All skill statistics
    pub skill_stats: Vec<SkillStats>,
    /// Generated insights
    pub insights: Vec<SkillInsight>,
    /// Most effective skill
    pub most_effective_skill: Option<String>,
    /// Least effective skill
    pub least_effective_skill: Option<String>,
    /// Skills with good synergy
    pub synergistic_pairs: Vec<(String, String)>,
    /// Total turns tracked
    pub total_turns_tracked: usize,
    /// Timestamp of report generation
    pub generated_at: DateTime<Utc>,
}

/// Skill Tracker - tracks skill activations and effectiveness
pub struct SkillTracker {
    /// Per-skill statistics
    skill_stats: RwLock<HashMap<String, SkillStats>>,
    /// Turn-level activation records
    turn_activations: RwLock<VecDeque<TurnSkillActivation>>,
    /// Persist path
    persist_path: RwLock<Option<String>>,
    /// Max turn records to keep
    max_turn_records: usize,
}

impl SkillTracker {
    /// Create a new skill tracker
    pub fn new() -> Self {
        Self {
            skill_stats: RwLock::new(HashMap::new()),
            turn_activations: RwLock::new(VecDeque::new()),
            persist_path: RwLock::new(None),
            max_turn_records: 1000,
        }
    }

    /// Create with persistence enabled
    #[allow(dead_code)]
    pub fn with_persistence(persist_path: &str) -> Arc<Self> {
        let tracker = Self {
            skill_stats: RwLock::new(HashMap::new()),
            turn_activations: RwLock::new(VecDeque::new()),
            persist_path: RwLock::new(Some(persist_path.to_string())),
            max_turn_records: 1000,
        };
        let arc = Arc::new(tracker);
        
        // Try to load existing data
        if let Err(e) = arc.load() {
            tracing::warn!("Failed to load skill tracker data: {}", e);
        }
        
        info!("Skill tracker initialized with persistence: {}", persist_path);
        arc
    }

    /// Record skill activations for a turn
    pub fn record_turn_skills(&self, turn_id: &str, session_id: &str, active_skills: &[String], success: bool, had_accomplishment: bool) {
        let timestamp = Utc::now();
        let active_skills_vec: Vec<String> = active_skills.to_vec();
        
        // Record per-skill statistics
        {
            let mut stats = self.skill_stats.write();
            for skill_name in &active_skills_vec {
                let skill_stats = stats.entry(skill_name.clone()).or_insert_with(|| SkillStats::new(skill_name.clone()));
                
                // Get co-occurring skills (other than this one)
                let co_occurrences: Vec<String> = active_skills_vec.iter()
                    .filter(|s| *s != skill_name)
                    .cloned()
                    .collect();
                
                skill_stats.record_activation(success, None, &co_occurrences);
                
                if had_accomplishment {
                    skill_stats.record_accomplishment();
                }
            }
        }

        // Record turn-level activation
        {
            let mut activations = self.turn_activations.write();
            activations.push_back(TurnSkillActivation {
                turn_id: turn_id.to_string(),
                session_id: session_id.to_string(),
                active_skills: active_skills_vec.clone(),
                success,
                had_accomplishment,
                timestamp,
            });

            // Trim if too large
            while activations.len() > self.max_turn_records {
                activations.pop_front();
            }
        }

        debug!("Recorded skill activations for turn {}: {:?}", turn_id, active_skills_vec);
        self.persist();
    }

    /// Update quality scores for skills based on session quality
    #[allow(dead_code)]
    pub fn update_quality_for_skills(&self, session_id: &str, quality_score: f32) {
        let mut stats = self.skill_stats.write();
        let activations = self.turn_activations.read();
        
        // Find all skills that were active in this session
        let mut session_skills: HashSet<String> = HashSet::new();
        for activation in activations.iter().rev() {
            if activation.session_id == session_id {
                for skill in &activation.active_skills {
                    session_skills.insert(skill.clone());
                }
            }
        }

        // Update quality scores
        for skill_name in session_skills {
            if let Some(skill_stats) = stats.get_mut(&skill_name) {
                skill_stats.update_quality_score(quality_score);
            }
        }
    }

    /// Get skill statistics for a specific skill
    pub fn get_skill_stats(&self, skill_name: &str) -> Option<SkillStats> {
        self.skill_stats.read().get(skill_name).cloned()
    }

    /// Get all skill statistics
    #[allow(dead_code)]
    pub fn get_all_stats(&self) -> Vec<SkillStats> {
        self.skill_stats.read().values().cloned().collect()
    }

    /// Get skill effectiveness report
    pub fn generate_effectiveness_report(&self) -> SkillEffectivenessReport {
        let stats = self.skill_stats.read();
        let all_stats: Vec<SkillStats> = stats.values().cloned().collect();
        
        // Find most/least effective
        let mut sorted_by_success: Vec<&SkillStats> = all_stats.iter()
            .filter(|s| s.activation_count >= 3) // Min 3 activations for meaningful ranking
            .collect();
        sorted_by_success.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap_or(std::cmp::Ordering::Equal));

        let most_effective = sorted_by_success.first().map(|s| s.skill_name.clone());
        let least_effective = sorted_by_success.last().map(|s| s.skill_name.clone());

        // Find synergistic pairs (high co-occurrence with high success)
        let mut synergistic_pairs = Vec::new();
        for skill in &all_stats {
            if skill.activation_count >= 3 && skill.success_rate >= 0.7 {
                for (co_skill, count) in &skill.often_co_occurs_with {
                    if *count >= 2 {
                        synergistic_pairs.push((skill.skill_name.clone(), co_skill.clone()));
                    }
                }
            }
        }
        synergistic_pairs.sort();
        synergistic_pairs.dedup();

        // Generate insights
        let insights = self.generate_insights(&all_stats);

        drop(stats);

        SkillEffectivenessReport {
            skill_stats: all_stats,
            insights,
            most_effective_skill: most_effective,
            least_effective_skill: least_effective,
            synergistic_pairs,
            total_turns_tracked: self.turn_activations.read().len(),
            generated_at: Utc::now(),
        }
    }

    /// Generate insights from skill statistics
    fn generate_insights(&self, all_stats: &[SkillStats]) -> Vec<SkillInsight> {
        let mut insights = Vec::new();

        for skill in all_stats {
            if skill.activation_count == 0 {
                continue;
            }

            let insight_type;
            let description;
            let effectiveness;
            let recommendation;

            if skill.activation_count < 3 {
                insight_type = SkillInsightType::InsufficientData;
                description = format!("Skill '{}' has only been activated {} times, not enough data for meaningful insights.", skill.skill_name, skill.activation_count);
                effectiveness = 0.0;
                recommendation = format!("Continue using '{}' to gather more data.", skill.skill_name);
            } else if skill.success_rate >= 0.8 {
                insight_type = SkillInsightType::HighlyEffective;
                description = format!("Skill '{}' has a {:.0}% success rate over {} activations.", skill.skill_name, skill.success_rate * 100.0, skill.activation_count);
                effectiveness = skill.success_rate;
                recommendation = format!("Highly effective! Consider using '{}' more often for similar tasks.", skill.skill_name);
            } else if skill.success_rate < 0.4 {
                insight_type = SkillInsightType::LowEffectiveness;
                description = format!("Skill '{}' has a low {:.0}% success rate. Consider if this skill is being applied appropriately.", skill.skill_name, skill.success_rate * 100.0);
                effectiveness = skill.success_rate;
                recommendation = format!("Review the use cases for '{}'. It may not be the right skill for the current task types.", skill.skill_name);
            } else {
                // Check for synergy
                if !skill.often_co_occurs_with.is_empty() && skill.success_rate >= 0.6 {
                    insight_type = SkillInsightType::GoodSynergy;
                    let co_skills: Vec<String> = skill.often_co_occurs_with.iter().take(3).map(|(s, _)| s.clone()).collect();
                    description = format!("Skill '{}' often works well with {:?}. Success rate: {:.0}%", skill.skill_name, co_skills, skill.success_rate * 100.0);
                    effectiveness = skill.success_rate;
                    recommendation = format!("Try using '{}' together with {} for better results.", skill.skill_name, co_skills.join(", "));
                } else {
                    // Neutral - not declining but not highly effective either
                    continue;
                }
            }

            insights.push(SkillInsight {
                skill_name: skill.skill_name.clone(),
                insight_type,
                description,
                effectiveness,
                recommendation,
            });
        }

        // Sort by effectiveness descending (highly effective first)
        insights.sort_by(|a, b| b.effectiveness.partial_cmp(&a.effectiveness).unwrap_or(std::cmp::Ordering::Equal));
        insights.truncate(10); // Top 10 insights

        insights
    }

    /// Get skill effectiveness for recommendation weighting
    #[allow(dead_code)]
    pub fn get_effectiveness_for_skill(&self, skill_name: &str) -> f32 {
        self.skill_stats.read()
            .get(skill_name)
            .map(|s| if s.activation_count >= 3 { s.success_rate } else { 0.5 }) // Default 0.5 for new skills
            .unwrap_or(0.5)
    }

    /// Persist to disk
    fn persist(&self) {
        let path = self.persist_path.read().clone();
        if let Some(path) = path {
            if let Err(e) = self.save_to_file(&path) {
                tracing::warn!("Failed to persist skill tracker: {}", e);
            }
        }
    }

    /// Save to JSON file
    fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct PersistData {
            skill_stats: HashMap<String, SkillStats>,
            turn_activations: Vec<TurnSkillActivation>,
        }

        let data = PersistData {
            skill_stats: self.skill_stats.read().clone(),
            turn_activations: self.turn_activations.read().clone().into_iter().collect(),
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load from JSON file
    #[allow(dead_code)]
    fn load(&self) -> std::io::Result<()> {
        let path = self.persist_path.read().clone();
        let Some(path) = path else {
            return Ok(());
        };

        let path = Path::new(&path);
        if !path.exists() {
            return Ok(());
        }

        #[derive(Serialize, Deserialize)]
        struct PersistData {
            skill_stats: HashMap<String, SkillStats>,
            turn_activations: Vec<TurnSkillActivation>,
        }

        let json = std::fs::read_to_string(path)?;
        let data: PersistData = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        *self.skill_stats.write() = data.skill_stats;
        
        let mut activations = self.turn_activations.write();
        for activation in data.turn_activations {
            activations.push_back(activation);
        }
        while activations.len() > self.max_turn_records {
            activations.pop_front();
        }

        info!("Loaded skill tracker data");
        Ok(())
    }

    /// Clear all tracking data
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.skill_stats.write().clear();
        self.turn_activations.write().clear();
        self.persist();
    }

    /// Get summary statistics
    #[allow(dead_code)]
    pub fn get_summary(&self) -> SkillTrackerSummary {
        let stats = self.skill_stats.read();
        let total_activations: usize = stats.values().map(|s| s.activation_count).sum();
        let total_turns = self.turn_activations.read().len();
        
        SkillTrackerSummary {
            total_skills_tracked: stats.len(),
            total_activations,
            total_turns_tracked: total_turns,
        }
    }
}

/// Summary statistics for skill tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrackerSummary {
    pub total_skills_tracked: usize,
    pub total_activations: usize,
    pub total_turns_tracked: usize,
}

impl Default for SkillTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tracker() -> Arc<SkillTracker> {
        Arc::new(SkillTracker::new())
    }

    #[test]
    fn test_record_single_activation() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, true);
        
        let stats = tracker.get_skill_stats("file_ops");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.activation_count, 1);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.accomplishment_count, 1);
    }

    #[test]
    fn test_record_multiple_skills() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string(), "code_analysis".to_string()], true, false);
        
        let file_ops = tracker.get_skill_stats("file_ops").unwrap();
        let code_analysis = tracker.get_skill_stats("code_analysis").unwrap();
        
        assert_eq!(file_ops.activation_count, 1);
        assert_eq!(code_analysis.activation_count, 1);
        
        // Check co-occurrence
        assert!(file_ops.often_co_occurs_with.iter().any(|(s, _)| s == "code_analysis"));
        assert!(code_analysis.often_co_occurs_with.iter().any(|(s, _)| s == "file_ops"));
    }

    #[test]
    fn test_success_rate_calculation() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, false);
        tracker.record_turn_skills("turn2", "session1", &["file_ops".to_string()], true, false);
        tracker.record_turn_skills("turn3", "session1", &["file_ops".to_string()], false, false);
        
        let stats = tracker.get_skill_stats("file_ops").unwrap();
        assert_eq!(stats.activation_count, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert!((stats.success_rate - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_accomplishment_tracking() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, true);
        tracker.record_turn_skills("turn2", "session1", &["file_ops".to_string()], true, false);
        
        let stats = tracker.get_skill_stats("file_ops").unwrap();
        assert_eq!(stats.accomplishment_count, 1);
    }

    #[test]
    fn test_effectiveness_report() {
        let tracker = create_test_tracker();
        
        // Record enough data for insights
        for i in 0..5 {
            tracker.record_turn_skills(&format!("turn{}", i), "session1", &["file_ops".to_string()], i < 4, i == 2);
        }
        
        let report = tracker.generate_effectiveness_report();
        assert!(!report.skill_stats.is_empty());
        assert!(report.generated_at <= chrono::Utc::now());
    }

    #[test]
    fn test_get_effectiveness_for_new_skill() {
        let tracker = create_test_tracker();
        
        // New skill with no data should return 0.5 (neutral)
        let effectiveness = tracker.get_effectiveness_for_skill("nonexistent");
        assert!((effectiveness - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_get_effectiveness_for_tracked_skill() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, false);
        tracker.record_turn_skills("turn2", "session1", &["file_ops".to_string()], true, false);
        tracker.record_turn_skills("turn3", "session1", &["file_ops".to_string()], false, false);
        
        let effectiveness = tracker.get_effectiveness_for_skill("file_ops");
        assert!((effectiveness - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_skill_with_insufficient_data() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, false);
        
        let report = tracker.generate_effectiveness_report();
        // With only 1 activation, should be insufficient data
        assert!(!report.insights.is_empty());
        let insight = &report.insights[0];
        assert_eq!(insight.insight_type, SkillInsightType::InsufficientData);
    }

    #[test]
    fn test_highly_effective_skill() {
        let tracker = create_test_tracker();
        
        // Record 3+ successes
        for _ in 0..5 {
            tracker.record_turn_skills("turn", "session1", &["file_ops".to_string()], true, false);
        }
        
        let report = tracker.generate_effectiveness_report();
        let insight = report.insights.iter().find(|i| i.skill_name == "file_ops");
        assert!(insight.is_some());
        let insight = insight.unwrap();
        assert_eq!(insight.insight_type, SkillInsightType::HighlyEffective);
    }

    #[test]
    fn test_low_effectiveness_skill() {
        let tracker = create_test_tracker();
        
        // Record 3+ failures
        for _ in 0..5 {
            tracker.record_turn_skills("turn", "session1", &["file_ops".to_string()], false, false);
        }
        
        let report = tracker.generate_effectiveness_report();
        let insight = report.insights.iter().find(|i| i.skill_name == "file_ops");
        assert!(insight.is_some());
        let insight = insight.unwrap();
        assert_eq!(insight.insight_type, SkillInsightType::LowEffectiveness);
    }

    #[test]
    fn test_synergy_detection() {
        let tracker = create_test_tracker();
        
        // Record multiple co-occurrences with success
        for _ in 0..5 {
            tracker.record_turn_skills("turn", "session1", &["file_ops".to_string(), "code_analysis".to_string()], true, false);
        }
        
        let report = tracker.generate_effectiveness_report();
        // Should detect synergy between file_ops and code_analysis
        assert!(!report.synergistic_pairs.is_empty() || !report.insights.is_empty());
    }

    #[test]
    fn test_summary() {
        let tracker = create_test_tracker();
        
        tracker.record_turn_skills("turn1", "session1", &["file_ops".to_string()], true, false);
        tracker.record_turn_skills("turn2", "session1", &["code_analysis".to_string()], true, false);
        
        let summary = tracker.get_summary();
        assert_eq!(summary.total_skills_tracked, 2);
        assert_eq!(summary.total_activations, 2);
        assert_eq!(summary.total_turns_tracked, 2);
    }

    #[test]
    fn test_empty_tracker() {
        let tracker = create_test_tracker();
        
        let report = tracker.generate_effectiveness_report();
        assert!(report.skill_stats.is_empty());
        assert!(report.insights.is_empty());
        assert!(report.most_effective_skill.is_none());
    }
}
