# Commit, Push, and Create PR

Automate the git workflow: create branch, commit changes, push, and open a pull request.

Steps:
1. **Review Changes**: Show git diff of all changes
2. **Create Branch**: Create a new branch with descriptive name
3. **Stage Files**: Add relevant files to staging area
4. **Write Commit Message**: Generate a clear, descriptive commit message following conventions
5. **Commit**: Create the commit
6. **Push**: Push branch to remote repository
7. **Create PR**: Open pull request with summary and description

Commit message format:
```
<type>: <subject>

<body>

<footer>
```

Types: feat, fix, docs, style, refactor, test, chore

PR description should include:
- Summary of changes
- Motivation and context
- Test plan
- Screenshots (if UI changes)
- Breaking changes (if any)

Ask for confirmation before executing each destructive operation.
