#!/usr/bin/env bash
# scripts/review.sh — PR/file code review via ccd (DeepSeek Anthropic-compatible endpoint)
#
# Usage:
#   scripts/review.sh <PR_NUMBER>           Review a GitHub PR
#   scripts/review.sh <file...>             Review local file(s)
#
# Uses the same DeepSeek provider config as `ccd` zsh function.
# Review output goes to stdout and is saved to .claude/reviews/.

set -euo pipefail

DEEPSEEK_ENV="$HOME/.config/claude-providers/deepseek-anthropic.zsh"
REVIEW_DIR=".claude/reviews"
MAX_PROMPT_BYTES=200000

if [ ! -f "$DEEPSEEK_ENV" ]; then
    echo "error: DeepSeek env file not found at $DEEPSEEK_ENV" >&2
    exit 2
fi

mkdir -p "$REVIEW_DIR"

if [ "$#" -lt 1 ]; then
    echo "usage: $0 <PR_NUMBER>" >&2
    echo "       $0 <file> [file...]" >&2
    exit 2
fi

INPUT="$1"

# ── Helpers ──

run_review() {
    local prompt_file="$1"
    local output_file="$2"
    local prompt_size
    prompt_size=$(wc -c < "$prompt_file")

    echo "Running ccd review (prompt: $prompt_size bytes)..." >&2

    if [ "$prompt_size" -gt "$MAX_PROMPT_BYTES" ]; then
        echo "warning: prompt is $prompt_size bytes, may hit ARG_MAX; truncating diff" >&2
        # head -c on the prompt file to stay under limit, keeping the structure
        head -c "$MAX_PROMPT_BYTES" "$prompt_file" > "${prompt_file}.truncated"
        prompt_file="${prompt_file}.truncated"
    fi

    # Replicate ccd logic: source DeepSeek env, then claude --print
    (
        set +e
        # shellcheck disable=SC1090
        source "$DEEPSEEK_ENV"
        claude --dangerously-skip-permissions -p "$(cat "$prompt_file")" | tee "$output_file"
    )
    echo "Review saved to $output_file" >&2
}

# ── PR review ──

if [[ "$INPUT" =~ ^[0-9]+$ ]]; then
    PR="$INPUT"
    OWNER_REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner)"

    echo "Fetching PR #$PR from $OWNER_REPO..." >&2

    PR_TITLE="$(gh pr view "$PR" --json title -q .title)"
    PR_BODY="$(gh pr view "$PR" --json body -q .body)"
    PR_BRANCH="$(gh pr view "$PR" --json headRefName -q .headRefName)"
    PR_BASE="$(gh pr view "$PR" --json baseRefName -q .baseRefName)"
    PR_DIFF="$(gh pr diff "$PR")"
    # gh pr diff 不支持 --stat；用 gh pr view 的 files JSON 拼出 stat-like 列表。
    PR_FILES="$(gh pr view "$PR" --json files --jq '.files[] | "\(.path)  +\(.additions) -\(.deletions)"')"

    PROMPT_FILE="$REVIEW_DIR/PR-${PR}-prompt.md"
    OUTPUT_FILE="$REVIEW_DIR/PR-${PR}-review.md"

    # Build prompt. Static parts use quoted heredoc (no expansion).
    # Dynamic parts use printf %s to avoid shell interpretation of backticks/$() in diff.

    cat > "$PROMPT_FILE" << 'HEADER'
你是一个严格的代码审查者。请对以下 Pull Request 进行彻底的代码审查。

HEADER

    printf '\n## PR #%s: %s\n' "$PR" "$PR_TITLE" >> "$PROMPT_FILE"
    printf '**分支:** %s → %s\n' "$PR_BRANCH" "$PR_BASE" >> "$PROMPT_FILE"
    printf '**仓库:** %s\n\n' "$OWNER_REPO" >> "$PROMPT_FILE"
    printf '### 描述\n%s\n\n' "$PR_BODY" >> "$PROMPT_FILE"
    printf '### 变更文件\n%s\n\n' "$PR_FILES" >> "$PROMPT_FILE"

    printf '### Diff\n```diff\n' >> "$PROMPT_FILE"
    printf '%s\n' "$PR_DIFF" >> "$PROMPT_FILE"
    printf '```\n\n' >> "$PROMPT_FILE"

    cat >> "$PROMPT_FILE" << 'FOOTER'
## 审查要求

对每个发现的问题，请包含：
- **严重程度**: critical / major / minor / nit
- **文件与行号**: 精确定位
- **问题描述**: 什么有问题，为什么
- **修复建议**: 具体怎么改

覆盖以下方面：
1. Bug、逻辑错误、边界情况
2. 安全漏洞（注入、认证、数据暴露）
3. 性能问题（N+1查询、内存泄漏、不必要的分配）
4. 代码清晰度、命名、结构，是否符合项目约定（参考 CLAUDE.md）
5. 测试覆盖缺口
6. 架构与设计问题

按严重程度组织发现。最后给出关键问题总结和整体评估。

用中文回复。
FOOTER

    run_review "$PROMPT_FILE" "$OUTPUT_FILE"

# ── File review ──

else
    FILES=("$@")
    TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
    PROMPT_FILE="$REVIEW_DIR/files-${TIMESTAMP}-prompt.md"
    OUTPUT_FILE="$REVIEW_DIR/files-${TIMESTAMP}-review.md"

    {
        printf '你是一个严格的代码审查者。请审查以下文件。\n\n'
        for f in "${FILES[@]}"; do
            if [ -f "$f" ]; then
                printf '## %s\n\n```\n' "$f"
                cat "$f"
                printf '\n```\n\n'
            else
                printf '## %s (FILE NOT FOUND)\n\n' "$f"
            fi
        done

        cat << 'EOF'
## 审查要求

对每个问题包含严重程度（critical/major/minor/nit）、文件与行号、问题描述、修复建议。
覆盖 bug、安全、性能、代码清晰度、测试覆盖。
用中文回复。
EOF
    } > "$PROMPT_FILE"

    run_review "$PROMPT_FILE" "$OUTPUT_FILE"
fi
