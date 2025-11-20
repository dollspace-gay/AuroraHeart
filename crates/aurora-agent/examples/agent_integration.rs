//! Example showing how to integrate the agent system with the plugin manager
//!
//! This example demonstrates:
//! - Loading plugins from .AuroraHeart/plugins/
//! - Discovering agent definitions
//! - Spawning and executing specialized agents
//! - Tool permission enforcement

use aurora_agent::{AgentExecutor, AnthropicClient, ToolExecutor};
use aurora_core::plugin::PluginManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // 1. Set up the plugin manager
    let project_root = std::env::current_dir()?;
    let mut plugin_manager = PluginManager::new(&project_root);

    println!("Discovering plugins...");
    plugin_manager.discover_plugins()?;

    println!("Found {} plugins:", plugin_manager.plugins.len());
    for plugin_name in plugin_manager.plugins.keys() {
        println!("  - {}", plugin_name);
    }

    // 2. Get all agent definitions from plugins
    let agent_definitions = plugin_manager.get_all_agents();
    println!("\nDiscovered {} agents:", agent_definitions.len());
    for (agent_name, agent_def) in &agent_definitions {
        println!(
            "  - {}: {}",
            agent_name, agent_def.agent.description
        );
    }

    // 3. Set up the agent executor
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable not set");

    let client = Arc::new(AnthropicClient::new(api_key));
    let tool_executor = ToolExecutor::new();

    let mut agent_executor = AgentExecutor::new(client, tool_executor);

    // 4. Load agents from plugin manager
    let agents_map = agent_definitions
        .into_iter()
        .map(|(name, def)| (name, def.clone()))
        .collect();

    agent_executor.load_agents(agents_map);

    println!("\nAgent executor ready with {} agents", agent_executor.list_agents().len());

    // 5. Example: Execute a specialized agent
    if let Some(agent_name) = agent_executor.list_agents().first() {
        println!("\nExecuting agent: {}", agent_name);

        let result = agent_executor
            .execute_agent(
                agent_name,
                "Hello! Please introduce yourself and tell me what you can do.",
            )
            .await;

        match result {
            Ok((response, events)) => {
                println!("\n=== Agent Response ===");
                println!("{}", response);
                println!("\n=== Events ({}) ===", events.len());
                for (i, event) in events.iter().enumerate() {
                    println!("{}. {:?}", i + 1, event);
                }
            }
            Err(e) => {
                eprintln!("Error executing agent: {}", e);
            }
        }
    } else {
        println!("\nNo agents available. Create a plugin with an agent definition in:");
        println!("  .AuroraHeart/plugins/my-plugin/agents/my-agent.toml");
    }

    // 6. Example: Execute multiple agents in parallel
    if agent_executor.list_agents().len() >= 2 {
        println!("\n\nExecuting multiple agents in parallel...");

        let tasks = agent_executor
            .list_agents()
            .iter()
            .take(2)
            .map(|name| (name.clone(), "What are you specialized for?".to_string()))
            .collect();

        let results = agent_executor.execute_agents_parallel(tasks).await;

        for (i, result) in results.iter().enumerate() {
            match result {
                Ok((response, _)) => {
                    println!("\n=== Agent {} Response ===", i + 1);
                    println!("{}", response);
                }
                Err(e) => {
                    eprintln!("Agent {} error: {}", i + 1, e);
                }
            }
        }
    }

    Ok(())
}
