# Contributing Guide

Thank you for your interest in contributing to Kuse Cowork! This guide will help you get started.

## Ways to Contribute

- **Bug Reports**: Report issues you encounter
- **Feature Requests**: Suggest new features
- **Code**: Submit pull requests
- **Documentation**: Improve docs
- **Testing**: Help test new releases
- **Community**: Help others in discussions

## Getting Started

### 1. Fork the Repository

1. Go to [github.com/kuse-ai/kuse_cowork](https://github.com/kuse-ai/kuse_cowork)
2. Click "Fork"
3. Clone your fork:

```bash
git clone https://github.com/YOUR_USERNAME/kuse_cowork.git
cd kuse_cowork
```

### 2. Set Up Development Environment

Follow the [Development Setup](setup.md) guide.

### 3. Create a Branch

```bash
# Create feature branch
git checkout -b feature/my-feature

# Or bugfix branch
git checkout -b fix/issue-123
```

## Development Workflow

### Making Changes

1. Make your changes
2. Test locally with `pnpm tauri dev`
3. Run linters and tests

```bash
# Frontend
pnpm lint
pnpm test

# Backend
cd src-tauri
cargo fmt
cargo clippy
cargo test
```

### Commit Messages

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Examples:

```
feat(agent): add support for parallel tool execution

fix(docker): handle container cleanup on timeout

docs(readme): update installation instructions
```

### Pull Request Process

1. **Push your branch**:
   ```bash
   git push origin feature/my-feature
   ```

2. **Create Pull Request**:
   - Go to your fork on GitHub
   - Click "Compare & pull request"
   - Fill out the PR template

3. **PR Template**:
   ```markdown
   ## Description
   Brief description of changes

   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Documentation
   - [ ] Refactoring

   ## Testing
   How was this tested?

   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Tests pass locally
   - [ ] Documentation updated
   ```

4. **Review Process**:
   - Maintainers will review your PR
   - Address feedback
   - Once approved, PR will be merged

## Code Guidelines

### TypeScript/SolidJS

```typescript
// Use TypeScript types
interface Props {
  title: string;
  onClick: () => void;
}

// Functional components
const MyComponent: Component<Props> = (props) => {
  // Use signals for state
  const [count, setCount] = createSignal(0);

  // Use createMemo for derived state
  const doubled = createMemo(() => count() * 2);

  return (
    <div>
      <h1>{props.title}</h1>
      <button onClick={props.onClick}>
        Count: {count()} (doubled: {doubled()})
      </button>
    </div>
  );
};
```

### Rust

```rust
// Use proper error handling
pub fn process(input: &str) -> Result<Output, Error> {
    let data = parse(input)?;
    let result = transform(data)?;
    Ok(result)
}

// Document public APIs
/// Processes the input and returns transformed output.
///
/// # Arguments
/// * `input` - The input string to process
///
/// # Returns
/// The transformed output or an error
pub fn process(input: &str) -> Result<Output, Error> {
    // ...
}

// Use meaningful names
let user_settings = load_settings()?;  // Good
let us = load_settings()?;              // Avoid
```

### CSS

```css
/* Use component-scoped styles */
.my-component {
  /* Layout */
  display: flex;
  flex-direction: column;

  /* Spacing */
  padding: 1rem;
  margin: 0.5rem;

  /* Appearance */
  background: var(--background-color);
  border-radius: 4px;
}

/* Use CSS variables for theming */
.my-component-title {
  color: var(--text-color);
  font-size: 1.25rem;
}
```

## Testing Guidelines

### Unit Tests

```typescript
// Frontend
import { render, screen } from "@solidjs/testing-library";

test("renders button with text", () => {
  render(() => <Button>Click me</Button>);
  expect(screen.getByText("Click me")).toBeInTheDocument();
});
```

```rust
// Backend
#[test]
fn test_parse_response() {
    let input = r#"{"content": "hello"}"#;
    let result = parse_response(input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "hello");
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_tool_execution() {
    let executor = ToolExecutor::new(Some("/tmp/test".into()));
    let result = executor.execute(&read_file_tool).await;
    assert!(result.content.contains("expected content"));
}
```

## Documentation

### Code Documentation

```rust
/// Tool executor for running agent tools.
///
/// Handles execution of built-in tools and MCP tools,
/// managing the project context and permissions.
pub struct ToolExecutor {
    /// Path to the project workspace
    project_path: Option<String>,
    /// MCP manager for external tools
    mcp_manager: Option<Arc<MCPManager>>,
}
```

### User Documentation

- Use clear, concise language
- Include examples
- Add screenshots when helpful
- Keep docs up to date with code changes

## Issue Guidelines

### Bug Reports

Include:
- **Description**: What happened?
- **Expected**: What should happen?
- **Steps**: How to reproduce
- **Environment**: OS, version, etc.
- **Logs**: Error messages if any

### Feature Requests

Include:
- **Problem**: What problem does this solve?
- **Solution**: Proposed solution
- **Alternatives**: Other options considered
- **Context**: Any additional info

## Community Guidelines

### Be Respectful

- Treat everyone with respect
- Be constructive in feedback
- Welcome newcomers

### Be Helpful

- Answer questions when you can
- Share knowledge
- Help improve documentation

### Be Patient

- Maintainers are volunteers
- Reviews take time
- Complex features need discussion

## Recognition

Contributors are recognized in:
- GitHub contributors list
- Release notes
- Project README (for significant contributions)

## Questions?

- Open a [Discussion](https://github.com/kuse-ai/kuse_cowork/discussions)
- Check existing issues
- Read the documentation

Thank you for contributing!
