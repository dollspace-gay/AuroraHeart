//! Agent system for spawning specialized sub-agents with custom prompts and tool permissions
//!
//! This module provides the agent execution system that integrates with the plugin architecture
//! to spawn specialized AI agents with:
//! - Custom system prompts
//! - Tool access control (allowed/denied lists)
//! - Behavioral settings (max_turns, stop_on_error)
//! - Parallel execution support

use crate::client::{AnthropicClient, ClientError, AgenticEvent};
use crate::conversation::Conversation;
use crate::tools::{Tool, ToolExecutor, all_tools};
use aurora_core::plugin::{AgentDefinition, AgentInfo};
use aurora_core::hooks::{HookExecutor, ToolCallContext, AfterToolCallContext};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    /// Client error during API calls
    #[error("Client error: {0}")]
    Client(#[from] ClientError),

    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Invalid agent configuration
    #[error("Invalid agent configuration: {0}")]
    InvalidConfiguration(String),

    /// Tool access denied
    #[error("Tool access denied: {0}")]
    ToolAccessDenied(String),

    /// Agent execution exceeded max turns
    #[error("Agent execution exceeded max turns: {0}")]
    MaxTurnsExceeded(u32),

    /// Agent stopped due to error
    #[error("Agent stopped due to error: {0}")]
    StoppedOnError(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;

/// Agent execution context - isolated environment for a specific agent
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Agent configuration
    pub agent_info: AgentInfo,
    /// Conversation with custom system prompt
    pub conversation: Conversation,
    /// Current turn count
    pub turns: u32,
}

impl AgentContext {
    /// Create a new agent context from an agent definition
    pub fn new(agent_info: AgentInfo) -> Self {
        // Build system prompt from agent configuration
        let system_prompt = format!(
            "{}\n\n{}",
            agent_info.system_prompt.role,
            agent_info.system_prompt.instructions
        );

        let conversation = Conversation::with_system_prompt(system_prompt);

        Self {
            agent_info,
            conversation,
            turns: 0,
        }
    }

    /// Add a user message to the agent's conversation
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.conversation.add_user_message(content);
    }

    /// Get the model to use for this agent
    pub fn model(&self) -> &str {
        &self.agent_info.model
    }

    /// Get max turns allowed for this agent
    pub fn max_turns(&self) -> u32 {
        self.agent_info.behavior.max_turns
    }

    /// Check if agent should stop on error
    pub fn stop_on_error(&self) -> bool {
        self.agent_info.behavior.stop_on_error
    }

    /// Increment turn counter
    pub fn increment_turn(&mut self) {
        self.turns += 1;
    }

    /// Check if max turns exceeded
    pub fn is_max_turns_exceeded(&self) -> bool {
        self.turns >= self.max_turns()
    }
}

/// Filtered tool executor that enforces agent tool permissions
pub struct FilteredToolExecutor {
    /// Base tool executor
    base_executor: ToolExecutor,
    /// Tools that are allowed (if empty, all tools except denied are allowed)
    allowed_tools: Vec<String>,
    /// Tools that are explicitly denied
    denied_tools: Vec<String>,
}

impl FilteredToolExecutor {
    /// Create a new filtered tool executor
    pub fn new(
        base_executor: ToolExecutor,
        allowed_tools: Vec<String>,
        denied_tools: Vec<String>,
    ) -> Self {
        Self {
            base_executor,
            allowed_tools,
            denied_tools,
        }
    }

    /// Check if a tool is allowed for this agent
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // If tool is explicitly denied, reject
        if self.denied_tools.contains(&tool_name.to_string()) {
            return false;
        }

        // If allowed list is empty, allow all non-denied tools
        if self.allowed_tools.is_empty() {
            return true;
        }

