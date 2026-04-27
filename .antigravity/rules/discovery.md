# Discovery Protocol (DP-01)

This protocol defines the standard procedure for exploring the Ferruginous codebase using semantic search and static analysis.

## 1. Search Hierarchy
When exploring the codebase to understand a feature or investigate a bug, the following hierarchy MUST be followed:

1.  **Semantic Search (`ccc`)**: Use `ccc search "<query>"` for high-level discovery and to find conceptually related code (e.g., "font mapping," "refinement depth").
2.  **Greplike Search (`grep_search`)**: Use literal grep for pinpointing specific identifiers, trait names, or error strings identified by semantic search.
3.  **AST Exploration (`view_file`)**: Read the implementation files directly to verify structural details and logic.

## 2. Maintaining the Index
- The `cocoindex-code` daemon is responsible for background indexing.
- If search results seem stale or if significant structural changes have been made (e.g., massive file moves), manually run `ccc index` to synchronize the vector database.
- Use `ccc doctor` to verify embedding engine health if search results return empty or error out.

## 3. Reporting Findings
When reporting discovery results to the user or recording them in the `implementation_plan.md`:
- Indicate if semantic search was used to discover a pattern.
- Record any conceptual links found that were not obvious from file names alone.
