//! 임베딩 벡터 파일 I/O — 벡터 DB 종속성 제거용

use std::path::Path;

use anyhow::{bail, Context, Result};

const MAGIC: &[u8; 4] = b"VEC1";

/// 임베딩 벡터를 바이너리 파일로 저장
/// 포맷: magic(4) + dim(u32 LE, 4) + data(f32 LE × dim)
pub fn save_vec(path: &Path, embedding: &[f32]) -> Result<()> {
    let dim = embedding.len() as u32;
    let mut buf = Vec::with_capacity(8 + embedding.len() * 4);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&dim.to_le_bytes());
    for &v in embedding {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, &buf).context(format!("vec 저장 실패: {:?}", path))
}

/// 바이너리 파일에서 임베딩 벡터 로드
pub fn load_vec(path: &Path) -> Result<Vec<f32>> {
    let data = std::fs::read(path).context(format!("vec 읽기 실패: {:?}", path))?;
    if data.len() < 8 {
        bail!("vec 파일이 너무 짧음: {:?}", path);
    }
    if &data[..4] != MAGIC {
        bail!("vec magic 불일치: {:?}", path);
    }
    let dim = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let expected = 8 + dim * 4;
    if data.len() != expected {
        bail!(
            "vec 크기 불일치: 기대 {} 실제 {} (dim={}): {:?}",
            expected,
            data.len(),
            dim,
            path
        );
    }
    let mut vec = Vec::with_capacity(dim);
    for i in 0..dim {
        let offset = 8 + i * 4;
        let v = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        vec.push(v);
    }
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn roundtrip() {
        let file = NamedTempFile::new().expect("temp file");
        let embedding = vec![1.0_f32, -0.5, 0.0, 3.15, f32::MIN, f32::MAX];
        save_vec(file.path(), &embedding).expect("save");
        let loaded = load_vec(file.path()).expect("load");
        assert_eq!(embedding, loaded);
    }

    #[test]
    fn empty_vec() {
        let file = NamedTempFile::new().expect("temp file");
        let embedding: Vec<f32> = vec![];
        save_vec(file.path(), &embedding).expect("save");
        let loaded = load_vec(file.path()).expect("load");
        assert!(loaded.is_empty());
    }

    #[test]
    fn bad_magic() {
        let file = NamedTempFile::new().expect("temp file");
        std::fs::write(file.path(), b"BAD1\x00\x00\x00\x00").expect("write");
        assert!(load_vec(file.path()).is_err());
    }

    #[test]
    fn truncated_file() {
        let file = NamedTempFile::new().expect("temp file");
        // magic + dim=2 but only 1 float
        let mut data = Vec::new();
        data.extend_from_slice(b"VEC1");
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        std::fs::write(file.path(), &data).expect("write");
        assert!(load_vec(file.path()).is_err());
    }
}
