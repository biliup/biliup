# Repository agent instructions

## Personal agent delegation

The primary model is gpt-6.1-sol. Delegate suitable independent work to these configured custom agents when it materially helps the task:

- `6.1-sol-xhigh`: gpt-6.1-sol with xhigh reasoning, for complex implementation, difficult debugging, multi-file refactoring, and correctness review.
- `6-luna-max`: gpt-6-luna with high reasoning, for targeted search, log analysis, documentation, test execution, and narrowly scoped fixes.

The primary agent owns planning, task boundaries, integration, verification, and the final answer. Give subagents explicit scope and a concise expected result; avoid concurrent edits to the same files. Use the lighter agent for straightforward work and the stronger agent for ambiguity. Do not spawn agents for trivial tasks. Repository instructions and explicit user directions take precedence.

## Code editing

- Prefer the native `apply_patch` tool for ordinary source edits and small, reviewable changes.
- Use Python only for genuinely necessary programmatic or bulk transformations; do not use it to work around an available patch tool.
- If `apply_patch` is unavailable or fails, diagnose and report the cause before choosing another edit path.

## Documentation and smoke tests

- Do not add new documentation files unless the user explicitly requests them. This applies to `docs/` and other directories, including development notes, implementation summaries, and validation reports. Report routine progress and verification results in the conversation.
- Do not add new smoke tests or dedicated smoke-test workflows unless the user explicitly requests them. Use appropriate existing checks and focused regression tests for implementation changes.
- Keep `CLAUDE.md` as a relative symbolic link to this `AGENTS.md`, so both tools use the same repository instructions.
