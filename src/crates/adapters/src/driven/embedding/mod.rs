pub mod claude_embed;
pub mod local_embed;
// Phase 64 트리거 #11: onnx_embed (Rust 네이티브 ort) 폐기. fastembed_adapter가 대체.
pub mod openai_embed;
pub mod python_onnx_embed;
#[cfg(feature = "fastembed")]
pub mod fastembed_adapter;
#[cfg(feature = "fastembed")]
pub mod fastembed_sparse;
