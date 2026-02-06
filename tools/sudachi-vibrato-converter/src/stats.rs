use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversionStats {
    pub written: usize,
    pub skipped_negative_conn_ids: usize,
    pub normalized_pos_rows: usize,
    pub fallback_ctype_rows: usize,
    pub fallback_cform_rows: usize,
}

impl ConversionStats {
    pub fn write_env_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path)?;
        writeln!(file, "written={}", self.written)?;
        writeln!(
            file,
            "skipped_negative_conn_ids={}",
            self.skipped_negative_conn_ids
        )?;
        writeln!(file, "normalized_pos_rows={}", self.normalized_pos_rows)?;
        writeln!(file, "fallback_ctype_rows={}", self.fallback_ctype_rows)?;
        writeln!(file, "fallback_cform_rows={}", self.fallback_cform_rows)?;
        Ok(())
    }
}
