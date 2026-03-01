use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};

/// Open an in-memory embedded SurrealDB.
/// This compiles wherever `kv-mem` is supported.
pub async fn open_mem(ns: &str, db: &str) -> surrealdb::Result<Surreal<Db>> {
    let s = Surreal::new::<Mem>(()).await?;
    s.use_ns(ns).use_db(db).await?;
    Ok(s)
}

/// Open a SurrealKV-backed embedded SurrealDB (persistent).
/// This requires the `kv-surrealkv` crate feature.
pub async fn open_surrealkv(path: &str, ns: &str, db: &str) -> surrealdb::Result<Surreal<Db>> {
    let s = Surreal::new::<SurrealKv>(path).await?;
    s.use_ns(ns).use_db(db).await?;
    Ok(s)
}
