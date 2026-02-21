//! Deterministic task routing with typed schema.

use regex::Regex;

use crate::config::Settings;

#[derive(Debug, Clone)]
pub struct RoutedTask {
    pub intent: String,
    pub owner: String,
    pub priority: String,
    pub deadline: Option<String>,
    pub reason: String,
}

pub struct TaskRouter;

impl TaskRouter {
    pub fn route(message: &str, settings: &Settings, explicit_target: Option<&str>) -> RoutedTask {
        if let Some(target) = explicit_target {
            return RoutedTask {
                intent: infer_intent(message).to_string(),
                owner: target.to_string(),
                priority: infer_priority(message).to_string(),
                deadline: extract_deadline(message),
                reason: "explicit target provided by user".to_string(),
            };
        }

        let intent = infer_intent(message);
        let owner = select_owner(intent, settings);
        let priority = infer_priority(message);
        let deadline = extract_deadline(message);
        RoutedTask {
            intent: intent.to_string(),
            owner,
            priority: priority.to_string(),
            deadline,
            reason: format!("hard-rule routing by intent '{}'", intent),
        }
    }
}

fn infer_intent(message: &str) -> &'static str {
    let m = message.to_lowercase();

    if has_any(
        &m,
        &[
            "vulnerability",
            "security",
            "xss",
            "csrf",
            "auth",
            "token",
            "exploit",
            "permissions",
        ],
    ) {
        return "security";
    }
    if has_any(
        &m,
        &[
            "deploy",
            "infra",
            "incident",
            "latency",
            "uptime",
            "docker",
            "kubernetes",
            "monitoring",
        ],
    ) {
        return "operations";
    }
    if has_any(
        &m,
        &["campaign", "brand", "positioning", "launch", "audience", "ad copy"],
    ) {
        return "marketing";
    }
    if has_any(
        &m,
        &["seo", "serp", "keywords", "ranking", "backlinks", "organic traffic"],
    ) {
        return "seo";
    }
    if has_any(
        &m,
        &["lead", "pipeline", "deal", "prospect", "pricing", "close rate"],
    ) {
        return "sales";
    }
    if has_any(
        &m,
        &[
            "bug",
            "refactor",
            "code",
            "compile",
            "test",
            "rust",
            "api",
            "function",
            "error",
        ],
    ) {
        return "coding";
    }
    "general"
}

fn infer_priority(message: &str) -> &'static str {
    let m = message.to_lowercase();
    if has_any(
        &m,
        &["p0", "critical", "urgent", "asap", "immediately", "production down"],
    ) {
        return "urgent";
    }
    if has_any(&m, &["high", "today", "blocker", "important"]) {
        return "high";
    }
    if has_any(&m, &["low", "later", "someday", "nice to have"]) {
        return "low";
    }
    "medium"
}

fn extract_deadline(message: &str) -> Option<String> {
    let iso = Regex::new(r"\b(20\d{2}-\d{2}-\d{2})\b").ok()?;
    if let Some(cap) = iso.captures(message) {
        return cap.get(1).map(|m| m.as_str().to_string());
    }
    let rel = Regex::new(r"(?i)\b(today|tomorrow|next week|this week)\b").ok()?;
    rel.captures(message)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
}

fn select_owner(intent: &str, settings: &Settings) -> String {
    let candidates: &[&str] = match intent {
        "security" => &["security", "assistant"],
        "operations" => &["operations", "assistant"],
        "marketing" => &["marketing", "assistant"],
        "seo" => &["seo", "assistant"],
        "sales" => &["sales", "assistant"],
        "coding" => &["coder", "assistant"],
        _ => &["assistant"],
    };

    for candidate in candidates {
        if settings.agents.contains_key(*candidate) {
            return (*candidate).to_string();
        }
    }

    crate::core::routing::get_default_agent(settings)
        .or_else(|| settings.agents.keys().next().cloned())
        .unwrap_or_else(|| "assistant".to_string())
}

fn has_any(message: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| message.contains(term))
}
