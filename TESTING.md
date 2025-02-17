# Testing Documentation for async-mcp

This document provides an overview of the testing suite for the async-mcp project, including instructions on how to run the tests and the current test results.

## Test Modules and Methodologies

The async-mcp project includes the following test modules with their respective testing methodologies:

1. Completable Module:
   - Tests the CompletableString implementation by creating a completable that appends "1" and "2" to the input.
   - Tests the FixedCompletions implementation by creating a list of fruits and checking case-insensitive partial matches.

2. Server::Completion Module:
   - Tests serialization and deserialization of Resource and Prompt references.
   - Verifies correct handling of JSON serialization for Reference enum variants.
   - Tests serialization and deserialization of CompletionRequest, ensuring all fields are correctly preserved.

3. Server::Error Module:
   - Tests error code values to ensure they match expected integer values.
   - Verifies the string representation of error codes.
   - Tests creation and properties of JsonRpcError, including with and without additional data.
   - Tests ServerError creation, including with and without source errors.
   - Verifies error code conversion from integers to ErrorCode enum variants.

4. Server::Notifications Module:
   - Tests serialization and deserialization of Notification enum variants, specifically the Cancelled notification.
   - Verifies that the LoggingLevel enum correctly converts to string representations.

5. Server::Prompt Module:
   - Tests the PromptBuilder functionality:
     - Verifies correct creation of Prompt metadata and RegisteredPrompt.
     - Checks handling of required and optional arguments.
     - Tests argument completion callback registration.
     - Verifies prompt execution with the registered callback.
   - Tests invalid argument handling:
     - Ensures an error is returned for required arguments with empty names.
     - Verifies successful creation with valid optional arguments.

6. Server::Requests Module:
   - Tests serialization and deserialization of Request enum variants, specifically the Initialize request.
   - Verifies that the deserialized request maintains the correct structure and data, including nested fields like protocol_version and client_info.

7. Server::Resource Module:
   - Tests the ResourceTemplate functionality:
     - Verifies correct creation of a ResourceTemplate with a URI pattern.
     - Checks the addition and retrieval of list callbacks.
     - Tests the addition and retrieval of completion callbacks for template variables.
     - Ensures that non-existent completion callbacks return None.

8. Server::Roots Module:
   - Tests the RegisteredRoots functionality:
     - Verifies correct creation of a RegisteredRoots instance with a list callback.
     - Checks that the list_roots method returns the expected roots with correct properties.
   - Tests the RootExt trait implementation for Url:
     - Verifies that URLs within defined roots are correctly identified.
     - Ensures that URLs outside of defined roots are correctly identified as not within roots.

9. Server::Sampling Module:
   - Tests the sampling request and response flow:
     - Creates a SamplingRequest with various parameters including messages, model preferences, and sampling settings.
     - Implements a mock callback function that simulates a sampling operation.
     - Verifies that the callback correctly processes the request and returns a SamplingResult.
     - Checks that the returned SamplingResult contains the expected model name and response text.

10. Server::Tool Module:
    - Tests the ToolBuilder functionality:
      - Creates a tool with a name, description, and input schema.
      - Uses build_typed to create a tool with a typed execution callback.
      - Verifies that the created tool metadata matches the input parameters.
    - Tests the tool execution:
      - Calls the execute_callback with a JSON payload.
      - Verifies that the callback correctly processes the input and returns the expected response.
    - Checks error handling:
      - Implicitly tests that invalid arguments would be caught and returned as an error response.

11. Transport::Error Module: Tests for transport-specific errors
12. Transport::InMemory_Transport Module: Tests for in-memory transport
13. Transport::SSE_Transport Module: Tests for Server-Sent Events (SSE) transport
14. Transport::Stdio_Transport Module: Tests for standard I/O transport
15. Types Module: Tests for server capabilities
16. Bridge::OpenAI Module:
    - Tests converting MCP tools to OpenAI function format
    - Tests converting OpenAI functions to MCP format
    - Tests converting MCP tool responses to OpenAI function responses
17. Bridge::Ollama Module:
    - Tests converting MCP tools to Ollama function format
    - Tests parsing Ollama responses to extract function calls
18. Server::Notifications Module:
    - Tests serialization and deserialization of Notification enum variants
    - Tests string representation of LoggingLevel enum

## Missing Tests

The following areas lack comprehensive test coverage and should be addressed:

