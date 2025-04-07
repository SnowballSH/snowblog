# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Build Commands

- `deno task dev`: Start development server
- `deno task build`: Build for production
- `deno task preview`: Preview production build
- `deno task astro`: Run Astro CLI commands

## Code Style Guidelines

- **Components**: Use PascalCase for Astro component files (.astro)
- **Variables/Functions**: Use camelCase for variables and functions
- **Types**: Define interfaces for component props and data structures
- **Imports**: Use relative paths for components, direct imports for packages
- **Error Handling**: Use try/catch for async operations and provide fallbacks
- **Formatting**: Use `deno fmt` for code formatting
- **Structure**: Place TypeScript interfaces at the top of component files
- **Props**: Destructure from Astro.props with proper type annotations

## Tech Stack

- Astro v5.6.1 with TypeScript
- Deno for runtime and package management (deno.lock)
- Markdown: marked, easymde for editor
- Styling: CSS modules with global styles in styles/global.css
