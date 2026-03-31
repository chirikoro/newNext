//! A/B Testing and Feature Flags for Hayabusa.
//!
//! Cookie-based experiment assignment with deterministic hashing,
//! variant tracking, and conversion measurement.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let mut experiments = Experiments::new();
//! experiments.add(Experiment::new("hero-cta")
//!     .variant("control", 50)
//!     .variant("new-button", 50));
//!
//! let variant = experiments.assign("hero-cta", "user-123");
//! ```

use std::collections::HashMap;

// ─── Experiment ─────────────────────────────────────────────

/// An A/B test experiment
#[derive(Debug, Clone)]
pub struct Experiment {
    pub name: String,
    pub variants: Vec<Variant>,
    pub enabled: bool,
    pub sticky: bool, // use cookie to persist assignment
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub weight: u32,
}

impl Experiment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variants: Vec::new(),
            enabled: true,
            sticky: true,
        }
    }

    pub fn variant(mut self, name: impl Into<String>, weight: u32) -> Self {
        self.variants.push(Variant {
            name: name.into(),
            weight,
        });
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn sticky(mut self, sticky: bool) -> Self {
        self.sticky = sticky;
        self
    }

    /// Total weight across all variants
    pub fn total_weight(&self) -> u32 {
        self.variants.iter().map(|v| v.weight).sum()
    }

    /// Assign a variant based on a deterministic hash of the user key
    pub fn assign(&self, user_key: &str) -> Option<&str> {
        if !self.enabled || self.variants.is_empty() {
            return None;
        }
        let hash = deterministic_hash(&self.name, user_key);
        let total = self.total_weight();
        if total == 0 {
            return None;
        }
        let bucket = hash % total;
        let mut cumulative = 0;
        for variant in &self.variants {
            cumulative += variant.weight;
            if bucket < cumulative {
                return Some(&variant.name);
            }
        }
        self.variants.last().map(|v| v.name.as_str())
    }
}

// ─── Feature Flags ──────────────────────────────────────────

/// Feature flag with optional rollout percentage
#[derive(Debug, Clone)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub rollout_percentage: u32, // 0-100
    pub allowed_users: Vec<String>,
}

impl FeatureFlag {
    pub fn new(name: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            enabled,
            rollout_percentage: 100,
            allowed_users: Vec::new(),
        }
    }

    pub fn rollout(mut self, percentage: u32) -> Self {
        self.rollout_percentage = percentage.min(100);
        self
    }

    pub fn allow_user(mut self, user_id: impl Into<String>) -> Self {
        self.allowed_users.push(user_id.into());
        self
    }

    /// Check if the flag is active for a specific user
    pub fn is_active_for(&self, user_id: &str) -> bool {
        if !self.enabled {
            return false;
        }
        // Explicitly allowed users always get the feature
        if self.allowed_users.iter().any(|u| u == user_id) {
            return true;
        }
        // Percentage-based rollout
        if self.rollout_percentage >= 100 {
            return true;
        }
        let hash = deterministic_hash(&self.name, user_id);
        (hash % 100) < self.rollout_percentage
    }
}

// ─── Experiments Manager ────────────────────────────────────

/// Manager for all experiments and feature flags
#[derive(Debug, Clone)]
pub struct Experiments {
    pub experiments: HashMap<String, Experiment>,
    pub flags: HashMap<String, FeatureFlag>,
    pub conversions: HashMap<String, Vec<Conversion>>,
}

/// A conversion event
#[derive(Debug, Clone)]
pub struct Conversion {
    pub experiment: String,
    pub variant: String,
    pub user_key: String,
    pub event: String,
    pub value: Option<f64>,
}

