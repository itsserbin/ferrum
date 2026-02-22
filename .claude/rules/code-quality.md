# Code Quality

## DRY — Don't Repeat Yourself
NEVER duplicate logic.
Before writing new code:
- Search for existing functions or methods that already do the same thing.
- If 2 or more places perform identical logic, extract it into a shared function or method.
- NEVER copy-paste a block with minor variations — parameterize instead.

## KISS — Keep It Simple
Solve the current problem with the minimum necessary code.
NEVER add complexity for hypothetical future requirements.
Three similar lines are better than a premature abstraction.

## Rust-Specific Principles

### Type-driven design
Prefer making invalid states unrepresentable at compile time.
Use the newtype pattern, `PhantomData`, and sealed traits over runtime validation.

### Composition over deep trait chains
Use traits + structs.
Keep traits small and single-purpose (like `std`'s `Iterator`, `Display`).
NEVER design deep trait dependency chains.

### Static vs dynamic dispatch
Prefer `impl Trait` (static dispatch) for performance-critical inner loops.
Use `dyn Trait` when type erasure genuinely serves the design:
  - External library traits where generics are not possible
  - Heterogeneous IO (`Box<dyn Read + Send>`)
  - Shared render-path functions called once per frame

### RAII
Tie resource lifetimes to scopes.
Use `Drop` for cleanup.
NEVER manage resources manually when a guard type can do it.

### Error handling
ALWAYS propagate errors with `?`.
NEVER use `.unwrap()` in production code paths.
Use `anyhow::Result` — it is the project standard.
Do not introduce `thiserror` without discussion.

### Rust API Guidelines
Follow https://rust-lang.github.io/api-guidelines/:
- Getters NEVER start with `get_` — use `field_name()` not `get_field_name()`
- Implement common traits (`Debug`, `Display`, `Clone`, `PartialEq`) where reasonable
- Use standard conversion traits (`From`, `Into`, `TryFrom`) over custom conversion methods
