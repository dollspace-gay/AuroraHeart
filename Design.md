# AuroraHeart: Product Overview and Design Summary

## 1. Introduction and Core Philosophy

**Product Name:** AuroraHeart (RustAgent Studio)
**Target Platform:** Windows
**Core Technology:** Rust (Application Logic) + High-Performance Rust GUI Framework (Slint/Iced/Floem)
**Goal:** To build the fastest, most resource-efficient IDE on Windows that treats AI coding agents not as a plugin, but as a **first-class co-author**. The design prioritizes performance, security, and a "human-in-the-loop" workflow.

---

## 2. Core Architecture and Technology Stack

### A. Performance Foundation (Rust)
* **Architecture:** Split into a highly responsive **UI Layer** and a concurrent, asynchronous **AI Agent Core** (leveraging Tokio for non-blocking I/O).
* **Performance:** Near-instantaneous startup time and zero typing latency. Minimal memory footprint achieved through Rust's efficiency.
* **Text Component:** Custom, high-speed text rendering engine to handle large codebases and complex AI diffs efficiently.

### B. Security and Setup
* **Authentication:** The end-user only enters their preferred AI API key once. The key is stored securely using the **Windows Credential Manager** and is only ever accessible by the isolated **AI Agent Core**.
* **Local-First Option:** Designed to support local LLMs where possible to keep sensitive code off the cloud.

---

## 3. First-Class AI Agent Integration (The Agent Core)

The following features differentiate AuroraHeart by giving the AI agent comprehensive access to context and task management.

### A. Modular Agent Directives (Dynamic Prompting)
The agent's personality and rules are dynamically assembled based on the user's project and selected options.

* **Customizable Rules:** Instead of a single static prompt, the final system prompt is built from modular `.md` files stored in a configuration directory (`.AuroraHeart/directives/`).
* **Language-Specific Directives:** Checkboxes in the IDE UI (e.g., "Rust Security," "Python Performance") allow the user to instantly append language-specific rules, security practices, and quality standards to the agent's instructions, ensuring context-aware and high-quality code generation.

Modular Agent Directives Architecture
Instead of having one static claude/agents.md file, the AI Agent Core will dynamically assemble the final, full prompt based on the user's selections and the project context.

A. Directory Structure
You will store modular prompt segments in a structured directory within the IDE's configuration space (e.g., ~/.AuroraHeart/directives/).

.AuroraHeart/
└── directives/
    ├── base/
    │   ├── 01_role.md          # (e.g., "You are a meticulous co-pilot...")
    │   └── 02_tools.md         # (Definition of bd_query_issues, rag_retrieve_context, etc.)
    └── language/
        ├── rust/
        │   ├── quality.md      # (e.g., "Enforce ownership, zero-cost abstractions...")
        │   └── security.md     # (e.g., "Avoid 'unsafe' blocks, sanitize inputs...")
        └── python/
            ├── quality.md
            └── security.md
B. Dynamic Prompt Assembly
Base Prompt: The AI Agent Core always loads directives from the base/ directory, which defines the agent's identity and core tool usage.

Language Check: The IDE detects the project's primary language (e.g., by scanning Cargo.toml for Rust, requirements.txt for Python).

User Selection: The IDE UI presents a "Quality Directives" panel for the active language, with checkboxes corresponding to the available .md files (e.g., "Security Best Practices," "High Performance Optimization," "Linting/Style").

Final Prompt Construction: The Agent Core concatenates the following components in order to form the final system prompt sent to Claude:

Base Role and Identity (base/01_role.md)

Selected Language-Specific Directives (language/rust/quality.md, etc.)

Tool Definitions (base/02_tools.md)

Current Task/User Input

🛠️ II. Example Language-Specific Directive Content
This content is much more precise than a general prompt and significantly improves the quality of AI-generated code.

1. directives/language/rust/security.md
Markdown

### Security Directive: Rust Best Practices

You must adhere to the following security guidelines in all Rust code generation and refactoring:
1.  **Avoid `unsafe`:** Absolutely minimize the use of `unsafe` blocks. If one is required, you must include a detailed comment explaining *why* it is sound and cannot be avoided.
2.  **Input Sanitation:** Use crates like `serde` for robust deserialization, and ensure all external input (arguments, network data) is validated and sanitized before use.
3.  **Error Handling:** Employ `Result` and the `?` operator for exhaustive error handling. Never use `unwrap()` or `expect()` in production code paths; use proper error types.
4.  **Dependency Auditing:** When suggesting new crates, prefer widely-used, actively maintained, and audited crates (e.g., `tokio`, `hyper`, `ring`).
2. directives/language/rust/quality.md
Markdown

### Quality Directive: High-Performance Rust