impl Experiments {
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
            flags: HashMap::new(),
            conversions: Vec::new().into_iter().collect(),
        }
    }

    pub fn add(mut self, experiment: Experiment) -> Self {
        self.experiments.insert(experiment.name.clone(), experiment);
        self
    }

    pub fn add_flag(mut self, flag: FeatureFlag) -> Self {
        self.flags.insert(flag.name.clone(), flag);
        self
    }

    /// Assign a variant for a user in an experiment
    pub fn assign(&self, experiment_name: &str, user_key: &str) -> Option<&str> {
        self.experiments
            .get(experiment_name)
            .and_then(|exp| exp.assign(user_key))
    }

    /// Check if a feature flag is active for a user
    pub fn is_flag_active(&self, flag_name: &str, user_id: &str) -> bool {
        self.flags
            .get(flag_name)
            .map_or(false, |f| f.is_active_for(user_id))
    }

    /// Record a conversion event
    pub fn record_conversion(&mut self, experiment: &str, variant: &str, user_key: &str, event: &str, value: Option<f64>) {
        let conversion = Conversion {
            experiment: experiment.to_string(),
            variant: variant.to_string(),
            user_key: user_key.to_string(),
            event: event.to_string(),
            value,
        };
        self.conversions
            .entry(experiment.to_string())
            .or_default()
            .push(conversion);
    }

    /// Get conversion count for a variant
    pub fn conversion_count(&self, experiment: &str, variant: &str) -> usize {
        self.conversions
            .get(experiment)
            .map_or(0, |cs| cs.iter().filter(|c| c.variant == variant).count())
    }

    /// Generate the cookie value for persisting assignments
    pub fn assignment_cookie(&self, user_key: &str) -> String {
        let mut assignments = Vec::new();
        for (name, exp) in &self.experiments {
            if exp.sticky {
                if let Some(variant) = exp.assign(user_key) {
                    assignments.push(format!("{}={}", name, variant));
                }
            }
        }
        assignments.join("|")
    }

    /// Parse assignments from cookie value
    pub fn parse_cookie(cookie: &str) -> HashMap<String, String> {
        cookie
            .split('|')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next()?;
                Some((key.to_string(), value.to_string()))
            })
            .collect()
    }
}

