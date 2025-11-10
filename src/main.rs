// Import necessary items from our dependencies
use rmcp::{
    RoleServer,
    ServiceExt,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData, Implementation,
        InitializeRequestParam, InitializeResult, ListToolsResult, PaginatedRequestParam,
        ProtocolVersion, ServerCapabilities, Tool,
    },
    schemars, // For generating the "menu"
    service::RequestContext,
    transport::stdio, // The stdio communication channel
};
use serde::Deserialize; // For our tool's inputs
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

// 1. DEFINE YOUR TOOL'S INPUT PARAMETERS
// The AI will see this and know what to provide.
// 'schemars::JsonSchema' automatically builds the "menu" for the AI.
#[derive(Deserialize, schemars::JsonSchema)]
struct AddMemoryParams {
    #[schemars(description = "The content to store in memory")]
    content: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct GetMemoriesParams {}

// 2. DEFINE YOUR SERVER
// This struct will hold any state your server needs (like API keys, etc.)
#[derive(Clone)]
struct MyServer;

// 3. IMPLEMENT THE TOOL HANDLER
// This is the core of your server. We implement the `ServerHandler` trait.
impl ServerHandler for MyServer {
    // This function lists all available tools that the server provides
    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        use std::sync::Arc;

        // Schema for add_memory tool
        let memory_schema = schemars::schema_for!(AddMemoryParams);
        let memory_input_schema = rmcp::serde_json::to_value(memory_schema).map_err(|e| {
            ErrorData::internal_error(format!("Failed to serialize schema: {}", e), None)
        })?;

        let memory_input_schema_map =
            if let rmcp::serde_json::Value::Object(map) = memory_input_schema {
                Arc::new(map)
            } else {
                return Err(ErrorData::internal_error("Schema is not an object", None));
            };

        // Schema for get_memories tool
        let get_memories_schema = schemars::schema_for!(GetMemoriesParams);
        let get_memories_input_schema =
            rmcp::serde_json::to_value(get_memories_schema).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize schema: {}", e), None)
            })?;

        let get_memories_input_schema_map =
            if let rmcp::serde_json::Value::Object(map) = get_memories_input_schema {
                Arc::new(map)
            } else {
                return Err(ErrorData::internal_error("Schema is not an object", None));
            };

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "add_memory".into(),
                    title: None,
                    description: Some("Add a new memory about the user. Call this whenever the user shares preferences, facts about themselves, or explicitly asks you to remember something.".into()),
                    input_schema: memory_input_schema_map,
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "get_memories".into(),
                    title: None,
                    description: Some("Retrieve all stored memories about the user.".into()),
                    input_schema: get_memories_input_schema_map,
                    output_schema: None,
                    annotations: None,
                    icons: None,
                }
            ],
            next_cursor: None,
        })
    }

    // This function is called when the AI decides to *use* our tool.
    async fn call_tool(
        &self,
        params: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = params.name.as_ref();

        // This 'match' is how you handle multiple tools.
        match tool_name {
            "add_memory" => {
                // Parse the arguments into our AddMemoryParams struct
                let args = params.arguments.unwrap_or_default();
                let args_value = rmcp::serde_json::Value::Object(args);
                let memory_params: AddMemoryParams = rmcp::serde_json::from_value(args_value)
                    .map_err(|e| {
                        ErrorData::invalid_request(format!("Invalid parameters: {}", e), None)
                    })?;

                // Save the memory to markdown file
                save_memory(&memory_params.content).map_err(|e| {
                    ErrorData::internal_error(format!("Failed to save memory: {}", e), None)
                })?;

                let message = "Memory saved successfully.".to_string();
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            "get_memories" => {
                // Get all memories from the markdown file
                let memories = get_memories().map_err(|e| {
                    ErrorData::internal_error(format!("Failed to retrieve memories: {}", e), None)
                })?;

                Ok(CallToolResult::success(vec![Content::text(memories)]))
            }
            _ => {
                // Handle cases where the tool name is unknown
                Err(ErrorData::invalid_request(
                    format!("Unknown tool: {}", tool_name),
                    None,
                ))
            }
        }
    }

    // This function is called during initialization to set up the server
    async fn initialize(
        &self,
        _params: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "Memory MCP Server (Rust)".to_string(),
                title: None,
                version: "0.1.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: None,
        })
    }
}

// Helper function to save memory to markdown file
fn save_memory(content: &str) -> anyhow::Result<()> {
    use std::time::SystemTime;

    // Get the memory file path (in the current directory)
    let mut path = PathBuf::from(".");
    path.push("memories.md");

    // Create or append to the file
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    // Get current timestamp
    let now = SystemTime::now();

    // Write the memory with timestamp
    writeln!(file, "## {:?}", now)?;
    writeln!(file, "{}", content)?;
    writeln!(file)?;

    Ok(())
}

// Helper function to retrieve all memories from markdown file
fn get_memories() -> anyhow::Result<String> {
    use std::fs;

    // Get the memory file path (in the current directory)
    let mut path = PathBuf::from(".");
    path.push("memories.md");

    // Check if file exists
    if !path.exists() {
        return Ok("No memories found yet.".to_string());
    }

    // Read the file content
    let content = fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Ok("No memories found yet.".to_string());
    }

    Ok(content)
}

