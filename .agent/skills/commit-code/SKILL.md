---
name: commit-code
description: Consistent Conventional Commits pattern for closing out a milestone — one clean, well-scoped commit (or a short atomic sequence) referencing the EXECUTION.md milestone, only after verification has passed.
---

## When to use this

At the end of every milestone, after its verification skill(s) have passed and the `milestone-report` summary has been produced — never before. A commit is the last step of a milestone, not a running checkpoint mid-implementation.

## Commit types

Use exactly one of these as the commit type. If a change genuinely needs two, split into two commits rather than picking one type and burying the other change in the body.

| Type       | Use for                                                                                                            |
| ---------- | ------------------------------------------------------------------------------------------------------------------ |
| `feat`     | A new capability — a new endpoint, a new UI feature, a new schema field that adds behavior.                        |
| `fix`      | Correcting broken or wrong behavior (including a bug caught during a verify skill run).                            |
| `refactor` | Restructuring code with no behavior change (e.g. moving Prisma calls out of a controller into a service).          |
| `test`     | Adding or changing tests only.                                                                                     |
| `chore`    | Tooling, config, dependency bumps, `.env.example` updates, workspace scaffolding — no application behavior change. |
| `docs`     | PRD/EXECUTION/README/SKILL.md edits, code comments.                                                                |
| `style`    | Formatting-only changes (whitespace, import order) with no logic change.                                           |
| `build`    | Dockerfile, Nx config, package.json build scripts.                                                                 |
| `perf`     | A change made specifically for performance, with a stated before/after.                                            |

## Scope

Use the affected project/lib as the scope, matching the Nx project names: `choir-api`, `choir-client`, `shared-types`, `shared-validation`, `prisma`, `docker`, `deps`, or `workspace` for root-level scaffolding changes. Omit scope only for changes that genuinely span everything (e.g. the initial workspace commit).

## Message format

```bash
<type>(<scope>): <short imperative summary, no period, under ~70 chars>

<optional body — what and why, wrapped at ~72 chars>

Milestone: EXECUTION.md #<N> — <milestone name>
```

Examples:

```bash
feat(choir-api): add song CRUD endpoints with role guards

Implements POST/PATCH/DELETE /songs and PUT /songs/:id/parts per
PRD §7, guarded by requireAuth + requireRole(DIRECTOR).

Milestone: EXECUTION.md #4 — Core API (Songs, Parts, Links)

chore(workspace): scaffold Nx workspace with choir-api and choir-client

Generated via create-nx-workspace + @nx/express and @nx/next
generators. No application code yet.

Milestone: EXECUTION.md #0 — Workspace Scaffold
```

## Procedure

1. Confirm the milestone's verification skill(s) already passed in this session — do not commit on the strength of "it should work."
2. Run `npx nx affected -t lint,test` (or `run-many --all` if this milestone touched shared libs) as a pre-commit gate. A commit does not happen if this fails.
3. Review `git status` / `git diff --stat` before staging. Stage only files that belong to this milestone — no stray files left over from exploration, no `dist/`, `node_modules/`, or generated Prisma client output.
4. Grep the staged diff for anything that looks like a real secret (`git diff --cached | grep -iE "SECRET|PASSWORD|postgresql://.*:.*@"`) — if anything matches, unstage it and fix `.gitignore` before proceeding. This backs the `env-config-audit` and rules.md secrets rule at the commit boundary, which is the last line of defense before it's in history.
5. If the milestone's changes are naturally more than one concern (e.g. a dependency bump _and_ a new feature), split into separate commits in dependency order (`chore` before `feat` that needs it) rather than one commit mixing types.
6. Write the commit message per the format above, including the `Milestone:` footer — this is what makes `git log` a readable audit trail against `EXECUTION.md` later.
7. Never `commit --amend` or force-push a commit that's already been pushed/shared, without explicit instruction to do so in that turn.
8. Report the resulting commit hash(es) and message(s) as part of the milestone-report summary's "What I built" section.

## Definition of done

- Working tree is clean after commit (nothing relevant left unstaged).
- Commit message follows the type(scope): summary format with a Milestone footer.
- Pre-commit lint/test gate passed.
- No secrets in the diff.

## Common pitfalls

- Committing mid-milestone "as a checkpoint" before verification has actually run — a commit is a claim that this state works, not a save button.
- One giant commit spanning `feat` + `chore` + `docs` with a vague message like "milestone 4 done" — makes `git bisect` and review useless later.
- Committing `choir-api/.env.local` because it wasn't caught by `.gitignore` yet — always worth a first-commit check that `.env*` (except `.env.example`) is actually ignored.
- Skipping the `Milestone:` footer, which is what lets you or the agent later answer "which commit was EXECUTION.md #4?" without guessing.
