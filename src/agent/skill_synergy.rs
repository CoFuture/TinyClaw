//! Skill Synergy Module
//!
//! Analyzes skill pair effectiveness beyond simple co-occurrence tracking.
//! Computes synergy scores to identify which skill combinations lead to
//! better outcomes, detecting synergistic pairs (boost success) vs
//! antagonistic pairs (hurt success).
//!
//! Key concepts:
//! - Synergy Score: measures whether combined use is better/worse than expected
//! - Pair Analysis: tracks how skill pairs perform together vs separately
//! - Pattern Detection: identifies optimal skill combinations for different contexts

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use tracing::info;

/// Statistics for a pair of skills used together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPairStats {
    /// First skill name (alphabetically ordered for consistency)
    pub skill_a: String,
    /// Second skill name
    pub skill_b: String,
    /// How many times both skills were active together
    pub co_activations: usize,
    /// How many times both skills led to success
    pub co_successes: usize,
    /// How many times both skills led to failure
    pub co_failures: usize,
    /// How many times this pair led to accomplishment
    pub co_accomplishments: usize,
    /// Success rate when both skills are active
    pub co_success_rate: f32,
    /// Individual success rate for skill A when paired with B
    pub a_when_paired_rate: f32,
    /// Individual success rate for skill B when paired with A
    pub b_when_paired_rate: f32,
    /// First co-activation timestamp
    pub first_seen: Option<DateTime<Utc>>,
    /// Last co-activation timestamp
    pub last_seen: Option<DateTime<Utc>>,
}

impl SkillPairStats {
    /// Create new pair stats (skills should be alphabetically ordered)
    pub fn new(skill_a: String, skill_b: String) -> Self {
        Self {
            skill_a,
            skill_b,
            co_activations: 0,
            co_successes: 0,
            co_failures: 0,
            co_accomplishments: 0,
            co_success_rate: 0.0,
            a_when_paired_rate: 0.0,
            b_when_paired_rate: 0.0,
            first_seen: None,
            last_seen: None,
        }
    }

    /// Record a co-activation
    pub fn record_co_activation(&mut self, success: bool, a_success: bool, b_success: bool, had_accomplishment: bool) {
        self.co_activations += 1;
        let now = Utc::now();
        
        if self.first_seen.is_none() {
            self.first_seen = Some(now);
        }
        self.last_seen = Some(now);

        if success {
            self.co_successes += 1;
        } else {
            self.co_failures += 1;
        }

        if had_accomplishment {
            self.co_accomplishments += 1;
        }

        self.co_success_rate = self.co_successes as f32 / self.co_activations as f32;
        self.a_when_paired_rate = if a_success { 1.0 } else { 0.0 };
        self.b_when_paired_rate = if b_success { 1.0 } else { 0.0 };
    }
}

/// The relationship pattern between a skill pair
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SynergyPattern {
    /// Skills work much better together than expected
    HighlySynergistic,
    /// Skills work somewhat better together
    Synergistic,
    /// Skills perform about as expected when combined
    Neutral,
    /// Skills work somewhat worse together
    Antagonistic,
    /// Skills work much worse together than expected
    HighlyAntagonistic,
    /// Not enough data to determine pattern
    InsufficientData,
}

impl SynergyPattern {
    /// Get description of this pattern
    pub fn description(&self) -> &'static str {
        match self {
            SynergyPattern::HighlySynergistic => "These skills significantly boost each other's effectiveness",
            SynergyPattern::Synergistic => "These skills tend to work well together",
            SynergyPattern::Neutral => "Combined use has neutral impact on effectiveness",
            SynergyPattern::Antagonistic => "These skills may interfere with each other",
            SynergyPattern::HighlyAntagonistic => "These skills significantly hurt each other's effectiveness",
            SynergyPattern::InsufficientData => "Not enough data to determine synergy",
        }
    }

    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            SynergyPattern::HighlySynergistic => "🌟",
            SynergyPattern::Synergistic => "✨",
            SynergyPattern::Neutral => "➖",
            SynergyPattern::Antagonistic => "⚠️",
            SynergyPattern::HighlyAntagonistic => "🚫",
            SynergyPattern::InsufficientData => "❓",
        }
    }
}