Focus on idiomatic, high-performance, and maintainable Rust code:
1.  **Ownership and Borrowing:** Strictly follow Rust's ownership rules. Favor borrowing (`&` or `&mut`) over unnecessary cloning.
2.  **Zero-Cost Abstractions:** Prefer standard library types and patterns (e.g., iterators, `match` statements) over complex custom macros where a simple alternative exists.
3.  **Concurrency:** Use `Arc` and `Mutex` or `RwLock` appropriately for shared mutable state. Leverage `tokio` or `async-std` for non-blocking asynchronous operations where I/O is involved.
4.  **Code Style:** Adhere to `rustfmt` standards (4 spaces, standard naming conventions).


### B. RAG System for Long-Term Memory
A built-in Retrieval-Augmented Generation (RAG) system provides the agent with persistent, grounded context.

* **Knowledge Sources:** The RAG indexes source code, documentation, and (crucially) the history of the Issues Tracker.
* **Vector Database:** An embedded, fast vector store (Rust-native) stores embeddings, allowing the agent to retrieve the most relevant code snippets, architectural notes, and past solutions for any given task.

### C. Automatic Context Compression
Optimized for token efficiency and reduced latency/cost, this system manages the prompt context dynamically.

* **Conversational Compaction:** The AI Agent Core automatically summarizes long conversational histories when approaching the model's token limit, retaining key decisions while freeing up context space.
* **Just-in-Time Retrieval:** Instead of sending the full content of large files, the IDE provides compressed views or semantic chunks, forcing the AI to retrieve full files only when absolutely necessary.

---

## 4. Built-in Issues Tracker (Beads/BD) https://github.com/steveyegge/beads

A simple, integrated task management system that doubles as the AI's primary input/output stream for structured work.

* **Source of Truth:** Issues are stored as structured text (e.g., JSONL) directly in the project repository (`.agent_data/issues.jsonl`), enabling Git to handle versioning and merging.
* **Fast Access:** A local SQLite cache provides the AI Agent Core with instant, low-latency access to the issue database.
* **AI Tools:** The agent is given explicit tools (`bd_query_issues`, `bd_track_issue`, `bd_update_issue`) and is instructed to **always** use them to manage its tasks, turning unstructured problems into trackable, actionable items.

---

## 5. User Experience and Co-Authorship

* **Transparent Diff/Merge:** For multi-file or extensive AI-generated changes, the IDE presents a clear side-by-side or unified diff UI, allowing the developer to accept, reject, or modify every line proposed by the agent.
* **AI Agent Panel:** A dedicated, persistent panel provides the conversational chat interface, an **Agent History View** (a non-linear tree showing all task attempts and edits), and a log of all internal tool use.
* **Integrated Debugging and Testing:** AI agents are empowered to use internal tools like `tool_run_tests` and `tool_format_code` as part of their workflow, ensuring proposed changes are validated before being presented to the developer.

---

## 6. Technology Decisions (Confirmed)

Based on project requirements and long-term goals:

| Component | Technology Choice | Rationale |
|-----------|------------------|-----------|
| **GUI Framework** | Slint | Mature, declarative UI, excellent Windows support |
| **AI Provider** | Anthropic Claude (via API) | Feature parity with Claude Code VSCode plugin |
| **Text Editor (Phase 1)** | Slint TextEdit | Quick start with built-in component |
| **Text Editor (Phase 2+)** | lapce-core or xi-rope | Battle-tested, high-performance rope-based editing |
| **HTTP Client** | reqwest | Industry standard, async support, streaming |
| **Async Runtime** | tokio | De facto standard for async Rust |
| **Vector DB (RAG)** | qdrant-client or lance | Rust-native, actively maintained |
| **Encryption** | aes-gcm + ring | Secure, well-audited cryptography |
| **Diff Engine** | similar | Fast, accurate text diffs |
| **Syntax Highlighting** | tree-sitter or syntect | tree-sitter for accuracy, syntect for simplicity |
| **BD Integration** | CLI first, SQLite later | Fast to implement, optimize later |
| **Error Handling** | thiserror + anyhow | Idiomatic Rust error patterns |
| **Credential Storage** | Encrypted local files first | Windows Credential Manager in Phase 7 |
| **Config Location** | Per-project `.AuroraHeart/` | Project-specific configuration |

**Project Structure:** Workspace with multiple crates (`aurora-core`, `aurora-ui`, `aurora-agent`, `aurora-rag`, etc.)

---

## 7. Implementation Roadmap (Phased Approach)

### **Phase 1: Foundation & Basic UI** (Weeks 1-3)

**Goal**: Get a working Slint window with basic text editing and chat interface

