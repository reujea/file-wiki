//! [DEPRECATED] `crate::secrets` 호환 shim.
//!
//! 기존 `credential_store::{store, get, delete, is_available, migrate_credential, get_credential_key}` 호출을
//! 내부적으로 `module_secrets`로 위임한다. 신규 코드는 `crate::secrets`를 직접 사용할 것.

pub use crate::secrets::{
    delete, get, get_credential_key, is_available, migrate_credential, store,
};
