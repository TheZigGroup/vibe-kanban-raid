//! Codebase architecture rules and patterns for task generation

/// Frontend architecture rules that should be followed when implementing tasks
pub fn get_frontend_rules() -> String {
    r#"
## FRONTEND IMPLEMENTATION RULES

### 🚨 CRITICAL: DO NOT RECREATE EXISTING COMPONENTS
- The application ALREADY HAS a sidebar and header/navbar
- DO NOT create new layout components
- USE the existing NormalLayout or NewDesignLayout
- Pages should ONLY contain page-specific content

### 1. ROUTING & LAYOUT
- Routes must be wrapped in appropriate scope:
  * Legacy pages: `<LegacyDesignScope><NormalLayout /></LegacyDesignScope>`
  * New design: `<NewDesignScope><NewDesignLayout /></NewDesignScope>`
- DO NOT create standalone pages without layout wrappers
- Location: `/frontend/src/App.tsx`
- Use React Router v6: `useNavigate()`, `useParams()`, `useSearchParams()`

### 2. EXISTING LAYOUT COMPONENTS (DO NOT RECREATE)
- **Navbar**: `/frontend/src/components/layout/Navbar.tsx`
  * Already includes: logo, search, project actions, settings menu
  * Conditionally hidden for full-screen views (`?view=preview`)
- **NormalLayout**: `/frontend/src/components/layout/NormalLayout.tsx`
  * Structure: DevBanner → Navbar → Outlet
  * Uses `h-screen` and `flex-1 overflow-auto`

### 3. STYLING SYSTEM
**New Design CSS Variables** (`.new-design` scope):
- Text: `text-high`, `text-normal`, `text-low`
- Background: `bg-primary`, `bg-secondary`, `bg-panel`
- Accent: `bg-brand`, `bg-brand-hover`, `text-on-brand`
- Spacing: `px-base`, `py-half`, `gap-base`, `m-double`
- Font: IBM Plex Sans (default), IBM Plex Mono (code)
- Sizes: `text-xs` (8px), `text-sm` (10px), `text-base` (12px), `text-lg` (14px)

**❌ AVOID**:
- `text-gray-*` → Use `text-normal`, `text-low` instead
- `bg-gray-*` → Use `bg-primary`, `bg-secondary` instead
- Hardcoded spacing → Use tokens (`px-base`, `gap-base`)

**Location**:
- CSS: `/frontend/src/styles/new/index.css`
- Tailwind config: `/frontend/tailwind.new.config.js`

### 4. COMPONENT ARCHITECTURE
**Three-Layer Pattern**:
```
feature/
├── containers/    → State management, data fetching (hooks, context)
├── views/         → Pure presentation components (props only)
└── primitives/    → Reusable UI elements (PascalCase filenames!)
```

**Rules**:
- Containers: Manage state, call hooks, pass props to Views
- Views: Stateless, receive data via props, call callbacks
- Primitives: Small reusable components in `/components/ui-new/primitives/`
- **CRITICAL**: All files in `ui-new/` must be PascalCase (e.g., `Field.tsx`, NOT `field.tsx`)

### 5. STATE MANAGEMENT
**Zustand Stores** (UI state only):
```typescript
// Location: /frontend/src/stores/
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export const useMyStore = create<State>()(
  persist((set) => ({ ... }), { name: 'storage-key' })
);

// Export selectors for performance
export const useMyValue = () => useMyStore((s) => s.value);
```

**React Query** (server state):
```typescript
// Location: /frontend/src/hooks/
const { data, isLoading } = useQuery({
  queryKey: ['resource', id],
  queryFn: () => api.get(id),
  staleTime: 5 * 60 * 1000,
});
```

**Context API** (shared state):
- Location: `/frontend/src/contexts/`
- Pattern: Create Provider component + useX hook

### 6. DIALOG/MODAL PATTERN
**New Design** - Use NiceModal:
```typescript
import NiceModal, { useModal } from '@ebay/nice-modal-react';

export const MyDialog = NiceModal.create(({ ... }) => {
  const modal = useModal();
  return (
    <Dialog open={modal.visible} onOpenChange={modal.remove}>
      {/* content */}
    </Dialog>
  );
});

// Usage: MyDialog.show().then(result => { ... });
```

**Legacy Design** - Use defineModal:
```typescript
export const MyDialog = defineModal(async () => {
  const { MyDialogContent } = await import('./MyDialogContent');
  return MyDialogContent;
});
```

### 7. COMMON MISTAKES TO AVOID
1. ❌ Creating new navbar/sidebar components
2. ❌ Not wrapping routes in design scope + layout
3. ❌ Using hardcoded colors instead of CSS variables
4. ❌ Putting business logic in View components
5. ❌ Using camelCase for files in `ui-new/` (must be PascalCase)
6. ❌ Creating Context without matching useX hook
7. ❌ Forgetting to persist UI preferences in Zustand

### 8. CHECKLIST FOR NEW FEATURES
- [ ] Add route in `/frontend/src/App.tsx` with proper scope
- [ ] Create Container component (data fetching, state)
- [ ] Create View component (presentation only)
- [ ] Create Primitives if needed (PascalCase files)
- [ ] Use CSS variables and Tailwind tokens (no hardcoded colors)
- [ ] Add Zustand store for UI state if needed
- [ ] Add types in TypeScript
- [ ] Add i18n translations in `/frontend/src/i18n/locales/*/`
- [ ] Test with `pnpm run check` and `pnpm run lint`

### 9. KEY FILES TO REFERENCE
- Routing: `/frontend/src/App.tsx`
- Layouts: `/frontend/src/components/layout/NormalLayout.tsx`
- Theme: `/frontend/src/components/ThemeProvider.tsx`
- CSS Variables: `/frontend/src/styles/new/index.css`
- Primitives: `/frontend/src/components/ui-new/primitives/`
- Stores: `/frontend/src/stores/`
- Contexts: `/frontend/src/contexts/`

### 10. TESTING
- Run `pnpm run check` (TypeScript)
- Run `pnpm run lint` (ESLint)
- Test in dev mode: `pnpm run dev:qa`
"#
    .to_string()
}

