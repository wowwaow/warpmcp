# MF_MCP

# Warp MCP Tasks Server

A Rust-based Model Context Protocol (MCP) server designed to coordinate Warp terminal agents with enforced Trello-based task management, RAG-capable persistent memory, and heartbeat-based agent coordination.

## Key Features

### 1. Enforced Trello Workflow
- Agents MUST work exclusively through Trello tasks
- Automatic task assignment and progress tracking
- Required frequent updates via comments, checklists, and descriptions
- Linear task progression with list-based workflow

### 2. RAG-Capable Persistent Memory
- RedisJSON-based knowledge storage with semantic search
- Categories: task_progress, api_docs, code_patterns, errors, solutions, project_knowledge
- Multi-dimensional indexing for efficient retrieval
- Agent learning from collective experiences
- 7-day retention for all stored knowledge

### 3. Agent Coordination
- Mandatory heartbeat system (30-second intervals)
- Collision prevention through task assignment tracking
- Real-time visibility of all active agents
- Automatic cleanup of stale assignments

## Prerequisites

- Rust 1.70 or later
- Redis with RedisJSON module
- Trello API credentials
- Warp terminal (Preview version with MCP support)

## Environment Variables

```bash
# Required
TRELLO_KEY=your_trello_api_key
TRELLO_TOKEN=your_trello_token
TRELLO_BOARD_ID=your_board_id

# Optional
REDIS_URL=redis://127.0.0.1:6379  # Default
HEARTBEAT_TIMEOUT=120              # Default: 120 seconds