        // Otherwise, only allow tools in the allowed list
        self.allowed_tools.contains(&tool_name.to_string())
    }

    /// Get filtered list of tools available to this agent
    pub fn get_available_tools(&self) -> Vec<Tool> {
        all_tools()
            .into_iter()
            .filter(|tool| self.is_tool_allowed(&tool.name))
            .collect()
    }

    /// Execute a tool use request with permission checking
    pub async fn execute(&self, tool_use: &crate::tools::ToolUse) -> crate::tools::ToolResult {
        // Check if tool is allowed
        if !self.is_tool_allowed(&tool_use.name) {
            return crate::tools::ToolResult::error(
                tool_use.id.clone(),
                format!("Tool '{}' is not allowed for this agent", tool_use.name),
            );
        }

        // Execute with base executor
        self.base_executor.execute(tool_use).await
    }
}

/// Agent executor for running specialized sub-agents
pub struct AgentExecutor {
    /// Anthropic API client
    client: Arc<AnthropicClient>,
    /// Base tool executor
    base_tool_executor: ToolExecutor,
    /// Available agent definitions (from plugins)
    agent_definitions: HashMap<String, AgentDefinition>,
    /// Optional hook executor for lifecycle events
    hook_executor: Option<HookExecutor>,
}

impl AgentExecutor {
    /// Create a new agent executor
    pub fn new(
        client: Arc<AnthropicClient>,
        base_tool_executor: ToolExecutor,
    ) -> Self {
        Self {
            client,
            base_tool_executor,
            agent_definitions: HashMap::new(),
            hook_executor: None,
        }
    }

    /// Set the hook executor for lifecycle event hooks
    pub fn with_hooks(mut self, hook_executor: HookExecutor) -> Self {
        self.hook_executor = Some(hook_executor);
        self
    }

    /// Set the hook executor for lifecycle event hooks (mutable version)
    pub fn set_hook_executor(&mut self, hook_executor: HookExecutor) {
        self.hook_executor = Some(hook_executor);
    }

    /// Load agent definitions from a plugin manager
    pub fn load_agents(&mut self, agents: HashMap<String, AgentDefinition>) {
        self.agent_definitions = agents;
    }

    /// Add a single agent definition
    pub fn add_agent(&mut self, agent_def: AgentDefinition) {
        let name = agent_def.agent.name.clone();
        self.agent_definitions.insert(name, agent_def);
    }

    /// Get an agent definition by name
    pub fn get_agent(&self, name: &str) -> Option<&AgentDefinition> {
        self.agent_definitions.get(name)
    }

    /// List all available agent names
    pub fn list_agents(&self) -> Vec<String> {
        self.agent_definitions.keys().cloned().collect()
    }

    /// Spawn a new agent context
    pub fn spawn_agent(&self, agent_name: &str) -> Result<AgentContext> {
        let agent_def = self
            .agent_definitions
            .get(agent_name)
            .ok_or_else(|| AgentError::AgentNotFound(agent_name.to_string()))?;

        Ok(AgentContext::new(agent_def.agent.clone()))
    }

    /// Execute an agent with a user prompt
    ///
    /// # Arguments
    ///
    /// * `agent_name` - Name of the agent to execute
    /// * `user_prompt` - User message to send to the agent
    ///
    /// # Returns
    ///
    /// A tuple of (final response text, vector of events that occurred)
    pub async fn execute_agent(
        &self,
        agent_name: &str,
        user_prompt: impl Into<String>,
    ) -> Result<(String, Vec<AgenticEvent>)> {
        let mut context = self.spawn_agent(agent_name)?;
        context.add_user_message(user_prompt);

        self.run_agent_loop(&mut context).await
    }

