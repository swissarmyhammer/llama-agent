We need an integration test that exercises the cli

use https://huggingface.co/unsloth/Qwen3-0.6B-GGUF

by hand I test this as cargo run --package llama-agent-cli -- --model unsloth/Qwen3-0.6B-GGUF   --prompt "What is an apple?" --limit 64

you should set up the CLI to have a a testable separation between the argument parsing the actual execution

## Proposed Solution

I will implement this by:

1. **Refactor CLI Structure**: Create a testable separation between argument parsing and execution by:
   - Moving the main execution logic from `main()` to a new `run_cli()` function that takes parsed arguments
   - Making the CLI logic testable without invoking the full binary
   - Keeping `main()` focused only on argument parsing and error handling

2. **Create Integration Test**: Develop an integration test that:
   - Uses the refactored CLI functions directly (avoiding subprocess calls)
   - Tests with the specified model: `unsloth/Qwen3-0.6B-GGUF`  
   - Uses the same parameters as the manual test: `--prompt "What is an apple?" --limit 64`
   - Verifies the CLI executes successfully and produces expected output

3. **Test Structure**: Place the integration test in the CLI crate's test directory following Rust conventions

This approach provides proper separation of concerns while enabling comprehensive testing of the CLI functionality without external dependencies or subprocess complexity.