1. WebSocket Transport: The WebSocket implementation in src/transport/ws_transport.rs lacks specific tests. Test cases should be added to cover:
   - Opening and closing WebSocket connections
   - Sending and receiving messages
   - Error handling and edge cases

2. Metrics: The implementation and testing of metrics functionality are not visible in the reviewed files.

3. Ping Support: The implementation and testing of ping functionality are not visible in the reviewed files.

## Implementation Status

1. Notifications:
   - Progress Updates: Implemented
   - Cancellation: Implemented

2. Monitoring:
   - Logging Support: Implemented
     - Logging levels: Debug, Info, Warn, and Error
     - LoggingMessageParams struct for logging message notifications
   - Level Control: Implemented through LoggingLevel enum
   - Message Notifications: Implemented as part of the Notification enum
   - Metrics: Not implemented or not visible in the reviewed files

   Logging Tests:
   - test_logging_level_display: Verifies the string representation of logging levels
   - Partial coverage in test_notification_serialization (tests notification serialization in general)

3. Utilities:
   - Ping Support: Not visible in the reviewed files
   - Cancellation Support: Implemented
   - Progress Tracking:
     - Progress Notifications: Implemented
     - Progress Tokens: Implemented
     - Progress Values: Implemented

To improve the overall quality and reliability of the async-mcp project, it is recommended to add test coverage for the missing areas and implement the features that are currently not visible or not implemented.

## Running the Tests

To run the tests, use the following command in the project root directory:

```
cargo test
```

This will compile the project and run all the tests, displaying the results in the console.

## Running Benchmarks

To run the benchmarks, you need to use nightly Rust. First, switch to the nightly toolchain:

```
rustup default nightly
```

Then, run the benchmarks using:

```
cargo bench
```

This will compile the project in release mode and run all the benchmarks, displaying the results in the console.

Remember to switch back to stable Rust after running benchmarks:

```
rustup default stable
```

## Test Results

Below are the most recent test results:

```
running 33 tests
test server::error::tests::test_error_code_conversion ... ok
test server::completion::tests::test_prompt_reference ... ok
test server::completion::tests::test_resource_reference ... ok
test server::completion::tests::test_completion_request ... ok
test completable::tests::test_fixed_completions ... ok
test completable::tests::test_completable_string ... ok
test server::error::tests::test_error_codes ... ok
test server::error::tests::test_error_display ... ok
test server::error::tests::test_server_error ... ok
test server::error::tests::test_json_rpc_error ... ok
test server::notifications::tests::test_notification_serialization ... ok
test server::notifications::tests::test_logging_level_display ... ok
test server::prompt::tests::test_prompt_builder_invalid_args ... ok
test server::requests::tests::test_request_serialization ... ok
test server::prompt::tests::test_prompt_builder ... ok
test server::resource::tests::test_resource_template ... ok
test transport::error::tests::test_error_code ... ok
test server::roots::tests::test_url_within_roots ... ok
test server::roots::tests::test_roots_handler ... ok
test transport::error::tests::test_error_codes ... ok
test transport::error::tests::test_error_display ... ok
test server::sampling::tests::test_sampling_request ... ok
test server::tool::tests::test_tool_builder ... ok
test transport::sse_transport::tests::test_parse_large_sse_message ... ok
test transport::inmemory_transport::tests::test_async_transport ... ok
test transport::inmemory_transport::tests::test_multiple_messages ... ok
test types::tests::test_server_capabilities ... ok
test transport::tests::test_deserialize_initialize_request ... ok
test transport::sse_transport::tests::test_parse_real_sse_message ... ok
test transport::stdio_transport::tests::test_stdio_transport ... ok
test transport::stdio_transport::tests::test_shutdown_with_pending_io ... ok
test transport::stdio_transport::tests::test_graceful_shutdown ... ok
test transport::inmemory_transport::tests::test_graceful_shutdown ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.00s

   Doc-tests async_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All tests are currently passing, indicating that the project is in a stable state. Additionally, the doc-tests ran successfully with no failures.

## Benchmark Results

Latest benchmark results:

1. `bench_completable_string`: 80.946 ns
2. `bench_fixed_completions`: 179.57 ns
3. `bench_notification_serialization`: 62.963 ns
4. `bench_prompt_builder`: 247.22 ns
5. `bench_connection_setup`: 29.873 ns
6. `bench_notification_sending`: 1.1527 ms
7. `bench_server_capabilities_registration`: 20.460 ns

To run the benchmarks:

```
cargo bench
```
