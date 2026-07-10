# Contributing to Sakko

Thanks for your interest in contributing!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/Sakko.git`
3. Install dependencies: `cd Build && npm install`
4. Create a feature branch: `git checkout -b feat/your-feature`

## Development

```bash
cd Build
npm run dev      # watch mode
npm test         # run tests
npm run lint     # check lint
npm run typecheck  # type-check
```

## Pull Requests

- Keep changes focused. One feature or fix per PR.
- Add tests for new functionality
- Ensure all checks pass (test, lint, typecheck, build)
- Update CHANGELOG.md if applicable

## Code Style

- ESLint and Prettier are configured. Run `npm run make-pretty` before committing.
- Follow the existing patterns in the codebase
- Avoid adding comments unless necessary for clarity

## Reporting Issues

Use the GitHub issue tracker with the appropriate template.
