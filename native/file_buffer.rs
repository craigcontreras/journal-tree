use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Weak};

use parking_lot::Mutex;
use ropey::Rope;

static OPEN_FILES: LazyLock<Mutex<HashMap<u32, Weak<FileBuffer>>>> = LazyLock::new(Mutex::default);

pub(crate) struct FileBuffer {
    pub(crate) id: u32,
    pub(crate) rope: Mutex<Rope>,
}

impl FileBuffer {
    pub(crate) fn get(id: u32) -> Option<Arc<Self>> {
        OPEN_FILES.lock().get(&id)?.upgrade()
    }
}

impl Drop for FileBuffer {
    fn drop(&mut self) {
        OPEN_FILES.lock().remove(&self.id);
    }
}

#[cfg(not(target_family = "wasm"))]
mod desktop {
    use super::*;
    use napi::bindgen_prelude::*;
    use napi_derive::napi;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT_ID: AtomicU32 = AtomicU32::new(1);

    #[napi]
    pub struct LargeTextDocument {
        buffer: Arc<FileBuffer>,
    }

    #[napi]
    impl LargeTextDocument {
        #[napi(getter)]
        pub fn id(&self) -> u32 {
            self.buffer.id
        }

        #[napi]
        pub fn text(&self) -> String {
            let rope = self.buffer.rope.lock().clone();
            rope.to_string()
        }
    }

    #[napi(object, object_from_js = false)]
    pub struct OpenTextFileResult {
        pub source: Option<String>,
        pub document: Option<LargeTextDocument>,
    }

    pub enum LoadedFile {
        Plain(String),
        Large(Rope),
    }

    pub struct OpenTextFileTask(pub String);

    #[napi]
    impl Task for OpenTextFileTask {
        type Output = LoadedFile;
        type JsValue = OpenTextFileResult;

        fn compute(&mut self) -> Result<Self::Output> {
            let bytes = std::fs::read(&self.0).map_err(|error| {
                Error::from_reason(format!("Could not read {}: {error}", self.0))
            })?;
            let source = String::from_utf8(bytes)
                .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned());
            let large = source.len() >= 1024 * 1024
                || source
                    .bytes()
                    .filter(|&byte| byte == b'\n')
                    .nth(4998)
                    .is_some();
            Ok(if large {
                LoadedFile::Large(Rope::from_str(&source))
            } else {
                LoadedFile::Plain(source)
            })
        }

        fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
            let (source, document) = match output {
                LoadedFile::Plain(source) => (Some(source), None),
                LoadedFile::Large(rope) => {
                    let id = NEXT_ID
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                        .map_err(|_| Error::from_reason("Too many files opened"))?;
                    let buffer = Arc::new(FileBuffer {
                        id,
                        rope: Mutex::new(rope),
                    });
                    OPEN_FILES.lock().insert(id, Arc::downgrade(&buffer));
                    (None, Some(LargeTextDocument { buffer }))
                }
            };
            Ok(OpenTextFileResult { source, document })
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub use desktop::OpenTextFileTask;