    /// Run the agentic loop for a given agent context
    ///
    /// # Arguments
    ///
    /// * `context` - Mutable agent context that will be updated
    ///
    /// # Returns
    ///
    /// A tuple of (final response text, vector of events that occurred)
    pub async fn run_agent_loop(
        &self,
        context: &mut AgentContext,
    ) -> Result<(String, Vec<AgenticEvent>)> {
        // Clone what we need from agent_info before borrowing context mutably
        let agent_name = context.agent_info.name.clone();
        let max_turns = context.agent_info.behavior.max_turns;
        let stop_on_error = context.agent_info.behavior.stop_on_error;
        let allowed_tools = context.agent_info.tools.allowed.clone();
        let denied_tools = context.agent_info.tools.denied.clone();

        // Create filtered tool executor for this agent
        let filtered_executor = FilteredToolExecutor::new(
            self.base_tool_executor.clone(),
            allowed_tools,
            denied_tools,
        );

        let available_tools = filtered_executor.get_available_tools();
        let mut all_events = Vec::new();
        let mut final_response = String::new();

        // Run agentic loop with tool execution
        while !context.is_max_turns_exceeded() {
            context.increment_turn();

            tracing::debug!(
                "Agent '{}' turn {}/{}",
                agent_name,
                context.turns,
                max_turns
            );

            // Create request with filtered tools
            let request = crate::client::MessageRequest::from_conversation(
                &context.conversation,
                context.model(),
            )
            .with_tools(available_tools.clone());

            // Send request
            let response = self.client.send_message(request).await?;

            // Process response
            let mut has_tool_use = false;
            let mut tool_results = Vec::new();

            for content_block in &response.content {
                match content_block {
                    crate::client::ContentBlock::Text { text } => {
                        final_response = text.clone();
                        all_events.push(AgenticEvent::TextResponse {
                            text: text.clone(),
                        });
                    }
                    crate::client::ContentBlock::ToolUse { id, name, input } => {
                        has_tool_use = true;

                        // Record tool call event
                        all_events.push(AgenticEvent::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });

                        // Execute before_tool_call hook if available
                        if let Some(ref hook_executor) = self.hook_executor {
                            let hook_context = ToolCallContext {
                                tool_name: name.clone(),
                                tool_input: input.clone(),
                                tool_id: id.clone(),
                            };

                            if let Err(e) = hook_executor.execute_before_tool_call(&hook_context) {
                                tracing::warn!("BeforeToolCall hook failed: {}", e);
                            }
                        }

                        // Execute the tool with permission checking
                        let tool_use = crate::tools::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        };

                        let result = filtered_executor.execute(&tool_use).await;

                        // Execute after_tool_call hook if available
                        if let Some(ref hook_executor) = self.hook_executor {
                            let after_hook_context = AfterToolCallContext {
                                tool_name: name.clone(),
                                tool_input: input.clone(),
                                tool_id: id.clone(),
                                tool_output: result.content.clone(),
                                is_error: result.is_error.unwrap_or(false),
                            };

                            if let Err(e) = hook_executor.execute_after_tool_call(&after_hook_context) {
                                tracing::warn!("AfterToolCall hook failed: {}", e);
                            }
                        }

                        // Check if there was an error and stop_on_error is enabled
                        if result.is_error.unwrap_or(false) && stop_on_error {
                            all_events.push(AgenticEvent::ToolResult {
                                tool_use_id: result.tool_use_id.clone(),
                                content: result.content.clone(),
                                is_error: result.is_error,
                            });

                            return Err(AgentError::StoppedOnError(result.content.clone()));
                        }

                        // Record tool result event
                        all_events.push(AgenticEvent::ToolResult {
                            tool_use_id: result.tool_use_id.clone(),
                            content: result.content.clone(),
                            is_error: result.is_error,
                        });

                        tool_results.push(result);
                    }
                    crate::client::ContentBlock::ToolResult { .. } => {
                        // This shouldn't happen in assistant responses
                        tracing::warn!("Unexpected tool result in assistant response");
                    }
                }
            }

            // If no tool use, we're done
            if !has_tool_use {
                break;
            }

            // Add assistant message with tool use to conversation
            let assistant_message =
                crate::conversation::Message::assistant_with_blocks(response.content.clone());
            context.conversation.messages.push(assistant_message);

            // Add tool results to conversation as user message
            let tool_result_blocks: Vec<crate::client::ContentBlock> = tool_results
                .iter()
                .map(|result| crate::client::ContentBlock::from_tool_result(result))
                .collect();