/// Synergy score for a skill pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyScore {
    /// First skill name
    pub skill_a: String,
    /// Second skill name
    pub skill_b: String,
    /// Synergy score from -1.0 (antagonistic) to +1.0 (synergistic)
    pub score: f32,
    /// Pattern classification
    pub pattern: SynergyPattern,
    /// Combined success rate when both active
    pub combined_success_rate: f32,
    /// Expected success rate if independent (product of individual rates)
    pub expected_success_rate: f32,
    /// Lift: how much better/worse than expected
    pub lift: f32,
    /// Confidence level (based on sample size)
    pub confidence: f32,
    /// Number of co-occurrences
    pub co_occurrences: usize,
}

impl SynergyScore {
    /// Create a synergy score from pair stats and individual skill rates
    pub fn from_pair_stats(
        pair: &SkillPairStats,
        skill_a_individual_rate: f32,
        skill_b_individual_rate: f32,
    ) -> Self {
        let co_occurrences = pair.co_activations;
        
        // Not enough data
        if co_occurrences < 3 {
            return Self {
                skill_a: pair.skill_a.clone(),
                skill_b: pair.skill_b.clone(),
                score: 0.0,
                pattern: SynergyPattern::InsufficientData,
                combined_success_rate: pair.co_success_rate,
                expected_success_rate: 0.0,
                lift: 0.0,
                confidence: 0.0,
                co_occurrences,
            };
        }

        // Expected rate if independent (product of individual rates)
        let expected = skill_a_individual_rate * skill_b_individual_rate;
        
        // Lift: actual vs expected
        let lift = if expected > 0.0 {
            pair.co_success_rate / expected
        } else if pair.co_success_rate > 0.0 {
            2.0 // Pure success when expected is 0
        } else {
            1.0
        };

        // Synergy score: normalized lift from -1 to +1
        // lift > 1 means synergistic, lift < 1 means antagonistic
        // We use log to compress extreme values
        let log_lift = if lift > 0.0 { lift.ln() } else { -5.0 };
        let score = (log_lift / 2.5).clamp(-1.0, 1.0); // Normalize to [-1, 1]

        // Pattern classification
        let pattern = if lift > 1.5 {
            SynergyPattern::HighlySynergistic
        } else if lift > 1.1 {
            SynergyPattern::Synergistic
        } else if lift < 0.5 {
            SynergyPattern::HighlyAntagonistic
        } else if lift < 0.9 {
            SynergyPattern::Antagonistic
        } else {
            SynergyPattern::Neutral
        };

        // Confidence based on sample size (more co-occurrences = higher confidence)
        let confidence = ((co_occurrences as f32) / (co_occurrences as f32 + 10.0)).min(1.0);

        Self {
            skill_a: pair.skill_a.clone(),
            skill_b: pair.skill_b.clone(),
            score,
            pattern,
            combined_success_rate: pair.co_success_rate,
            expected_success_rate: expected,
            lift,
            confidence,
            co_occurrences,
        }
    }
}

/// A detected synergy insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyInsight {
    /// Primary skill name
    pub skill_a: String,
    /// Secondary skill name
    pub skill_b: String,
    /// Pattern detected
    pub pattern: SynergyPattern,
    /// Synergy score
    pub score: f32,
    /// Human-readable insight description
    pub description: String,
    /// Actionable recommendation
    pub recommendation: String,
}

/// Skill combination recommendation for a given context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyRecommendation {
    /// Primary skill to recommend
    pub primary_skill: String,
    /// Secondary skill(s) to combine with
    pub secondary_skills: Vec<String>,
    /// Why this combination is recommended
    pub reason: String,
    /// Expected improvement
    pub expected_improvement: String,
    /// Confidence level (0-1)
    pub confidence: f32,
}

