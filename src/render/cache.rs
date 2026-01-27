use std::sync::Mutex;

use super::ImgBuf;

pub struct ImageBufCacheEntry {
    schema_id: String,
    asset_path: String,
    img_buf: ImgBuf,
}

pub struct ImageBufCache {
    entries: Vec<ImageBufCacheEntry>,
}

impl ImageBufCache {
    /** Returns the first matching cached entry */
    pub fn filter_for(
        &self,
        schema_id: Option<String>,
        asset_path: Option<String>,
    ) -> Option<ImgBuf> {
        for entry in &self.entries {
            if let Some(schema_id_val) = schema_id.clone() {
                if schema_id_val != entry.schema_id {
                    continue;
                }
            }

            if let Some(asset_path_val) = asset_path.clone() {
                if asset_path_val != entry.asset_path {
                    continue;
                }
            }

            return Some(entry.img_buf.clone());
        }

        return None;
    }

    pub fn add_entry(&mut self, schema_id: &str, asset_path: &str, img_buf: ImgBuf) {
        self.entries.push(ImageBufCacheEntry {
            schema_id: schema_id.to_string(),
            asset_path: asset_path.to_string(),
            img_buf,
        })
    }
}

lazy_static::lazy_static! {
    pub static ref IMG_BUF_CACHE: Mutex<ImageBufCache> = Mutex::new(ImageBufCache { entries: Vec::new() });
}
