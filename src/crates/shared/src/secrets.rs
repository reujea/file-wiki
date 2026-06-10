//! file-pipeline 시크릿 헬퍼.
//!
//! `module_secrets_api::SecretStorage` 인터페이스를 file-pipeline 도메인용으로 감싼다.
//! - 백엔드: `module_secrets::KeyringSecretStore` (OS 키링)
//! - `service_name = "file-pipeline"` 고정
//! - `migrate_credential` / `get_credential_key` 도메인 헬퍼 추가
//!
//! 내부적으로 `Box<dyn SecretStorage>`를 보유해 향후 mock/대체 백엔드로 교체 가능.

use module_secrets::KeyringSecretStore;
use module_secrets_api::{SecretError, SecretStorage};
use std::sync::OnceLock;

const SERVICE_NAME: &str = "file-pipeline";

fn store_handle() -> &'static dyn SecretStorage {
    static STORE: OnceLock<KeyringSecretStore> = OnceLock::new();
    STORE.get_or_init(|| KeyringSecretStore::new(SERVICE_NAME))
}

/// 시크릿 저장
pub fn store(key: &str, value: &str) -> Result<(), String> {
    store_handle().store(key, value).map_err(|e| e.to_string())
}

/// 시크릿 조회
pub fn get(key: &str) -> Result<Option<String>, String> {
    store_handle().get(key).map_err(|e| e.to_string())
}

/// 시크릿 삭제
pub fn delete(key: &str) -> Result<(), String> {
    store_handle().delete(key).map_err(|e| e.to_string())
}

/// 시크릿 저장소 사용 가능 여부
pub fn is_available() -> bool {
    store_handle().is_available()
}

/// 크레덴셜의 API 키를 시크릿 저장소에 저장 (`credential_<name>` 키로).
pub fn migrate_credential(name: &str, api_key: &str) -> Result<(), String> {
    if api_key.is_empty() {
        return Ok(());
    }
    let key = format!("credential_{}", name);
    store(&key, api_key)?;
    tracing::info!("크레덴셜 '{}' API 키를 시크릿 저장소로 이관", name);
    Ok(())
}

/// 시크릿 저장소에서 크레덴셜의 API 키 조회.
pub fn get_credential_key(name: &str) -> Result<Option<String>, String> {
    let key = format!("credential_{}", name);
    get(&key)
}

/// `module_secrets_api::SecretError`를 `String`으로 변환 (호환성).
pub fn map_err(e: SecretError) -> String {
    e.to_string()
}