            let user_message =
                crate::conversation::Message::user_with_blocks(tool_result_blocks);
            context.conversation.messages.push(user_message);
        }

        // Check if we exceeded max turns
        if context.is_max_turns_exceeded() {
            tracing::warn!(
                "Agent '{}' exceeded max turns: {}",
                agent_name,
                max_turns
            );
        }

        Ok((final_response, all_events))
    }

    /// Execute multiple agents in parallel
    ///
    /// # Arguments
    ///
    /// * `tasks` - Vector of (agent_name, user_prompt) tuples
    ///
    /// # Returns
    ///
    /// Vector of results corresponding to each task
    pub async fn execute_agents_parallel(
        &self,
        tasks: Vec<(String, String)>,
    ) -> Vec<Result<(String, Vec<AgenticEvent>)>> {
        use futures::future::join_all;

        let futures: Vec<_> = tasks
            .into_iter()
            .map(|(agent_name, user_prompt)| {
                let agent_name_owned = agent_name.clone();
                let user_prompt_owned = user_prompt.clone();
                async move { self.execute_agent(&agent_name_owned, user_prompt_owned).await }
            })
            .collect();

        join_all(futures).await
    }
}

impl Default for FilteredToolExecutor {
    fn default() -> Self {
        Self {
            base_executor: ToolExecutor::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_core::plugin::{AgentBehavior, AgentSystemPrompt, AgentTools};

    fn create_test_agent_info(name: &str) -> AgentInfo {
        AgentInfo {
            name: name.to_string(),
            description: "Test agent".to_string(),
            model: "claude-sonnet-4".to_string(),
            system_prompt: AgentSystemPrompt {
                role: "You are a test agent".to_string(),
                instructions: "Follow test instructions".to_string(),
            },
            tools: AgentTools {
                allowed: vec!["read".to_string(), "write".to_string()],
                denied: vec!["bash".to_string()],
            },
            behavior: AgentBehavior {
                max_turns: 5,
                stop_on_error: false,
            },
        }
    }

    #[test]
    fn test_agent_context_creation() {
        let agent_info = create_test_agent_info("test-agent");
        let context = AgentContext::new(agent_info.clone());

        assert_eq!(context.agent_info.name, "test-agent");
        assert_eq!(context.turns, 0);
        assert_eq!(context.max_turns(), 5);
        assert!(!context.stop_on_error());
        assert!(!context.is_max_turns_exceeded());

        // Verify system prompt was constructed
        assert!(context
            .conversation
            .system_prompt
            .as_ref()
            .unwrap()
            .contains("You are a test agent"));
        assert!(context
            .conversation
            .system_prompt
            .as_ref()
            .unwrap()
            .contains("Follow test instructions"));
    }

    #[test]
    fn test_agent_context_turn_tracking() {
        let agent_info = create_test_agent_info("test-agent");
        let mut context = AgentContext::new(agent_info);

        assert_eq!(context.turns, 0);
        assert!(!context.is_max_turns_exceeded());

        for i in 1..=5 {
            context.increment_turn();
            assert_eq!(context.turns, i);
        }

        assert!(context.is_max_turns_exceeded());
    }

    #[test]
    fn test_filtered_tool_executor_allowed_tools() {
        let base_executor = ToolExecutor::new();
        let allowed = vec!["read".to_string(), "write".to_string()];
        let denied = vec!["bash".to_string()];

        let filtered = FilteredToolExecutor::new(base_executor, allowed, denied);

        // Allowed tools should be accessible
        assert!(filtered.is_tool_allowed("read"));
        assert!(filtered.is_tool_allowed("write"));

        // Denied tools should not be accessible
        assert!(!filtered.is_tool_allowed("bash"));

        // Tools not in allowed list should not be accessible
        assert!(!filtered.is_tool_allowed("grep"));
        assert!(!filtered.is_tool_allowed("glob"));
    }

    #[test]
    fn test_filtered_tool_executor_empty_allowed_list() {
        let base_executor = ToolExecutor::new();
        let allowed = vec![]; // Empty = allow all except denied
        let denied = vec!["bash".to_string()];

        let filtered = FilteredToolExecutor::new(base_executor, allowed, denied);

        // All tools except denied should be accessible
        assert!(filtered.is_tool_allowed("read"));
        assert!(filtered.is_tool_allowed("write"));
        assert!(filtered.is_tool_allowed("grep"));
        assert!(filtered.is_tool_allowed("glob"));

        // Denied tools should not be accessible
        assert!(!filtered.is_tool_allowed("bash"));
    }

    #[test]
    fn test_filtered_tool_executor_get_available_tools() {
        let base_executor = ToolExecutor::new();
        let allowed = vec!["read".to_string(), "write".to_string()];
        let denied = vec![];

        let filtered = FilteredToolExecutor::new(base_executor, allowed, denied);
        let available_tools = filtered.get_available_tools();

        // Should only have read and write tools
        assert_eq!(available_tools.len(), 2);
        assert!(available_tools.iter().any(|t| t.name == "read"));
        assert!(available_tools.iter().any(|t| t.name == "write"));
    }

    #[test]
    fn test_agent_executor_creation() {
        let client = Arc::new(AnthropicClient::new("test_key".to_string()));
        let executor = ToolExecutor::new();

        let agent_executor = AgentExecutor::new(client, executor);

        assert_eq!(agent_executor.list_agents().len(), 0);
    }

    #[test]
    fn test_agent_executor_add_agent() {
        let client = Arc::new(AnthropicClient::new("test_key".to_string()));
        let executor = ToolExecutor::new();

        let mut agent_executor = AgentExecutor::new(client, executor);

        let agent_info = create_test_agent_info("test-agent");
        let agent_def = AgentDefinition {
            agent: agent_info,
        };

        agent_executor.add_agent(agent_def);

        assert_eq!(agent_executor.list_agents().len(), 1);
        assert!(agent_executor.get_agent("test-agent").is_some());
        assert!(agent_executor.get_agent("nonexistent").is_none());
    }

    #[test]
    fn test_agent_executor_spawn_agent() {
        let client = Arc::new(AnthropicClient::new("test_key".to_string()));
        let executor = ToolExecutor::new();

        let mut agent_executor = AgentExecutor::new(client, executor);

        let agent_info = create_test_agent_info("test-agent");
        let agent_def = AgentDefinition {
            agent: agent_info,
        };

        agent_executor.add_agent(agent_def);

        // Should successfully spawn agent
        let context = agent_executor.spawn_agent("test-agent");
        assert!(context.is_ok());

        let context = context.unwrap();
        assert_eq!(context.agent_info.name, "test-agent");
        assert_eq!(context.turns, 0);

        // Should fail for nonexistent agent
        let result = agent_executor.spawn_agent("nonexistent");
        assert!(result.is_err());
        match result {
            Err(AgentError::AgentNotFound(_)) => {}
            _ => panic!("Expected AgentNotFound error"),
        }
    }

    #[test]
    fn test_agent_executor_load_agents() {
        let client = Arc::new(AnthropicClient::new("test_key".to_string()));
        let executor = ToolExecutor::new();

        let mut agent_executor = AgentExecutor::new(client, executor);

        let mut agents = HashMap::new();
        for i in 0..3 {
            let agent_info = create_test_agent_info(&format!("agent-{}", i));
            let agent_def = AgentDefinition {
                agent: agent_info,
            };
            agents.insert(format!("agent-{}", i), agent_def);
        }

        agent_executor.load_agents(agents);

        assert_eq!(agent_executor.list_agents().len(), 3);
        assert!(agent_executor.get_agent("agent-0").is_some());
        assert!(agent_executor.get_agent("agent-1").is_some());
        assert!(agent_executor.get_agent("agent-2").is_some());
    }

    #[tokio::test]
    async fn test_filtered_tool_executor_permission_check() {
        let base_executor = ToolExecutor::new();
        let allowed = vec!["read".to_string()];
        let denied = vec!["bash".to_string()];

        let filtered = FilteredToolExecutor::new(base_executor, allowed, denied);

        // Try to execute a denied tool
        let tool_use = crate::tools::ToolUse {
            id: "test_id".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        };

        let result = filtered.execute(&tool_use).await;
        assert!(result.is_error.unwrap_or(false));
        assert!(result.content.contains("not allowed"));

        // Try to execute a tool not in allowed list
        let tool_use = crate::tools::ToolUse {
            id: "test_id".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test.txt", "content": "test"}),
        };

        let result = filtered.execute(&tool_use).await;
        assert!(result.is_error.unwrap_or(false));
        assert!(result.content.contains("not allowed"));
    }
}