**Deliverables**:
- Workspace structure (`aurora-core`, `aurora-ui`, `aurora-agent`)
- Slint UI with:
  - Main text editor pane (using Slint's TextEdit)
  - AI chat panel (side-by-side or bottom pane)
  - File tree view (basic)
- Basic project configuration system (`.AuroraHeart/config.toml`)
- Encrypted API key storage (using `aes-gcm` + local file)
- Simple file I/O (open/save files)

**Success Criteria**:
- Can open AuroraHeart, see a text editor and chat panel
- Can configure Claude API key
- Can type in both panes

---

### **Phase 2: AI Agent Core (Claude Code Parity)** (Weeks 4-8)

**Goal**: Implement the AI agent with tool-calling capabilities like Claude Code

**Deliverables**:
- **Agent Core** (`aurora-agent` crate):
  - Anthropic API client (using `reqwest` + streaming)
  - Tool system implementing Claude Code's tools:
    - `Read`: Read file contents
    - `Write`: Write new files
    - `Edit`: Make precise edits to existing files
    - `Bash`: Execute shell commands
    - `Grep`: Search code for patterns
    - `Glob`: Find files by pattern
    - `Task`: Spawn sub-agents for complex tasks
  - Conversation management (system prompt + history)
  - Token counting and context management

- **UI Integration**:
  - Chat interface for conversing with Claude
  - Tool execution visualization (show when agent reads/writes files)
  - Streaming responses from Claude

- **BD Integration (CLI approach)**:
  - Subprocess execution of `bd` commands
  - Parse JSON output from `bd list --json`, `bd show --json`, etc.
  - UI panel showing current issues

**Success Criteria**:
- Can chat with Claude about code
- Claude can read, write, and edit files in the project
- Claude can execute bash commands (with approval)
- Claude can search codebase with grep/glob
- BD issues visible in UI

---

### **Phase 3: Diff/Merge & File Operations** (Weeks 9-11)

**Goal**: Implement the sophisticated diff UI that Claude Code has

**Deliverables**:
- **Diff Engine**:
  - Side-by-side or unified diff view (using `similar` crate)
  - Accept/reject individual changes
  - Multi-file diff view when Claude modifies multiple files

- **File Watcher**:
  - Detect external file changes
  - Reload files automatically

- **Improved Text Editor**:
  - Syntax highlighting (using `tree-sitter` or `syntect`)
  - Line numbers
  - Basic editor features (undo/redo, search/replace)

**Success Criteria**:
- When Claude suggests file changes, see clear diff view
- Can accept/reject changes with buttons
- Syntax highlighting works for major languages (Rust, Python, JS, etc.)

---

### **Phase 4: Modular Directives System** (Weeks 12-14)

**Goal**: Implement the dynamic prompt assembly system from Section 3.A

**Deliverables**:
- **Directive System**:
  - Load directives from `.AuroraHeart/directives/` structure
  - Base directives (role, tools)
  - Language-specific directives (Rust, Python, etc.)
  - Auto-detect project language (scan for `Cargo.toml`, `package.json`, etc.)

- **UI Controls**:
  - Settings panel to enable/disable directive modules
  - Checkboxes for "Security Best Practices", "Performance Optimization", etc.
  - Preview of final assembled prompt

- **Directive Templates**:
  - Ship with default directives for Rust, Python, TypeScript, Go
  - User can add custom directives

**Success Criteria**:
- Can toggle different directive modules
- Agent behavior changes based on selected directives
- Project language auto-detected

---

### **Phase 5: Advanced Context & RAG** (Weeks 15-20)

**Goal**: Implement the RAG system and context compression from Section 3.B and 3.C

**Deliverables**:
- **RAG System** (`aurora-rag` crate):
  - Vector database integration (qdrant or lance)
  - Embedding generation (using fastembed or Anthropic's API)
  - Index codebase, docs, and BD issue history
  - Semantic search for relevant context

- **Context Compression**:
  - Automatic conversation summarization when approaching token limits
  - Smart chunking of large files
  - "Just-in-time" retrieval (don't send full files unless needed)

- **Memory System**:
  - Store and retrieve past solutions
  - "Remember" architectural decisions

**Success Criteria**:
- Can ask Claude about any part of large codebase
- Claude retrieves relevant context automatically
- Long conversations don't hit token limits

---

### **Phase 6: Polish & Advanced Features** (Weeks 21-24)

**Goal**: Match remaining Claude Code features and add AuroraHeart-specific polish

**Deliverables**:
- **Agent History Tree**:
  - Visual tree showing all task attempts
  - Branching for different approaches
  - Can rewind and try different paths

- **Integrated Testing**:
  - Agent can run tests (`tool_run_tests`)
  - Agent can format code (`tool_format_code`)
  - Show test results in UI

- **Git Integration**:
  - Stage/commit/push from UI
  - Agent can create commits (with approval)
  - Show git status in file tree

- **Performance Optimization**:
  - Consider upgrading to `lapce-core` for text editing
  - Optimize Slint rendering
  - Profile and optimize hot paths

- **BD Library Integration**:
  - Replace subprocess BD calls with direct SQLite access
  - Faster issue queries
  - Real-time updates

**Success Criteria**:
- Feature parity with Claude Code VSCode plugin
- Snappy, responsive UI (sub-16ms frame times)
- All tests passing

---

### **Phase 7: Windows-Specific Polish** (Weeks 25+)

**Goal**: Make AuroraHeart feel native to Windows

**Deliverables**:
- Windows Credential Manager integration
- Windows installer/updater
- System tray integration
- Windows notifications
- File associations (.rs, .py, etc. open in AuroraHeart)
- Windows-specific keyboard shortcuts
- High-DPI display support

**Success Criteria**:
- Seamless Windows integration
- Professional installer experience
- Windows users feel "at home"

---

## 8. Workspace Structure

```
AuroraHeart/
├── Cargo.toml                  # Workspace root
├── .beads/                     # BD issue tracker database
├── design.md                   # This document
├── claude.md                   # Development guidelines
├── README.md
├── .AuroraHeart/              # Template config for projects using AuroraHeart
│   ├── config.toml            # Default configuration
│   └── directives/
│       ├── base/
│       │   ├── 01_role.md
│       │   └── 02_tools.md
│       └── language/
│           ├── rust/
│           │   ├── quality.md
│           │   └── security.md
│           ├── python/
│           │   ├── quality.md
│           │   └── security.md
│           ├── typescript/
│           │   └── quality.md
│           └── go/
│               └── quality.md
├── crates/
│   ├── aurora-core/           # Core types, config, utilities
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs      # Configuration management
│   │       ├── crypto.rs      # Encrypted credential storage
│   │       └── types.rs       # Shared types
│   ├── aurora-ui/             # Slint UI and main application
│   │   ├── Cargo.toml
│   │   ├── build.rs           # Slint build script
│   │   ├── ui/                # Slint files
│   │   │   ├── main.slint
│   │   │   ├── editor.slint
│   │   │   └── chat.slint
│   │   └── src/
│   │       ├── main.rs
│   │       ├── editor.rs
│   │       └── file_tree.rs
│   ├── aurora-agent/          # AI agent core
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs      # Anthropic API client
│   │       ├── tools.rs       # Tool system (Read, Write, Edit, etc.)
│   │       ├── conversation.rs
│   │       └── directives.rs  # Directive loading system
│   ├── aurora-rag/            # RAG system (Phase 5)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── indexer.rs
│   │       ├── embeddings.rs
│   │       └── retrieval.rs
│   ├── aurora-editor/         # Advanced text editor (Phase 3+)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rope.rs
│   │       ├── syntax.rs
│   │       └── diff.rs
│   └── aurora-bd/             # BD integration (Phase 6)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── cli.rs         # Subprocess BD calls
│           └── db.rs          # Direct SQLite access
└── tests/
    └── integration/
        ├── test_agent_tools.rs
        └── test_ui_workflow.rs
```

---

## 9. Testing Strategy

Following Test-Driven Development (TDD) principles:

### Unit Tests
- Write tests alongside each module implementation
- Test all public APIs and major internal functions
- Use Rust's built-in `#[cfg(test)]` modules

### Integration Tests
- Agent tool execution (Read, Write, Edit, etc.)
- UI interactions and state management
- Configuration loading and encryption
- BD integration

### Property Tests
- Rope operations (using `proptest`)
- Diff algorithm correctness
- Encryption/decryption round-trips

### Snapshot Tests
- UI component rendering (using `insta`)
- Directive assembly outputs
- Diff generation

### End-to-End Tests
- Full workflows:
  - Create project → configure API key → ask Claude → accept changes
  - Multi-file refactoring
  - BD issue creation and tracking

### Coverage Goals
- Aim for >80% code coverage
- 100% coverage for critical paths (crypto, agent tools)

---

## 10. Development Workflow

### Issue Tracking
- All work tracked in BD (Beads issue tracker)
- Epic → Sub-issues structure
- Use `bd ready` to find unblocked work
- Close issues only when tests pass

### Code Quality
- No `todo!()` or `unimplemented!()` in production code
- Zero compiler warnings
- Zero clippy warnings
- Formatted with `rustfmt`
- All public APIs documented

### Git Workflow
- Manual commits by developer
- Descriptive commit messages
- Reference BD issues in commits (e.g., "Implements aurora-1: Setup workspace")

### Continuous Integration (Future)
- Run tests on every commit
- Check formatting and clippy
- Measure code coverage
- Build for Windows target


