GenerationRequest should have a SessionId rather than a session, this will avoid a clone.


## Proposed Solution

The current `GenerationRequest` struct stores the entire `Session` object, which requires cloning when passing the request around. This is inefficient since the session data can be large.

**Changes needed:**
1. Change `GenerationRequest.session: Session` to `GenerationRequest.session_id: SessionId`
2. Update all code that constructs `GenerationRequest` to pass `SessionId` instead of `Session`  
3. Update all code that processes `GenerationRequest` to retrieve the `Session` from `SessionManager` using the `SessionId`
4. Update specifications to reflect the change

**Benefits:**
- Avoids expensive cloning of entire `Session` objects
- More efficient memory usage
- Cleaner separation of concerns - request only holds reference to session

**Implementation steps:**
1. Update the struct definition in `types.rs`
2. Update constructors in tests, examples, CLI, and agent code
3. Update consumers in agent processing code to fetch session from SessionManager
4. Run tests to verify everything works correctly