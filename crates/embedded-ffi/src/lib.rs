use libc::{c_char, size_t};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{ffi::CStr, ptr, sync::Mutex};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};

static RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
});

#[repr(C)]
pub struct Handle {
    db: Mutex<Surreal<Db>>,
}

#[repr(C)]
pub struct SeBuf {
    pub ptr: *mut u8,
    pub len: size_t,
}

#[derive(Serialize)]
struct VersionInfo<'a> {
    surrealdb_version: &'a str,
    enabled_backends: &'a [&'a str],
    build_target: &'a str,
}

const ENABLED_BACKENDS: &[&str] = &["mem", "surrealkv"];

async fn open_mem(ns: &str, db: &str) -> surrealdb::Result<Surreal<Db>> {
    let s = Surreal::new::<Mem>(()).await?;
    s.use_ns(ns).use_db(db).await?;
    Ok(s)
}

async fn open_surrealkv(path: &str, ns: &str, db: &str) -> surrealdb::Result<Surreal<Db>> {
    let s = Surreal::new::<SurrealKv>(path).await?;
    s.use_ns(ns).use_db(db).await?;
    Ok(s)
}

fn ok_bytes(bytes: Vec<u8>) -> SeBuf {
    let len = bytes.len();
    let mut bytes = std::mem::ManuallyDrop::new(bytes);
    SeBuf {
        ptr: bytes.as_mut_ptr(),
        len: len as size_t,
    }
}

fn err_bytes(msg: String) -> SeBuf {
    // Encode errors as CBOR: { "error": "..."}
    let payload = serde_json::json!({ "error": msg });
    let bytes = serde_cbor::to_vec(&payload).unwrap_or_else(|_| Vec::new());
    ok_bytes(bytes)
}

fn ok_cbor<T: Serialize>(value: &T) -> SeBuf {
    match serde_cbor::to_vec(value) {
        Ok(bytes) => ok_bytes(bytes),
        Err(e) => err_bytes(format!("CBOR encode error: {e}")),
    }
}

unsafe fn cstr(p: *const c_char) -> Result<String, String> {
    if p.is_null() {
        return Err("null string".into());
    }
    let s = unsafe { CStr::from_ptr(p) };
    s.to_str()
        .map(|s| s.to_string())
        .map_err(|e| format!("utf8 error: {e}"))
}

#[unsafe(no_mangle)]
pub extern "C" fn se_buf_free(buf: SeBuf) {
    if buf.ptr.is_null() || buf.len == 0 {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(
            buf.ptr,
            buf.len as usize,
            buf.len as usize,
        ));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn se_open_mem(ns: *const c_char, db: *const c_char) -> *mut Handle {
    let ns = match unsafe { cstr(ns) } {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };
    let dbname = match unsafe { cstr(db) } {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };

    let res = RT.block_on(async move {
        let db = open_mem(&ns, &dbname).await?;
        Ok::<_, surrealdb::Error>(Handle { db: Mutex::new(db) })
    });

    match res {
        Ok(h) => Box::into_raw(Box::new(h)),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn se_open_surrealkv(
    path: *const c_char,
    ns: *const c_char,
    db: *const c_char,
) -> *mut Handle {
    let path = match unsafe { cstr(path) } {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };
    let ns = match unsafe { cstr(ns) } {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };
    let dbname = match unsafe { cstr(db) } {
        Ok(v) => v,
        Err(_) => return ptr::null_mut(),
    };

    let res = RT.block_on(async move {
        let db = open_surrealkv(&path, &ns, &dbname).await?;
        Ok::<_, surrealdb::Error>(Handle { db: Mutex::new(db) })
    });

    match res {
        Ok(h) => Box::into_raw(Box::new(h)),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn se_close(handle: *mut Handle) {
    if handle.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(handle)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn se_version() -> SeBuf {
    let version = VersionInfo {
        surrealdb_version: env!("SURREALDB_CRATE_VERSION"),
        enabled_backends: ENABLED_BACKENDS,
        build_target: env!("BUILD_TARGET"),
    };
    ok_cbor(&version)
}

fn decode_vars(
    vars_cbor: *const u8,
    vars_len: size_t,
) -> Result<Option<serde_json::Value>, String> {
    if vars_cbor.is_null() || vars_len == 0 {
        return Ok(None);
    }

    let slice = unsafe { std::slice::from_raw_parts(vars_cbor, vars_len as usize) };
    serde_cbor::from_slice::<serde_json::Value>(slice)
        .map(Some)
        .map_err(|e| format!("vars CBOR decode error: {e}"))
}

fn query_take_impl(
    handle: *mut Handle,
    surql: String,
    vars: Option<serde_json::Value>,
    index: usize,
) -> SeBuf {
    RT.block_on(async {
        let h = unsafe { &*handle };
        let db = match h.db.lock() {
            Ok(g) => g,
            Err(_) => return err_bytes("mutex poisoned".into()),
        };

        let mut q = db.query(surql);
        if let Some(v) = vars {
            q = q.bind(v);
        }

        match q.await {
            Ok(mut resp) => {
                let val: surrealdb::types::Value = match resp.take(index) {
                    Ok(v) => v,
                    Err(e) => return err_bytes(format!("resp.take({index}) error: {e}")),
                };

                ok_cbor(&val)
            }
            Err(e) => err_bytes(e.to_string()),
        }
    })
}

/// Execute SurrealQL.
/// - vars_cbor: CBOR-encoded map/object (or empty)
/// Returns CBOR-encoded Surreal response, or CBOR { "error": "..." }.
#[unsafe(no_mangle)]
pub extern "C" fn se_query(
    handle: *mut Handle,
    surql: *const c_char,
    vars_cbor: *const u8,
    vars_len: size_t,
) -> SeBuf {
    if handle.is_null() {
        return err_bytes("null handle".into());
    }

    let surql = match unsafe { cstr(surql) } {
        Ok(v) => v,
        Err(e) => return err_bytes(e),
    };

    let vars = match decode_vars(vars_cbor, vars_len) {
        Ok(v) => v,
        Err(e) => return err_bytes(e),
    };

    query_take_impl(handle, surql, vars, 0)
}

/// Execute SurrealQL and return the result at the requested statement index.
/// - vars_cbor: CBOR-encoded map/object (or empty)
/// Returns CBOR-encoded Surreal response for the given index, or CBOR { "error": "..." }.
#[unsafe(no_mangle)]
pub extern "C" fn se_query_take(
    handle: *mut Handle,
    surql: *const c_char,
    vars_cbor: *const u8,
    vars_len: size_t,
    index: size_t,
) -> SeBuf {
    if handle.is_null() {
        return err_bytes("null handle".into());
    }

    let surql = match unsafe { cstr(surql) } {
        Ok(v) => v,
        Err(e) => return err_bytes(e),
    };

    let vars = match decode_vars(vars_cbor, vars_len) {
        Ok(v) => v,
        Err(e) => return err_bytes(e),
    };

    query_take_impl(handle, surql, vars, index as usize)
}