/// Backend architecture rules for task generation
pub fn get_backend_rules() -> String {
    r#"
## BACKEND IMPLEMENTATION RULES

### 1. PROJECT STRUCTURE
- `crates/server/src/routes/` - API route handlers (Axum)
- `crates/services/src/services/` - Business logic services
- `crates/db/src/models/` - Database models (SQLx)
- `crates/db/migrations/` - SQL migrations
- `crates/executors/` - Executor implementations

### 2. ROUTING PATTERN (Axum)
```rust
// crates/server/src/routes/my_feature.rs
use axum::{Router, extract::{Path, State}, routing::get};

pub async fn list_items(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<Item>>>, ApiError> {
    let items = MyService::new(deployment.db().pool.clone()).list(project_id).await?;
    Ok(ResponseJson(ApiResponse::success(items)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/items", get(list_items).post(create_item))
        .route("/items/:id", get(get_item).delete(delete_item))
}
```

### 3. DATABASE MODELS
```rust
// crates/db/src/models/my_model.rs
use sqlx::{FromRow, SqlitePool};
use serde::{Serialize, Deserialize};
use ts_rs::TS;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MyModel {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl MyModel {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(MyModel, "SELECT * FROM my_table WHERE id = $1", id)
            .fetch_optional(pool)
            .await
    }
}
```

### 4. MIGRATIONS
- Create in `crates/db/migrations/` with timestamp prefix
- Format: `YYYYMMDDHHMMSS_description.sql`
- Run: `pnpm run prepare-db` after creating

### 5. TYPE GENERATION
- Add `#[derive(TS)]` to structs that need TypeScript types
- Run `pnpm run generate-types` to regenerate `shared/types.ts`
- DO NOT manually edit `shared/types.ts`

### 6. SERVICES PATTERN
```rust
// crates/services/src/services/my_service.rs
pub struct MyService {
    pool: SqlitePool,
}

impl MyService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn do_something(&self, id: Uuid) -> Result<Item, MyError> {
        // Business logic here
    }
}
```

### 7. ERROR HANDLING
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
}
```

### 8. DATABASE RULES (CRITICAL)

**Database Initialization**:
- Database MUST be created before running migrations
- For PostgreSQL projects: Create database with `CREATE DATABASE dbname;`
- For SQLite projects: Database file is created automatically by migrations
- NEVER assume database exists - always check or create it first

**Migration Workflow**:
```
1. Create migration file in crates/db/migrations/
   Format: YYYYMMDDHHMMSS_description.sql

2. Write SQL for both UP and DOWN (if needed)

3. Run migrations:
   - pnpm run prepare-db (updates SQLx query cache)
   - Migrations run automatically in dev mode

4. Add migration file to git
   - Migration files MUST be committed
   - SQLx query cache files MUST be committed (.sqlx/ directory)
```

**Migration Best Practices**:
- One logical change per migration
- Always test migrations can be applied to empty database
- Include data migrations if schema changes affect existing data
- Never edit old migrations - create new ones to fix issues
- Use transactions where supported

**Database Schema Updates**:
```rust
// 1. Create migration file
// 2. Update model struct
#[derive(FromRow, Serialize, Deserialize, TS)]
pub struct MyModel {
    pub new_field: String,  // Add new field
}

// 3. Update all queries to include new field
sqlx::query_as!(MyModel, "SELECT id, name, new_field FROM my_table WHERE...")

// 4. Run pnpm run prepare-db to update query cache
// 5. Run pnpm run generate-types to update TypeScript
```

**CRITICAL MISTAKES TO AVOID**:
1. ❌ Running migrations manually with `npm run init-db` - migrations should run automatically
2. ❌ Creating tables in code without migrations - ALWAYS use migration files
3. ❌ Assuming database exists - check or create it first
4. ❌ Not committing migration files or .sqlx/ cache
5. ❌ Editing old migrations - create new ones instead
6. ❌ Not running `pnpm run prepare-db` after schema changes
7. ❌ Forgetting to update TypeScript types after model changes

**Task Completion Checklist for Database Changes**:
- [ ] Migration file created in `crates/db/migrations/`
- [ ] Migration SQL is correct and tested
- [ ] Models updated with new fields
- [ ] All queries updated to include new fields
- [ ] `pnpm run prepare-db` executed successfully
- [ ] `pnpm run generate-types` executed if models changed
- [ ] Migration file committed to git
- [ ] .sqlx/ query cache committed to git
- [ ] Database can be initialized from scratch with migrations
- [ ] No manual `npm run init-db` required

### 9. TESTING
- Run `cargo test --workspace` for all tests
- Run `cargo check` for compilation check
- Run `pnpm run backend:check` for Rust cargo check
"#
    .to_string()
}

/// Get rules for both frontend and backend
pub fn get_all_rules() -> String {
    format!("{}\n\n{}", get_frontend_rules(), get_backend_rules())
}