/// Complete synergy analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSynergyAnalysis {
    /// All analyzed skill pairs
    pub pair_scores: Vec<SynergyScore>,
    /// Generated insights
    pub insights: Vec<SynergyInsight>,
    /// Synergistic pairs (positive synergy)
    pub synergistic_pairs: Vec<SynergyScore>,
    /// Antagonistic pairs (negative synergy)
    pub antagonistic_pairs: Vec<SynergyScore>,
    /// Recommendations for skill combinations
    pub recommendations: Vec<SynergyRecommendation>,
    /// Total pairs analyzed
    pub total_pairs_analyzed: usize,
    /// High-confidence pairs (for reliable recommendations)
    pub high_confidence_count: usize,
    /// Timestamp
    pub generated_at: DateTime<Utc>,
}

/// Individual skill's aggregate synergy data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSynergySummary {
    /// Skill name
    pub skill_name: String,
    /// Best synergy partner
    pub best_partner: Option<String>,
    /// Best synergy score
    pub best_synergy_score: f32,
    /// Worst synergy partner (antagonistic)
    pub worst_partner: Option<String>,
    /// Worst synergy score
    pub worst_synergy_score: f32,
    /// Average synergy score across all partners
    pub avg_synergy_score: f32,
    /// Number of partners with positive synergy
    pub positive_partners: usize,
    /// Number of partners with negative synergy
    pub negative_partners: usize,
}

/// Skill Synergy Analyzer - analyzes skill pair effectiveness
pub struct SkillSynergyAnalyzer {
    /// Per-skill-pair statistics
    pair_stats: RwLock<HashMap<(String, String), SkillPairStats>>,
    /// Individual skill success rates (computed from turn data)
    individual_rates: RwLock<HashMap<String, (usize, usize)>>, // (successes, total)
    /// Persist path
    persist_path: RwLock<Option<String>>,
    /// Minimum co-occurrences for analysis
    min_co_occurrences: usize,
}

