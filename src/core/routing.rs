//! Message routing for TinyVegeta.
#![allow(dead_code)]
//!
//! Handles:
//! - Agent routing (@agent_id prefix)
//! - Team routing (@team_id prefix)
//! - Mention tag extraction ([@agent: message])

use regex::Regex;
use std::collections::HashMap;

use crate::config::{AgentConfig, Settings, TeamConfig};

/// Parse agent routing from message prefix.
///
/// Returns the agent ID if the message starts with `@agent_id `.
///
/// # Examples
///
/// ```
/// let (agent, message) = parse_agent_routing("@coder fix the bug").unwrap();
/// assert_eq!(agent, "coder");
/// assert_eq!(message, "fix the bug");
/// ```
pub fn parse_agent_routing(message: &str) -> Option<(String, String)> {
    let re = Regex::new(r"^@(\w+)\s+(.+)$").ok()?;

    let caps = re.captures(message)?;
    let agent_id = caps.get(1)?.as_str().to_lowercase();
    let remaining = caps.get(2)?.as_str();

    Some((agent_id, remaining.to_string()))
}

/// Parse team routing from message prefix.
///
/// Returns the team ID if the message starts with `@team_id ` (where team_id is a valid team).
///
/// # Examples
///
/// ```
/// let (team, message) = parse_team_routing("@dev fix the bug", &teams).unwrap();
/// assert_eq!(team, "dev");
/// ```
pub fn parse_team_routing(
    message: &str,
    teams: &HashMap<String, TeamConfig>,
) -> Option<(String, String)> {
    let re = Regex::new(r"^@(\w+)\s+(.+)$").ok()?;

    let caps = re.captures(message)?;
    let team_id = caps.get(1)?.as_str().to_lowercase();
    let remaining = caps.get(2)?.as_str();

    // Check if it's a valid team
    if teams.contains_key(&team_id) {
        return Some((team_id, remaining.to_string()));
    }

    None
}

/// Extract mention tags from a response.
///
/// Format: `[@agent_id: message]` or `[@agent1,agent2: message]`
///
/// Returns a list of (agent_id, message) tuples.
///
/// # Examples
///
/// ```
/// let mentions = extract_mentions("Hello [@coder: fix this] [@reviewer: check this]");
/// assert_eq!(mentions.len(), 2);
/// assert_eq!(mentions[0].0, "coder");
/// ```
pub fn extract_mentions(response: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();

    // Extract shared context (text outside all mention tags)
    let shared_context = extract_shared_context(response);

    // Regex for [@agent: message] or [@agent1,agent2: message]
    let re = match Regex::new(r"\[@(\w+(?:,\w+)*):\s*([\s\S]*?)\]") {
        Ok(r) => r,
        Err(_) => return results,
    };

    let mut seen = std::collections::HashSet::new();

    for caps in re.captures_iter(response) {
        let targets = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let direct_message = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();

        // Split comma-separated targets
        for target in targets.split(',') {
            let target_id = target.trim().to_lowercase();

            if target_id.is_empty() || seen.contains(&target_id) {
                continue;
            }

            seen.insert(target_id.clone());

            // Construct full message (shared context + directed message)
            let full_message = if shared_context.is_empty() {
                direct_message.to_string()
            } else {
                format!(
                    "{}\n\n------\n\nDirected to you:\n{}",
                    shared_context, direct_message
                )
            };

            results.push((target_id, full_message));
        }
    }

    results
}

/// Extract shared context from response (text outside mention tags).
fn extract_shared_context(response: &str) -> String {
    let re = match Regex::new(r"\[@(\w+(?:,\w+)*):\s*[\s\S]*?\]") {
        Ok(r) => r,
        Err(_) => return response.to_string(),
    };

    // Remove all mention tags and return the remaining text
    let context = re.replace_all(response, "").trim().to_string();
    context
}

/// Find the first team that contains an agent.
pub fn find_team_for_agent(
    agent_id: &str,
    teams: &HashMap<String, TeamConfig>,
) -> Option<(String, TeamConfig)> {
    for (team_id, team) in teams {
        if team.agents.contains(&agent_id.to_string()) {
            return Some((team_id.clone(), team.clone()));
        }
    }
    None
}

