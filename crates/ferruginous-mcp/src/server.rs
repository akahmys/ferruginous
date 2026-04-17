use rmcp::{tool, tool_router, tool_handler, ServiceExt, handler::server::{ServerHandler, router::Router, wrapper::Parameters}};
use crate::tools::render::{RenderArgs, render_page_impl};
use crate::tools::audit::{AuditArgs, audit_document_impl};

/// The Ferruginous MCP Server implementation.
///
/// It provides tools for PDF rendering and structural auditing via the
/// Model Context Protocol.
pub struct FerruginousServer;

#[tool_handler]
impl ServerHandler for FerruginousServer {}

#[tool_router]
impl FerruginousServer {
    /// Creates a new instance of the Ferruginous MCP server.
    pub fn new() -> Self {
        Self
    }

    #[tool(
        name = "render_page",
        description = "Renders a specific page of a PDF document to a PNG image for visual inspection."
    )]
    /// MCP tool: render_page
    pub async fn render_page(&self, Parameters(args): Parameters<RenderArgs>) -> Result<String, String> {
        render_page_impl(args).await
    }

    #[tool(
        name = "audit_document",
        description = "Performs a structural compliance audit of a PDF document, checking Catalog, XRef, and Page Tree integrity."
    )]
    /// MCP tool: audit_document
    pub async fn audit_document(&self, Parameters(args): Parameters<AuditArgs>) -> Result<String, String> {
        audit_document_impl(args).await
    }
}

impl Default for FerruginousServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry point for running the Ferruginous MCP server over stdio.
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let server = FerruginousServer::new();
    let router = Router::new(server).with_tools(FerruginousServer::tool_router());
    
    let transport = rmcp::transport::stdio();
    
    println!("Ferruginous MCP Server starting on stdio...");
    router.serve(transport).await.map_err(|e| format!("Server error: {e}"))?;
    
    Ok(())
}
