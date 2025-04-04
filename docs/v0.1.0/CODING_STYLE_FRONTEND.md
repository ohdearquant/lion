# LionForge Frontend Coding Style Guide (React + TypeScript)

This guide outlines standards for the React/TypeScript frontend of LionForge
IDE.

## 1. Language & Tooling

- **Language:** TypeScript (latest stable). Enable `strict` mode in
  `tsconfig.json`.
- **Framework:** React (latest stable). Use functional components and hooks.
- **Build Tool:** Vite (recommended) or Create React App.
- **Package Manager:** `npm` or `yarn` (be consistent).
- **Linter:** ESLint with standard React/TypeScript plugins
  (`eslint-plugin-react`, `eslint-plugin-react-hooks`,
  `@typescript-eslint/eslint-plugin`). Enforce rules via configuration
  (`.eslintrc.js`).
- **Formatter:** Prettier. Configure to run on save and enforce via CI.

## 2. Naming Conventions

- **Components:** `PascalCase.tsx` (e.g., `AgentListView.tsx`)
- **Hooks:** `useCamelCase.ts` (e.g., `useTauriEvent.ts`)
- **Utility/API Modules:** `camelCase.ts` (e.g., `apiClient.ts`)
- **Variables/Functions:** `camelCase`
- **Types/Interfaces:** `PascalCase` (e.g., `interface AgentSummary { ... }`)
- **Constants:** `UPPER_SNAKE_CASE` (e.g., `const API_TIMEOUT = 5000;`)

## 3. Component Structure

- Prefer functional components with hooks.
- Keep components small and focused on a single responsibility.
- Define prop types using TypeScript interfaces.
- Use clear and descriptive prop names.
- Organize components logically in folders (e.g., `src/components/agents`,
  `src/components/workflows`).

## 4. State Management

- **Local State:** Use `useState` for simple component-local state.
- **Shared State:** Use Zustand (preferred) or Redux Toolkit for
  application-wide state (e.g., agent list, runtime status, selected project).
  Define clear slices/stores and actions/selectors. Avoid direct mutation.
- **Server Cache State:** Consider `react-query` or RTK Query for managing data
  fetched from the backend, handling caching, loading states, and refetching.

## 5. Styling

- **Method:** Choose ONE consistent method:
  - Tailwind CSS (Recommended for utility-first)
  - CSS Modules (Scoped CSS per component)
  - Styled Components / Emotion (CSS-in-JS)
- Define a basic theme (colors, fonts, spacing) and reuse variables/tokens.

## 6. API Interaction (Tauri)

- Use `@tauri-apps/api/tauri`'s `invoke` function for calling Rust backend
  commands.
- Create a dedicated API client module (`src/lib/api.ts`) that wraps `invoke`
  calls with typed functions for better maintainability and type safety.
- Handle errors returned from `invoke` gracefully, displaying user-friendly
  messages.
- Use `@tauri-apps/api/event`'s `listen` function for receiving events from the
  backend. Use custom hooks (`useTauriEvent`) to manage listeners and state
  updates. Ensure listeners are cleaned up on component unmount (`useEffect`
  return function).

## 7. TypeScript Usage

- Use explicit types wherever possible (variables, function parameters, return
  types). Avoid `any` unless absolutely necessary and justified.
- Use interfaces (`interface`) for defining object shapes (props, API payloads).
- Use enums (`enum`) for sets of related constants (e.g., status types).
- Leverage utility types (`Partial`, `Omit`, `Pick`, etc.) where appropriate.

## 8. Asynchronous Code

- Use `async/await` for handling promises (e.g., from `invoke`).
- Manage loading and error states explicitly in components that perform
  asynchronous operations.

## 9. Testing

- Write unit/integration tests for components using Vitest/Jest and React
  Testing Library.
- Focus on testing component behavior from the user's perspective (rendered
  output, interaction results).
- Mock Tauri API calls (`invoke`, `listen`) during testing.
