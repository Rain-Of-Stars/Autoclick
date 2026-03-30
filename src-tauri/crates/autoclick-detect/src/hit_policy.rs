use serde::{Deserialize, Serialize};

use crate::r#match::MatchResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HitPolicyConfig {
    pub threshold: f32,
    pub min_detections: u32,
    pub cooldown_ms: u64,
}

impl Default for HitPolicyConfig {
    fn default() -> Self {
        Self {
            threshold: 0.88,
            min_detections: 1,
            cooldown_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HitDecision {
    NoMatch,
    BelowThreshold,
    Pending(u32),
    CoolingDown(u64),
    ShouldClick(MatchResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
struct LastHitKey {
    template_id: String,
    x: u32,
    y: u32,
}

#[derive(Debug, Default)]
pub struct HitPolicy {
    config: HitPolicyConfig,
    consecutive_hits: u32,
    last_click_at_ms: Option<u64>,
    last_hit_key: Option<LastHitKey>,
}

impl HitPolicy {
    pub fn new(config: HitPolicyConfig) -> Self {
        Self {
            config,
            consecutive_hits: 0,
            last_click_at_ms: None,
            last_hit_key: None,
        }
    }

    pub fn evaluate(
        &mut self,
        matched: Option<MatchResult>,
        frame_timestamp_ms: u64,
    ) -> HitDecision {
        let Some(matched) = matched else {
            self.consecutive_hits = 0;
            self.last_hit_key = None;
            return HitDecision::NoMatch;
        };

        if matched.score < self.config.threshold {
            self.consecutive_hits = 0;
            self.last_hit_key = None;
            return HitDecision::BelowThreshold;
        }

        if let Some(last_click_at_ms) = self.last_click_at_ms {
            let elapsed = frame_timestamp_ms.saturating_sub(last_click_at_ms);
            if elapsed < self.config.cooldown_ms {
                return HitDecision::CoolingDown(self.config.cooldown_ms - elapsed);
            }
        }

        let current_key = LastHitKey {
            template_id: matched.template_id.clone(),
            x: matched.x,
            y: matched.y,
        };

        if self.last_hit_key.as_ref() == Some(&current_key) {
            self.consecutive_hits += 1;
        } else {
            self.last_hit_key = Some(current_key);
            self.consecutive_hits = 1;
        }

        if self.consecutive_hits >= self.config.min_detections.max(1) {
            self.last_click_at_ms = Some(frame_timestamp_ms);
            self.consecutive_hits = 0;
            self.last_hit_key = None;
            HitDecision::ShouldClick(matched)
        } else {
            HitDecision::Pending(self.consecutive_hits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HitDecision, HitPolicy, HitPolicyConfig};
    use crate::r#match::MatchResult;

    fn matched(score: f32) -> MatchResult {
        MatchResult {
            template_id: "id-1".to_string(),
            template_name: "sample".to_string(),
            score,
            x: 5,
            y: 6,
            width: 10,
            height: 12,
            scale: 1.0,
        }
    }

    #[test]
    fn hit_policy_requires_multiple_detections() {
        let mut policy = HitPolicy::new(HitPolicyConfig {
            threshold: 0.9,
            min_detections: 2,
            cooldown_ms: 0,
        });
        assert_eq!(
            policy.evaluate(Some(matched(0.95)), 10),
            HitDecision::Pending(1)
        );
        assert!(matches!(
            policy.evaluate(Some(matched(0.95)), 20),
            HitDecision::ShouldClick(_)
        ));
    }

    #[test]
    fn hit_policy_enforces_cooldown() {
        let mut policy = HitPolicy::new(HitPolicyConfig {
            threshold: 0.9,
            min_detections: 1,
            cooldown_ms: 100,
        });
        assert!(matches!(
            policy.evaluate(Some(matched(0.95)), 10),
            HitDecision::ShouldClick(_)
        ));
        assert_eq!(
            policy.evaluate(Some(matched(0.95)), 50),
            HitDecision::CoolingDown(60)
        );
    }
}