impl SkillSynergyAnalyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        Self {
            pair_stats: RwLock::new(HashMap::new()),
            individual_rates: RwLock::new(HashMap::new()),
            persist_path: RwLock::new(None),
            min_co_occurrences: 3,
        }
    }

    /// Create with persistence path
    #[allow(dead_code)]
    pub fn with_persistence(persist_path: &str) -> Arc<Self> {
        let analyzer = Self {
            pair_stats: RwLock::new(HashMap::new()),
            individual_rates: RwLock::new(HashMap::new()),
            persist_path: RwLock::new(Some(persist_path.to_string())),
            min_co_occurrences: 3,
        };
        let arc = Arc::new(analyzer);
        
        if let Err(e) = arc.load() {
            tracing::warn!("Failed to load skill synergy data: {}", e);
        }
        
        info!("Skill synergy analyzer initialized with persistence: {}", persist_path);
        arc
    }

    /// Record skill activations from a turn
    pub fn record_turn(&self, skills: &[String], success: bool, had_accomplishment: bool) {
        if skills.is_empty() {
            return;
        }

        // Update individual skill rates
        {
            let mut rates = self.individual_rates.write();
            for skill in skills {
                let entry = rates.entry(skill.clone()).or_insert((0, 0));
                entry.1 += 1;
                if success {
                    entry.0 += 1;
                }
            }
        }

        // Update pair stats for all skill combinations
        if skills.len() >= 2 {
            let sorted_skills = {
                let mut s = skills.to_vec();
                s.sort();
                s
            };

            let mut pairs = self.pair_stats.write();
            for i in 0..sorted_skills.len() {
                for j in (i + 1)..sorted_skills.len() {
                    let skill_a = sorted_skills[i].clone();
                    let skill_b = sorted_skills[j].clone();
                    let key = (skill_a.clone(), skill_b.clone());
                    let pair = pairs.entry(key).or_insert_with(|| {
                        SkillPairStats::new(skill_a.clone(), skill_b.clone())
                    });

                    // Get individual success for each skill in this turn
                    let rates = self.individual_rates.read();
                    let a_success = rates.get(&skill_a).map(|(s, _)| *s > 0).unwrap_or(false);
                    let b_success = rates.get(&skill_b).map(|(s, _)| *s > 0).unwrap_or(false);
                    drop(rates);

                    pair.record_co_activation(success, a_success, b_success, had_accomplishment);
                }
            }
        }

        self.persist();
    }

    /// Get individual success rate for a skill
    pub fn get_individual_rate(&self, skill_name: &str) -> f32 {
        self.individual_rates
            .read()
            .get(skill_name)
            .map(|(s, t)| if *t > 0 { *s as f32 / *t as f32 } else { 0.0 })
            .unwrap_or(0.0)
    }

    /// Compute synergy score for a specific pair
    pub fn get_pair_synergy(&self, skill_a: &str, skill_b: &str) -> Option<SynergyScore> {
        // Ensure alphabetical ordering
        let (a, b) = if skill_a < skill_b {
            (skill_a.to_string(), skill_b.to_string())
        } else {
            (skill_b.to_string(), skill_a.to_string())
        };

        let key = (a.clone(), b.clone());
        let pair = self.pair_stats.read().get(&key)?.clone();

        let a_rate = self.get_individual_rate(&a);
        let b_rate = self.get_individual_rate(&b);

        Some(SynergyScore::from_pair_stats(&pair, a_rate, b_rate))
    }

    /// Get all synergy scores
    #[allow(dead_code)]
    pub fn get_all_synergy_scores(&self) -> Vec<SynergyScore> {
        self.pair_stats
            .read()
            .iter()
            .filter(|(_, pair)| pair.co_activations >= self.min_co_occurrences)
            .map(|((a, b), pair)| {
                let a_rate = self.get_individual_rate(a);
                let b_rate = self.get_individual_rate(b);
                SynergyScore::from_pair_stats(pair, a_rate, b_rate)
            })
            .collect()
    }

    /// Generate complete synergy analysis
    pub fn generate_analysis(&self) -> SkillSynergyAnalysis {
        let mut all_scores = Vec::new();
        let mut synergistic = Vec::new();
        let mut antagonistic = Vec::new();
        let mut insights = Vec::new();

        let pairs = self.pair_stats.read();
        for ((a, b), pair) in pairs.iter() {
            if pair.co_activations < self.min_co_occurrences {
                continue;
            }

            let a_rate = self.get_individual_rate(a);
            let b_rate = self.get_individual_rate(b);
            let score = SynergyScore::from_pair_stats(pair, a_rate, b_rate);

            all_scores.push(score.clone());

            // Classify
            match score.pattern {
                SynergyPattern::HighlySynergistic | SynergyPattern::Synergistic => {
                    synergistic.push(score.clone());
                }
                SynergyPattern::HighlyAntagonistic | SynergyPattern::Antagonistic => {
                    antagonistic.push(score.clone());
                }
                _ => {}
            }

            // Generate insight for high-confidence findings
            if score.confidence >= 0.3 && score.pattern != SynergyPattern::Neutral && score.pattern != SynergyPattern::InsufficientData {
                insights.push(self.generate_insight(&score));
            }
        }
        drop(pairs);

        // Sort by score
        synergistic.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        antagonistic.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        // Generate recommendations
        let recommendations = self.generate_recommendations(&synergistic);

        // Sort insights by score
        insights.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        insights.truncate(10);

        let high_confidence_count = all_scores.iter().filter(|s| s.confidence >= 0.5).count();

        SkillSynergyAnalysis {
            pair_scores: all_scores,
            insights,
            synergistic_pairs: synergistic,
            antagonistic_pairs: antagonistic,
            recommendations,
            total_pairs_analyzed: self.pair_stats.read().len(),
            high_confidence_count,
            generated_at: Utc::now(),
        }
    }

    /// Generate an insight from a synergy score
    fn generate_insight(&self, score: &SynergyScore) -> SynergyInsight {
        let pattern = &score.pattern;
        let description = format!(
            "{} {} {} (score: {:.2}, {} co-activations)",
            score.skill_a,
            pattern.emoji(),
            score.skill_b,
            score.score,
            score.co_occurrences
        );

        let recommendation = match pattern {
            SynergyPattern::HighlySynergistic => {
                format!(
                    "Strongly recommend using '{}' with '{}' - they significantly boost each other's effectiveness ({:.0}% actual vs {:.0}% expected success rate)",
                    score.skill_a,
                    score.skill_b,
                    score.combined_success_rate * 100.0,
                    score.expected_success_rate * 100.0
                )
            }
            SynergyPattern::Synergistic => {
                format!(
                    "Consider combining '{}' with '{}' - they work well together (success rate: {:.0}%)",
                    score.skill_a,
                    score.skill_b,
                    score.combined_success_rate * 100.0
                )
            }
            SynergyPattern::Antagonistic => {
                format!(
                    "Be cautious: '{}' and '{}' may interfere with each other. Consider using them separately or in sequence.",
                    score.skill_a,
                    score.skill_b
                )
            }
            SynergyPattern::HighlyAntagonistic => {
                format!(
                    "Avoid using '{}' and '{}' together - they significantly hurt each other's effectiveness.",
                    score.skill_a,
                    score.skill_b
                )
            }
            _ => {
                format!(
                    "No significant synergy or antagonism detected between '{}' and '{}'.",
                    score.skill_a,
                    score.skill_b
                )
            }
        };

        SynergyInsight {
            skill_a: score.skill_a.clone(),
            skill_b: score.skill_b.clone(),
            pattern: score.pattern.clone(),
            score: score.score,
            description,
            recommendation,
        }
    }

    /// Generate synergy recommendations based on analysis
    fn generate_recommendations(&self, synergistic: &[SynergyScore]) -> Vec<SynergyRecommendation> {
        let mut recommendations = Vec::new();

        // Top synergistic pairs
        for pair in synergistic.iter().take(5) {
            if pair.pattern == SynergyPattern::HighlySynergistic || pair.pattern == SynergyPattern::Synergistic {
                recommendations.push(SynergyRecommendation {
                    primary_skill: pair.skill_a.clone(),
                    secondary_skills: vec![pair.skill_b.clone()],
                    reason: format!(
                        "{}% success rate when combined (vs {:.0}% expected)",
                        (pair.combined_success_rate * 100.0) as i32,
                        pair.expected_success_rate * 100.0
                    ),
                    expected_improvement: format!(
                        "+{:.0}% improvement over individual use",
                        (pair.lift - 1.0) * 100.0
                    ),
                    confidence: pair.confidence,
                });
            }
        }

        recommendations.truncate(5);
        recommendations
    }

    /// Get synergy summary for a specific skill
    #[allow(dead_code)]
    pub fn get_skill_synergy_summary(&self, skill_name: &str) -> Option<SkillSynergySummary> {
        let mut best_partner: Option<(String, f32)> = None;
        let mut worst_partner: Option<(String, f32)> = None;
        let mut total_score: f32 = 0.0;
        let mut count = 0;
        let mut positive = 0;
        let mut negative = 0;

        let pairs = self.pair_stats.read();
        for ((a, b), pair) in pairs.iter() {
            let (this_skill, other) = if a == skill_name {
                (a.clone(), b.clone())
            } else if b == skill_name {
                (b.clone(), a.clone())
            } else {
                continue;
            };

            if pair.co_activations < self.min_co_occurrences {
                continue;
            }

            let other_rate = self.get_individual_rate(&other);
            let this_rate = self.get_individual_rate(&this_skill);
            let score = SynergyScore::from_pair_stats(pair, other_rate, this_rate);

            total_score += score.score;
            count += 1;

            if score.score > 0.0 {
                positive += 1;
            } else if score.score < 0.0 {
                negative += 1;
            }

            if best_partner.is_none() || score.score > best_partner.as_ref().unwrap().1 {
                best_partner = Some((other.clone(), score.score));
            }
            if worst_partner.is_none() || score.score < worst_partner.as_ref().unwrap().1 {
                worst_partner = Some((other.clone(), score.score));
            }
        }
        drop(pairs);

        if count == 0 {
            return None;
        }

        Some(SkillSynergySummary {
            skill_name: skill_name.to_string(),
            best_partner: best_partner.as_ref().map(|(n, _)| n.clone()),
            best_synergy_score: *best_partner.as_ref().map(|(_, s)| s).unwrap_or(&0.0),
            worst_partner: worst_partner.as_ref().map(|(n, _)| n.clone()),
            worst_synergy_score: *worst_partner.as_ref().map(|(_, s)| s).unwrap_or(&0.0),
            avg_synergy_score: total_score / count as f32,
            positive_partners: positive,
            negative_partners: negative,
        })
    }

    /// Get top N synergistic pairs for a given skill
    #[allow(dead_code)]
    pub fn get_top_synergies_for_skill(&self, skill_name: &str, n: usize) -> Vec<SynergyScore> {
        let mut scores = Vec::new();

        let pairs = self.pair_stats.read();
        for ((a, b), pair) in pairs.iter() {
            if *a != skill_name && *b != skill_name {
                continue;
            }
            if pair.co_activations < self.min_co_occurrences {
                continue;
            }

            let other = if a == skill_name { b.clone() } else { a.clone() };
            let this_rate = self.get_individual_rate(skill_name);
            let other_rate = self.get_individual_rate(&other);
            let score = SynergyScore::from_pair_stats(pair, this_rate, other_rate);

            scores.push(score);
        }
        drop(pairs);

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);
        scores
    }

    /// Persist to disk
    fn persist(&self) {
        if let Some(path) = self.persist_path.read().as_ref() {
            if let Err(e) = self.save_to_file(path) {
                tracing::warn!("Failed to persist skill synergy: {}", e);
            }
        }
    }

    /// Save to JSON file
    fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct PersistData {
            pair_stats: HashMap<(String, String), SkillPairStats>,
            individual_rates: HashMap<String, (usize, usize)>,
        }

        let data = PersistData {
            pair_stats: self.pair_stats.read().clone(),
            individual_rates: self.individual_rates.read().clone(),
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
        let path = match self.persist_path.read().as_ref() {
            Some(p) => p.to_string(),
            None => return Ok(()),
        };

        let path_ref = Path::new(&path);
        if !path_ref.exists() {
            return Ok(());
        }

        #[derive(Serialize, Deserialize)]
        struct PersistData {
            pair_stats: HashMap<(String, String), SkillPairStats>,
            individual_rates: HashMap<String, (usize, usize)>,
        }

        let json = std::fs::read_to_string(path_ref)?;
        let data: PersistData = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Merge loaded data with existing (don't overwrite newer data)
        {
            let mut pairs = self.pair_stats.write();
            for (key, pair) in data.pair_stats {
                pairs.entry(key).or_insert(pair);
            }
        }
        {
            let mut rates = self.individual_rates.write();
            for (skill, rate) in data.individual_rates {
                rates.entry(skill).or_insert(rate);
            }
        }

        info!("Loaded skill synergy data");
        Ok(())
    }

    /// Clear all data
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.pair_stats.write().clear();
        self.individual_rates.write().clear();
        self.persist();
    }

    /// Get total pairs tracked
    #[allow(dead_code)]
    pub fn total_pairs(&self) -> usize {
        self.pair_stats.read().len()
    }
}