/// Check if a mentioned ID is a valid teammate of the current agent.
pub fn is_teammate(
    mentioned_id: &str,
    current_agent_id: &str,
    team_id: &str,
    teams: &HashMap<String, TeamConfig>,
    agents: &HashMap<String, AgentConfig>,
) -> bool {
    let team = match teams.get(team_id) {
        Some(t) => t,
        None => return false,
    };

    mentioned_id != current_agent_id
        && team.agents.contains(&mentioned_id.to_string())
        && agents.contains_key(mentioned_id)
}

/// Resolve an agent ID from various routing formats.
///
/// - `@agent_id` -> agent_id
/// - `@team_id` -> team.leader_agent
/// - `@agent1,agent2` -> [agent1, agent2]
pub fn resolve_routing_target(
    target: &str,
    teams: &HashMap<String, TeamConfig>,
    agents: &HashMap<String, AgentConfig>,
) -> Vec<String> {
    let mut results = Vec::new();

    // Check if it's a team
    if let Some(team) = teams.get(target) {
        if let Some(leader) = &team.leader_agent {
            if agents.contains_key(leader) {
                results.push(leader.clone());
            }
        }
        return results;
    }

    // Check if it's a comma-separated list
    if target.contains(',') {
        for t in target.split(',') {
            let t = t.trim();
            if agents.contains_key(t) {
                results.push(t.to_string());
            }
        }
        return results;
    }

    // Check if it's a single agent
    if agents.contains_key(target) {
        results.push(target.to_string());
    }

    results
}

/// Get the default agent from settings.
pub fn get_default_agent(settings: &Settings) -> Option<String> {
    if let Some(id) = settings.routing.default_agent.as_deref() {
        if settings.agents.contains_key(id) {
            return Some(id.to_string());
        }
    }

    if settings.agents.contains_key("assistant") {
        return Some("assistant".to_string());
    }
    if settings.agents.contains_key("tinyvegeta") {
        return Some("tinyvegeta".to_string());
    }
    if settings.agents.contains_key("tiny-vegeta") {
        return Some("tiny-vegeta".to_string());
    }

    // Stable fallback.
    let mut ids: Vec<String> = settings.agents.keys().cloned().collect();
    ids.sort();
    ids.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_routing() {
        let (agent, msg) = parse_agent_routing("@coder fix the bug").unwrap();
        assert_eq!(agent, "coder");
        assert_eq!(msg, "fix the bug");

        let (agent, msg) = parse_agent_routing("@Coder fix the bug").unwrap();
        assert_eq!(agent, "coder");

        // No routing prefix
        let result = parse_agent_routing("just a message");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_mentions() {
        let mentions = extract_mentions("Hello [@coder: fix this]");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].0, "coder");
        assert_eq!(mentions[0].1, "fix this");

        // Multiple mentions
        let mentions = extract_mentions("Start [@coder: fix] [@reviewer: check]");
        assert_eq!(mentions.len(), 2);

        // Comma-separated
        let mentions = extract_mentions("Hey [@coder,reviewer: look at this]");
        assert_eq!(mentions.len(), 2);

        // Shared context
        let mentions = extract_mentions("Please review [@reviewer: the PR]");
        assert!(mentions[0].1.contains("Please review"));
    }

    #[test]
    fn test_find_team_for_agent() {
        let mut teams = HashMap::new();
        teams.insert(
            "dev".to_string(),
            TeamConfig {
                name: "Dev Team".to_string(),
                agents: vec!["coder".to_string(), "reviewer".to_string()],
                leader_agent: Some("coder".to_string()),
            },
        );

        let result = find_team_for_agent("coder", &teams);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "dev");
    }

    #[test]
    fn test_team_routing_to_leader() {
        let mut teams = HashMap::new();
        teams.insert(
            "board".to_string(),
            TeamConfig {
                name: "Board".to_string(),
                agents: vec!["assistant".to_string(), "coder".to_string()],
                leader_agent: Some("assistant".to_string()),
            },
        );
        let mut agents = HashMap::new();
        agents.insert("assistant".to_string(), AgentConfig::default());
        agents.insert("coder".to_string(), AgentConfig::default());
        let out = resolve_routing_target("board", &teams, &agents);
        assert_eq!(out, vec!["assistant".to_string()]);
    }
}
