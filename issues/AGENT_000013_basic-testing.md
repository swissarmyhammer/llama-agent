# Basic Testing Infrastructure

Refer to ./specifications/index.md

## Objective
Set up comprehensive testing infrastructure for core components with unit and integration tests.

## Tasks
- [ ] Create test modules for each major component
- [ ] Add unit tests for core types and error handling
- [ ] Create mock implementations for testing
- [ ] Add integration tests for model loading (with test models)
- [ ] Test session management and message handling
- [ ] Add queue testing with concurrent scenarios
- [ ] Create MCP client testing with mock servers
- [ ] Add CLI testing with various argument combinations

## Test Structure
- Unit tests in each module (types, session, queue, etc.)
- Integration tests in `tests/` directory
- Test utilities and helpers in common module
- Mock implementations for external dependencies
- Property-based testing for core data structures

## Component Testing
- ModelManager: Test loading, auto-detection, error cases
- RequestQueue: Test concurrency, timeouts, capacity limits
- SessionManager: Test CRUD operations, concurrent access
- ChatTemplateEngine: Test rendering, tool call extraction
- MCPClient: Test with mock MCP servers
- AgentServer: End-to-end integration testing

## Test Data
- Small test models for model loading tests
- Mock MCP server responses for tool testing
- Various chat template formats for template testing
- Edge cases and error scenarios
- Performance test scenarios

## Testing Tools
- Use cargo nextest for test execution
- Property-based testing with proptest where appropriate
- Mock servers and test doubles for external dependencies
- Test utilities for common setup/teardown

## Acceptance Criteria
- All major components have comprehensive unit tests
- Integration tests cover end-to-end scenarios
- Tests run reliably in CI environment
- Test coverage is comprehensive for error paths
- Performance tests validate queue behavior under load
- All tests pass consistently