use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use super::document::DocumentStore;

/// The central e8085 Language Server state and RPC dispatcher.
pub struct E8085LanguageServer {
    pub client: Client,
    pub documents: Arc<DocumentStore>,
}

impl E8085LanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocumentStore::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for E8085LanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "%".to_string(),
                        " ".to_string(),
                        ",".to_string(),
                    ]),
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    completion_item: None,
                }),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: None,
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "e8085-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "e8085 Language Server initialized.")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let text = params.text_document.text;

        self.documents.insert(uri.clone(), version, text);
        if let Some(doc) = self.documents.get(&uri) {
            let diags = super::diagnostics::compute_diagnostics(&doc);
            self.client
                .publish_diagnostics(uri, diags, Some(version))
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.update(&uri, version, change.text);
            if let Some(doc) = self.documents.get(&uri) {
                let diags = super::diagnostics::compute_diagnostics(&doc);
                self.client
                    .publish_diagnostics(uri, diags, Some(version))
                    .await;
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(uri) {
            Ok(super::hover::get_hover(&doc, pos))
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(uri) {
            Ok(super::definition::get_definition(&doc, pos))
        } else {
            Ok(None)
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = &params.text_document_position.position;

        if let Some(doc) = self.documents.get(uri) {
            Ok(super::completion::get_completions(&doc, pos))
        } else {
            Ok(None)
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let pos = &params.position;

        if let Some(doc) = self.documents.get(uri) {
            Ok(super::rename::prepare_rename(&doc, pos))
        } else {
            Ok(None)
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = &params.text_document_position.position;
        let new_name = &params.new_name;

        if let Some(doc) = self.documents.get(uri) {
            Ok(super::rename::rename(&doc, pos, new_name))
        } else {
            Ok(None)
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let range = &params.range;

        if let Some(doc) = self.documents.get(uri) {
            Ok(Some(super::hints::get_inlay_hints(&doc, range)))
        } else {
            Ok(None)
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;

        if let Some(doc) = self.documents.get(uri) {
            Ok(Some(super::code_actions::get_code_actions(&doc, &params)))
        } else {
            Ok(None)
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        if let Some(doc) = self.documents.get(uri) {
            let formatted = crate::asm::format_source(&doc.text);
            if formatted == doc.text {
                return Ok(Some(vec![]));
            }
            let full_range = Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: doc.offset_to_position(doc.text.len()),
            };
            Ok(Some(vec![TextEdit {
                range: full_range,
                new_text: formatted,
            }]))
        } else {
            Ok(None)
        }
    }
}
