//! Policy management commands
//!
//! This module contains commands for policy management.
//! It allows defining, listing, and removing policies that control plugin permissions.

use super::interfaces::policy;
use anyhow::Result;
use colored::*;

/// Add a policy rule
pub fn add_policy(rule_id: &str, subject: &str, object: &str, action: &str) -> Result<()> {
    // Use the policy interface to add the rule
    policy::add_policy_rule(rule_id, subject, object, action)?;

    println!("{}", "Policy rule added successfully".green().bold());
    println!("ID:      {}", rule_id.cyan());
    println!("Subject: {}", subject);
    println!("Object:  {}", object);
    println!("Action:  {}", action.yellow());

    // Show relevant next commands
    println!("\n{}", "Related commands:".bold());
    println!(
        "  {}",
        format!(
            "lion-cli policy check --subject \"{}\" --object \"{}\" --action \"{}\"",
            subject, object, action
        )
        .italic()
    );
    println!("  {}", "lion-cli policy list".italic());

    Ok(())
}

/// List all policy rules
pub fn list_policies() -> Result<()> {
    // Use the policy interface to get all rules
    let rules = policy::list_policy_rules()?;

    if rules.is_empty() {
        println!("{}", "No policy rules defined".yellow().bold());
        println!("\nTo add a policy rule, use:");
        println!("  {}", "lion-cli policy add --rule-id <ID> --subject <SUBJECT> --object <OBJECT> --action <ACTION>".italic());
        return Ok(());
    }

    let rules_count = rules.len();
    println!(
        "{:<10} | {:<40} | {:<20} | {}",
        "ID".underline(),
        "Subject".underline(),
        "Object".underline(),
        "Action".underline()
    );
    println!("{}", "-".repeat(80));

    for rule in &rules {
        // Color the action based on its type
        let action_colored = match rule.action.to_lowercase().as_str() {
            "allow" => rule.action.green(),
            "deny" => rule.action.red(),
            "audit" => rule.action.yellow(),
            _ => rule.action.normal(),
        };

        println!(
            "{:<10} | {:<40} | {:<20} | {}",
            rule.id.cyan(),
            rule.subject,
            rule.object,
            action_colored
        );
    }

    println!("\n{} policy rules found", rules_count.to_string().yellow());

    Ok(())
}

/// Remove a policy rule
pub fn remove_policy(rule_id: &str) -> Result<()> {
    println!("Removing policy rule: {}", rule_id.cyan());

    // Use the policy interface to remove the rule
    policy::remove_policy_rule(rule_id)?;

    println!("{}", "Policy rule removed successfully".green());
    Ok(())
}

/// Check if a policy would allow a specific action
pub fn check_policy(subject: &str, object: &str, action: &str) -> Result<()> {
    // Use the policy interface to check the policy
    let decision = policy::check_policy(subject, object, action)?;

    println!("{}", "Policy Check".bold().underline());
    println!("Subject: {}", subject);
    println!("Object: {}", object);
    println!("Action:  {}", action.yellow());
    println!("\nDecision: {}", decision.decision.to_string().bold());

    println!("\nApplicable rules:");
    for rule_id in &decision.applicable_rules {
        println!("  - {}", rule_id.cyan());
    }

    Ok(())
}
