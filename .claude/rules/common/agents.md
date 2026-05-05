# Agent Orchestration

## Available Agents

Located in `~/.claude/agents/`:

| Agent | Purpose | When to Use |
|-------|---------|-------------|
| planner | Implementation planning | Complex features, refactoring |
| architect | System design | Architectural decisions |
| tdd-guide | Test-driven development | New features, bug fixes |
| code-reviewer | 統合レビュー(失敗パターン+ドメイン+品質) | After writing code |
| build-error-resolver | Fix build errors | When build fails |
| refactor-cleaner | Dead code cleanup | Code maintenance |
| doc-updater | Documentation | Updating docs |

## Immediate Agent Usage

No user prompt needed:
1. Complex feature requests - Use **planner** agent
2. **Code just written/modified** - Use **code-reviewer** agent (失敗パターン+ドメイン知識+品質の3軸統合レビュー)
3. Bug fix or new feature - Use **tdd-guide** agent
4. Architectural decision - Use **architect** agent

### code-reviewer 自動呼び出し条件
以下の場合、ユーザーの指示なしで code-reviewer エージェントを起動する:
- Rust/TypeScript/CSS のコードを新規作成または修正した後
- レビュー結果の CRITICAL/HIGH はユーザーに報告し、修正してからコミットする

## Parallel Task Execution

ALWAYS use parallel Task execution for independent operations:

```markdown
# GOOD: Parallel execution
Launch 3 agents in parallel:
1. Agent 1: Security analysis of auth module
2. Agent 2: Performance review of cache system
3. Agent 3: Type checking of utilities

# BAD: Sequential when unnecessary
First agent 1, then agent 2, then agent 3
```

## Multi-Perspective Analysis

For complex problems, use split role sub-agents:
- Factual reviewer
- Senior engineer
- Security expert
- Consistency reviewer
- Redundancy checker