impl Default for Experiments {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Client-Side Script ─────────────────────────────────────

/// Generate client-side A/B test tracking script
pub fn ab_test_script(experiment: &str, variant: &str) -> String {
    format!(
        r#"<script>
(function(){{
  var exp='{exp}',variant='{var}';
  document.documentElement.setAttribute('data-experiment-'+exp, variant);
  if(window.gtag)gtag('event','experiment_impression',{{experiment_id:exp,variant_id:variant}});
  if(window.plausible)plausible('Experiment',{{props:{{experiment:exp,variant:variant}}}});
}})();
</script>"#,
        exp = experiment,
        var = variant,
    )
}

/// Generate CSS for showing/hiding variants
pub fn variant_css(experiment: &str, variants: &[&str]) -> String {
    let mut css = String::new();
    for variant in variants {
        // Hide all variants by default
        css.push_str(&format!(
            "[data-variant=\"{v}\"] {{ display: none; }}\n",
            v = variant
        ));
        // Show only the active variant
        css.push_str(&format!(
            "[data-experiment-{exp}=\"{v}\"] [data-variant=\"{v}\"] {{ display: block; }}\n",
            exp = experiment,
            v = variant
        ));
    }
    format!("<style>{}</style>", css)
}

// ─── Deterministic Hash ─────────────────────────────────────

fn deterministic_hash(experiment: &str, user_key: &str) -> u32 {
    let input = format!("{}:{}", experiment, user_key);
    let mut hash: u32 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_assign() {
        let exp = Experiment::new("test")
            .variant("a", 50)
            .variant("b", 50);
        let v = exp.assign("user1").unwrap();
        assert!(v == "a" || v == "b");
    }

    #[test]
    fn test_experiment_deterministic() {
        let exp = Experiment::new("test")
            .variant("a", 50)
            .variant("b", 50);
        let v1 = exp.assign("user123");
        let v2 = exp.assign("user123");
        assert_eq!(v1, v2); // same user always gets same variant
    }

    #[test]
    fn test_experiment_distribution() {
        let exp = Experiment::new("test")
            .variant("a", 50)
            .variant("b", 50);
        let mut counts = HashMap::new();
        for i in 0..1000 {
            let key = format!("user_{}", i);
            let v = exp.assign(&key).unwrap();
            *counts.entry(v.to_string()).or_insert(0) += 1;
        }
        // Rough check: both should have significant count
        assert!(*counts.get("a").unwrap_or(&0) > 300);
        assert!(*counts.get("b").unwrap_or(&0) > 300);
    }

    #[test]
    fn test_experiment_disabled() {
        let exp = Experiment::new("test")
            .variant("a", 100)
            .enabled(false);
        assert_eq!(exp.assign("user1"), None);
    }

    #[test]
    fn test_experiment_empty() {
        let exp = Experiment::new("test");
        assert_eq!(exp.assign("user1"), None);
    }

    #[test]
    fn test_feature_flag_basic() {
        let flag = FeatureFlag::new("dark_mode", true);
        assert!(flag.is_active_for("any_user"));

        let flag = FeatureFlag::new("dark_mode", false);
        assert!(!flag.is_active_for("any_user"));
    }

    #[test]
    fn test_feature_flag_rollout() {
        let flag = FeatureFlag::new("beta", true).rollout(50);
        let mut active = 0;
        for i in 0..1000 {
            if flag.is_active_for(&format!("user_{}", i)) {
                active += 1;
            }
        }
        // ~50% should be active (allow wide margin)
        assert!(active > 300 && active < 700, "active={}", active);
    }

    #[test]
    fn test_feature_flag_allowed_user() {
        let flag = FeatureFlag::new("beta", true)
            .rollout(0) // 0% rollout
            .allow_user("vip_user");
        assert!(flag.is_active_for("vip_user"));
        assert!(!flag.is_active_for("regular_user"));
    }

    #[test]
    fn test_experiments_manager() {
        let mgr = Experiments::new()
            .add(Experiment::new("hero").variant("a", 50).variant("b", 50))
            .add_flag(FeatureFlag::new("new_ui", true));

        let v = mgr.assign("hero", "user1");
        assert!(v.is_some());
        assert!(mgr.is_flag_active("new_ui", "anyone"));
    }

    #[test]
    fn test_assignment_cookie() {
        let mgr = Experiments::new()
            .add(Experiment::new("exp1").variant("a", 100));
        let cookie = mgr.assignment_cookie("user1");
        assert!(cookie.contains("exp1=a"));
    }

    #[test]
    fn test_parse_cookie() {
        let assignments = Experiments::parse_cookie("exp1=a|exp2=b");
        assert_eq!(assignments.get("exp1"), Some(&"a".to_string()));
        assert_eq!(assignments.get("exp2"), Some(&"b".to_string()));
    }

    #[test]
    fn test_conversion_tracking() {
        let mut mgr = Experiments::new()
            .add(Experiment::new("cta").variant("red", 50).variant("blue", 50));
        mgr.record_conversion("cta", "red", "user1", "click", None);
        mgr.record_conversion("cta", "red", "user2", "click", None);
        mgr.record_conversion("cta", "blue", "user3", "click", None);
        assert_eq!(mgr.conversion_count("cta", "red"), 2);
        assert_eq!(mgr.conversion_count("cta", "blue"), 1);
    }

    #[test]
    fn test_ab_test_script() {
        let script = ab_test_script("hero", "variant_b");
        assert!(script.contains("hero"));
        assert!(script.contains("variant_b"));
        assert!(script.contains("data-experiment"));
    }

    #[test]
    fn test_variant_css() {
        let css = variant_css("hero", &["control", "new"]);
        assert!(css.contains("data-variant=\"control\""));
        assert!(css.contains("data-experiment-hero"));
    }

    #[test]
    fn test_total_weight() {
        let exp = Experiment::new("t").variant("a", 30).variant("b", 70);
        assert_eq!(exp.total_weight(), 100);
    }
}