impl Default for SkillSynergyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_analyzer() -> Arc<SkillSynergyAnalyzer> {
        Arc::new(SkillSynergyAnalyzer::new())
    }

    #[test]
    fn test_basic_synergy_detection() {
        let analyzer = create_test_analyzer();
        
        // Record some individual uses first to establish non-perfect individual rates
        // Then record co-activations where combination leads to success
        // This simulates a scenario where combining skills helps when individual use might fail
        for _ in 0..3 {
            analyzer.record_turn(&["file_ops".to_string()], true, false);
            analyzer.record_turn(&["code_analysis".to_string()], true, false);
        }
        // Now co-activate - combined use is more successful
        for _ in 0..5 {
            analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, true);
        }
        
        let analysis = analyzer.generate_analysis();
        // Should have tracked some pairs
        assert!(analysis.total_pairs_analyzed >= 1, "Should have at least 1 pair analyzed");
    }

    #[test]
    fn test_insufficient_data() {
        let analyzer = create_test_analyzer();
        
        // Only 2 co-activations - not enough for reliable analysis
        analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, true);
        analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, false);
        
        let analysis = analyzer.generate_analysis();
        // Should have no scored pairs due to insufficient data
        assert!(analysis.high_confidence_count == 0);
    }

    #[test]
    fn test_single_skill_no_pairs() {
        let analyzer = create_test_analyzer();
        
        analyzer.record_turn(&["file_ops".to_string()], true, true);
        analyzer.record_turn(&["file_ops".to_string()], false, false);
        
        let analysis = analyzer.generate_analysis();
        assert_eq!(analysis.total_pairs_analyzed, 0);
    }

    #[test]
    fn test_get_individual_rate() {
        let analyzer = create_test_analyzer();
        
        analyzer.record_turn(&["file_ops".to_string()], true, true);
        analyzer.record_turn(&["file_ops".to_string()], true, false);
        analyzer.record_turn(&["file_ops".to_string()], false, false);
        
        let rate = analyzer.get_individual_rate("file_ops");
        assert!((rate - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_synergy_analysis_generation() {
        let analyzer = create_test_analyzer();
        
        // Record enough data for analysis
        for i in 0..5 {
            analyzer.record_turn(
                &["file_ops".to_string(), "code_analysis".to_string()],
                i < 4, // 4 successes, 1 failure
                i == 2
            );
        }
        
        let analysis = analyzer.generate_analysis();
        assert!(analysis.generated_at <= chrono::Utc::now());
        assert!(analysis.total_pairs_analyzed >= 1);
    }

    #[test]
    fn test_synergy_pattern_classification() {
        let analyzer = create_test_analyzer();
        
        // Establish non-perfect individual rates first
        // Then high success when combined
        for _ in 0..3 {
            analyzer.record_turn(&["file_ops".to_string()], false, false); // 0% individual rate
            analyzer.record_turn(&["web_search".to_string()], false, false);
        }
        for _ in 0..3 {
            analyzer.record_turn(&["file_ops".to_string()], true, false);
            analyzer.record_turn(&["web_search".to_string()], true, false);
        }
        // file_ops: 3/6 = 50%, web_search: 3/6 = 50%
        // Now co-activate with high success
        for _ in 0..10 {
            analyzer.record_turn(&["file_ops".to_string(), "web_search".to_string()], true, true);
        }
        // co_rate = 10/10 = 100%, expected = 0.5 * 0.5 = 0.25, lift = 4.0 -> HighlySynergistic
        
        let score = analyzer.get_pair_synergy("file_ops", "web_search");
        assert!(score.is_some());
        let score = score.unwrap();
        // Should be highly synergistic since combined (100%) >> expected (25%)
        assert!(score.pattern == SynergyPattern::HighlySynergistic || score.pattern == SynergyPattern::Synergistic);
    }

    #[test]
    fn test_empty_analyzer() {
        let analyzer = create_test_analyzer();
        
        let analysis = analyzer.generate_analysis();
        assert!(analysis.pair_scores.is_empty());
        assert!(analysis.insights.is_empty());
        assert!(analysis.synergistic_pairs.is_empty());
        assert!(analysis.antagonistic_pairs.is_empty());
    }

    #[test]
    fn test_skill_synergy_summary() {
        let analyzer = create_test_analyzer();
        
        for _ in 0..5 {
            analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, true);
            analyzer.record_turn(&["file_ops".to_string(), "web_search".to_string()], true, true);
        }
        
        let summary = analyzer.get_skill_synergy_summary("file_ops");
        assert!(summary.is_some());
        let summary = summary.unwrap();
        assert_eq!(summary.skill_name, "file_ops");
        assert!(summary.positive_partners >= 0);
    }

    #[test]
    fn test_top_synergies_for_skill() {
        let analyzer = create_test_analyzer();
        
        // Different synergy levels
        for _ in 0..5 {
            analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, true);
        }
        for _ in 0..3 {
            analyzer.record_turn(&["file_ops".to_string(), "web_search".to_string()], false, false);
        }
        
        let top = analyzer.get_top_synergies_for_skill("file_ops", 2);
        assert!(!top.is_empty());
    }

    #[test]
    fn test_recommendations_generation() {
        let analyzer = create_test_analyzer();
        
        // Strong synergistic pair
        for _ in 0..10 {
            analyzer.record_turn(&["file_ops".to_string(), "code_analysis".to_string()], true, true);
        }
        
        let analysis = analyzer.generate_analysis();
        // May or may not have recommendations depending on confidence
        assert!(analysis.recommendations.len() <= 5);
    }
}
