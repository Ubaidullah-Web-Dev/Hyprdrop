# Hyprdrop (`hyprdrop`) — Master Architecture & Implementation Specification

You are a Principal Linux Systems Architect, Wayland/Hyprland Protocol Engineer, and Rust GUI Specialist. Build, test, and package **Hyprdrop** (`hyprdrop`) — an ultra-lightweight, blazingly fast, production-grade radial drag-and-drop file converter, quick-action studio editor, and floating drop-shelf built for **Arch Linux** and **Hyprland (Wayland)**, ready for the **AUR**.

## 0. SKILLS, SUBAGENTS & MCP SETUP (EXECUTE FIRST)

1. **Activate Installed Global Skills**:
   Read and apply these installed skills from `~/.agents/skills/`:
   - `taste-design` & `design-md`: Enforce high-craft industrial UI design, tactile surfaces, crisp typography, and zero AI-slop colors.
   - `superpowers` & `agent-skills`: Follow spec-first planning, modular architecture, strict verification, and test-driven development.

2. **Create Project MCP Configuration (`.agents/mcp_config.json`)**:
   ```json
   {
     "mcpServers": {
       "filesystem": {
         "command": "npx",
         "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
       },
       "git": {
         "command": "uvx",
         "args": ["mcp-server-git", "--repository", "."]
       },
       "sequential-thinking": {
         "command": "npx",
         "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"]
       }
     }
   }