// 4. CREATE THE MAIN FUNCTION TO RUN THE SERVER
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create an instance of our server
    let server = MyServer;

    // This is the crucial part:
    // 1. 'stdio()' creates the stdio transport.
    // 2. '.serve()' attaches our server logic to the transport.
    // 3. '.waiting()' keeps the server running until it's shut down.
    let running_service = server.serve(stdio()).await?;
    let _quit_reason = running_service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rmcp::serde_json;
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    #[test]
    fn test_mcp_protocol_initialize_and_list_tools() {
        // Build the binary first
        let build_result = Command::new("cargo")
            .args(&["build"])
            .output()
            .expect("Failed to build binary");

        assert!(build_result.status.success(), "Build should succeed");

        // Start the MCP server process
        let mut child = Command::new("./target/debug/memory-mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start MCP server");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);

        // Test 1: Send initialize request
        let initialize_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        writeln!(stdin, "{}", initialize_request.to_string())
            .expect("Failed to write initialize request");
        stdin.flush().expect("Failed to flush");

        // Read initialize response
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .expect("Failed to read initialize response");

        println!("Initialize response: {}", response_line);

        let init_response: serde_json::Value =
            serde_json::from_str(&response_line).expect("Failed to parse initialize response");

        assert_eq!(init_response["jsonrpc"], "2.0");
        assert_eq!(init_response["id"], 1);
        assert!(
            init_response["result"].is_object(),
            "Should have result object"
        );
        assert_eq!(
            init_response["result"]["serverInfo"]["name"],
            "Memory MCP Server (Rust)"
        );
        println!("✓ Initialize test passed");

        // Test 2: Send initialized notification
        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        writeln!(stdin, "{}", initialized_notification.to_string())
            .expect("Failed to write initialized notification");
        stdin.flush().expect("Failed to flush");

        // Test 3: Send list_tools request
        let list_tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        writeln!(stdin, "{}", list_tools_request.to_string())
            .expect("Failed to write list_tools request");
        stdin.flush().expect("Failed to flush");

        // Read list_tools response
        let mut tools_response_line = String::new();
        reader
            .read_line(&mut tools_response_line)
            .expect("Failed to read list_tools response");

        println!("List tools response: {}", tools_response_line);

        let tools_response: serde_json::Value = serde_json::from_str(&tools_response_line)
            .expect("Failed to parse list_tools response");

        assert_eq!(tools_response["jsonrpc"], "2.0");
        assert_eq!(tools_response["id"], 2);
        assert!(
            tools_response["result"].is_object(),
            "Should have result object"
        );
        assert!(
            tools_response["result"]["tools"].is_array(),
            "Should have tools array"
        );

        let tools = tools_response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2, "Should have exactly 2 tools");

        let memory_tool = &tools[0];
        assert_eq!(memory_tool["name"], "add_memory");
        assert!(
            memory_tool["inputSchema"].is_object(),
            "Should have inputSchema"
        );

        let get_memories_tool = &tools[1];
        assert_eq!(get_memories_tool["name"], "get_memories");
        assert!(
            get_memories_tool["inputSchema"].is_object(),
            "Should have inputSchema"
        );

        println!("✓ List tools test passed");
        println!(
            "  Tool 1: {} - {}",
            memory_tool["name"], memory_tool["description"]
        );
        println!(
            "  Tool 2: {} - {}",
            get_memories_tool["name"], get_memories_tool["description"]
        );

        // Test 4: Call add_memory tool to save a memory
        let add_memory_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "add_memory",
                "arguments": {
                    "content": "User prefers dark mode and uses Rust for development"
                }
            }
        });

        writeln!(stdin, "{}", add_memory_request.to_string())
            .expect("Failed to write add_memory request");
        stdin.flush().expect("Failed to flush");

        // Read add_memory response
        let mut add_memory_response_line = String::new();
        reader
            .read_line(&mut add_memory_response_line)
            .expect("Failed to read add_memory response");

        println!("Add memory response: {}", add_memory_response_line);

        let add_memory_response: serde_json::Value =
            serde_json::from_str(&add_memory_response_line)
                .expect("Failed to parse add_memory response");

        assert_eq!(add_memory_response["jsonrpc"], "2.0");
        assert_eq!(add_memory_response["id"], 3);
        assert!(
            add_memory_response["result"].is_object(),
            "Should have result object"
        );

        let add_content = &add_memory_response["result"]["content"];
        assert!(add_content.is_array(), "Should have content array");
        assert!(
            add_content[0]["text"]
                .as_str()
                .unwrap()
                .contains("Memory saved successfully")
        );

        println!("✓ Add memory test passed");

        // Test 5: Call get_memories tool to retrieve memories
        let get_memories_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_memories",
                "arguments": {}
            }
        });

        writeln!(stdin, "{}", get_memories_request.to_string())
            .expect("Failed to write get_memories request");
        stdin.flush().expect("Failed to flush");

        // Read get_memories response
        let mut get_memories_response_line = String::new();
        reader
            .read_line(&mut get_memories_response_line)
            .expect("Failed to read get_memories response");

        println!("Get memories response: {}", get_memories_response_line);

        let get_memories_response: serde_json::Value =
            serde_json::from_str(&get_memories_response_line)
                .expect("Failed to parse get_memories response");

        assert_eq!(get_memories_response["jsonrpc"], "2.0");
        assert_eq!(get_memories_response["id"], 4);
        assert!(
            get_memories_response["result"].is_object(),
            "Should have result object"
        );

        let get_content = &get_memories_response["result"]["content"];
        assert!(get_content.is_array(), "Should have content array");
        let memories_text = get_content[0]["text"].as_str().unwrap();
        assert!(
            memories_text.contains("User prefers dark mode and uses Rust for development"),
            "Should contain the memory we just added"
        );

        println!("✓ Get memories test passed");
        println!("  Retrieved memories contain our test data");

        // Clean up
        child.kill().expect("Failed to kill child process");

        // Remove test memories file
        let _ = std::fs::remove_file("memories.md");
    }
}
