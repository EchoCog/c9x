# Deep Tree Echo Implementation - @dtecho Command

## Overview

This implementation adds support for YAML-based prompts to the Codex CLI, specifically enabling the `@dtecho` functionality requested in issue #1.

## New Features

### 1. YAML Prompt Support (`yaml_prompt.rs`)

The new module provides:
- **YAML parsing**: Loads `.prompt.yml` files with `messages` and `model` fields
- **Template variables**: Supports `{{input}}` and other placeholder substitution
- **Integration**: Converts YAML prompts to internal `Prompt` structure
- **Validation**: Handles system/user/assistant message roles

### 2. CLI Command (`codex dtecho`)

New subcommand with options:
- `--prompt-file` / `-p`: Specify YAML prompt file (defaults to `echo-tree.prompt.yml`)
- Input files: Multiple input files to process
- Template substitution: Combines input file contents for `{{input}}` variable

## Usage Examples

```bash
# Use default echo-tree.prompt.yml with input files
codex dtecho file1.md file2.md

# Use custom prompt file
codex dtecho --prompt-file custom.prompt.yml input.md

# Short form
codex dt --prompt-file analysis.yml data/*.md
```

## YAML Prompt Format

```yaml
messages:
  - role: system
    content: |
      You are the Deep Tree Echo Core Engine...
  - role: user
    content: "Analyze this input: {{input}}"
model: "openai/gpt-4.1"
```

## Template Variables

- `{{input}}`: Combined content from all input files
- Custom variables can be added to the `template_vars` HashMap

## Implementation Details

### Files Modified/Added:
1. `codex-rs/core/Cargo.toml` - Added `serde_yaml` dependency
2. `codex-rs/core/src/yaml_prompt.rs` - New YAML prompt handling module
3. `codex-rs/core/src/lib.rs` - Export new module
4. `codex-rs/cli/src/main.rs` - Add `dtecho` subcommand and handler

### Key Components:

#### YamlPrompt Structure
```rust
pub struct YamlPrompt {
    pub messages: Vec<YamlMessage>,
    pub model: Option<String>,
}
```

#### Template Substitution
```rust
fn substitute_template_vars(content: &str, vars: &HashMap<String, String>) -> String
```

#### CLI Integration
```rust
struct DtechoCommand {
    prompt_file: PathBuf,
    input_files: Vec<PathBuf>,
    config_overrides: CliConfigOverrides,
}
```

## Testing

The implementation includes:
- Unit tests for template substitution
- YAML parsing validation tests
- Integration with existing Codex infrastructure

## Future Enhancements

1. **Advanced template variables**: Add date, file paths, metadata
2. **Model execution**: Actually run the processed prompts through the AI model
3. **Output formatting**: Structure the Deep Tree Echo analysis output
4. **Batch processing**: Handle multiple prompt files
5. **Configuration**: Allow custom template variable definitions

## Deep Tree Echo Context

This implementation specifically supports the Deep Tree Echo architecture described in `echo-tree.prompt.yml`:

- **Multi-layer architecture**: Root, Cognitive, Extension, Security membranes
- **Core Engine**: Hypergraph Memory, Echo Propagation, Cognitive Grammar
- **Stepwise reasoning**: Structured analysis with reasoning and conclusions
- **Template-driven**: Process input files through the Deep Tree Echo lens

The `{{input}}` template variable allows the echo-tree prompt to analyze any input files, making it a flexible tool for document analysis and action item extraction as requested in the issue.